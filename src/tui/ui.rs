//! Pure rendering for the interactive TUI.
//!
//! [`draw`] takes only a [`Frame`] and an immutable [`App`] — no I/O, no data
//! collection, no platform-specific code — so every panel can be rendered and
//! asserted on with `ratatui::backend::TestBackend` (see `tests/tui_tests.rs`).
//!
//! Colours mirror the classic renderer in [`crate::render`]: usage bars are
//! green below 60 %, yellow below 85 % and red above; temperatures are green
//! below 70 °C, yellow below 85 °C and red above.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Cell, Clear, Gauge, HighlightSpacing, Padding, Paragraph, Row, Sparkline,
    Table, Tabs, Wrap,
};

use crate::format::{fmt_kb, fmt_size};
use crate::types::{DimmSlot, MoboInfo, PiBoardInfo, ProcessMem, TempReading};

use super::app::{App, TAB_TITLES, Tab, ratio};

// ─── Palette ──────────────────────────────────────────────────────────────────

const BORDER: Color = Color::DarkGray;
const LABEL: Color = Color::Gray;
const VALUE: Color = Color::White;
const ACCENT: Color = Color::Cyan;
const TITLE: Color = Color::Green;

/// Usage colour, matching the thresholds used by [`crate::format::bar`].
fn usage_color(r: f64) -> Color {
    if r > 0.85 {
        Color::Red
    } else if r > 0.6 {
        Color::Yellow
    } else {
        Color::Green
    }
}

/// Temperature colour, matching [`crate::format::temp_colored`].
fn temp_style(c: f64) -> Style {
    if c >= 85.0 {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else if c >= 70.0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    }
}

/// A bordered panel with a netscanner-style right-aligned `|Title|`.
fn panel(title: &str) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .padding(Padding::horizontal(1))
        .title_top(
            Line::from(vec![
                Span::styled("|", Style::default().fg(BORDER)),
                Span::styled(title.to_string(), Style::default().fg(ACCENT)),
                Span::styled("|", Style::default().fg(BORDER)),
            ])
            .right_aligned(),
        )
}

/// A plain `used/total` text bar (no ANSI — `format::bar` colours its output
/// with `colored`, which would render as literal escapes inside a buffer).
fn text_bar(r: f64, width: usize) -> Span<'static> {
    let filled = ((r * width as f64).round() as usize).min(width);
    let mut s = "█".repeat(filled);
    s.push_str(&"░".repeat(width.saturating_sub(filled)));
    Span::styled(s, Style::default().fg(usage_color(r)))
}

fn label(text: &str) -> Span<'static> {
    Span::styled(text.to_string(), Style::default().fg(LABEL))
}

fn value(text: &str) -> Span<'static> {
    Span::styled(text.to_string(), Style::default().fg(VALUE))
}

/// A key/value line for the small info panels.
fn kv(k: &str, v: &str) -> Line<'static> {
    Line::from(vec![label(&format!("{k:<14}")), value(v)])
}

/// Render `text` centred both ways inside `area`, in a panel titled `title`.
fn centered_note(frame: &mut Frame, area: Rect, title: &str, text: Text<'static>) {
    let block = panel(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let lines = (text.lines.len() as u16).min(inner.height);
    let [slot] = Layout::vertical([Constraint::Length(lines)]).flex(Flex::Center).areas(inner);
    frame.render_widget(Paragraph::new(text).centered().wrap(Wrap { trim: true }), slot);
}

// ─── Entry point ──────────────────────────────────────────────────────────────

/// Draw the whole application for one frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Nothing sensible fits; draw a marker instead of doing layout arithmetic.
    if area.width < 6 || area.height < 3 {
        frame.render_widget(Paragraph::new("raminfo"), area);
        return;
    }

    let total = app.snapshot.mem.total_kb;
    let right_title = format!(
        "|{} · {}|",
        std::env::consts::OS,
        if total > 0 { fmt_kb(total) } else { "no data".to_string() }
    );

    let outer = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .title_top(Line::from(Span::styled(
            format!(" raminfo v{} ", env!("CARGO_PKG_VERSION")),
            Style::default().fg(TITLE).add_modifier(Modifier::BOLD),
        )))
        .title_top(Line::from(Span::styled(right_title, Style::default().fg(BORDER))).right_aligned());
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(inner);

    draw_header(frame, header, app);
    draw_footer(frame, footer, app);

    if body.height > 0 && body.width > 0 {
        match app.tab {
            Tab::Overview => draw_overview(frame, body, app),
            Tab::Hardware => draw_hardware(frame, body, app),
            Tab::Processes => draw_processes(frame, body, app),
            Tab::Temps => draw_temps(frame, body, app),
        }
    }

    if app.show_help {
        draw_help(frame, area);
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let (tabs_area, hint_area) = if area.width >= 60 {
        let [t, h] = Layout::horizontal([Constraint::Min(0), Constraint::Length(18)]).areas(area);
        (t, Some(h))
    } else {
        (area, None)
    };

    let tabs = Tabs::new(TAB_TITLES.to_vec())
        .select(app.tab.index())
        .style(Style::default().fg(LABEL))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .divider(Span::styled(" ", Style::default().fg(BORDER)))
        .padding("", " ");
    frame.render_widget(tabs, tabs_area);

    if let Some(hint) = hint_area {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "q:quit  ?:help",
                Style::default().fg(BORDER),
            )))
            .right_aligned(),
            hint,
        );
    }
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let dot = Span::styled("  •  ", Style::default().fg(BORDER));
    let line = Line::from(vec![
        label(&format!("refresh {}s", app.interval_secs())),
        dot.clone(),
        label(&format!("updates {}", app.updates)),
        dot.clone(),
        label("1-4/Tab tabs"),
        dot.clone(),
        label("r refresh"),
        dot.clone(),
        label("+/- rate"),
        dot,
        label("? help"),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

// ─── Tab 1: Overview ──────────────────────────────────────────────────────────

fn draw_overview(frame: &mut Frame, area: Rect, app: &App) {
    let (left, right) = if area.width >= 64 {
        let [l, r] =
            Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(area);
        (l, Some(r))
    } else {
        (area, None)
    };

    let [table_area, detail_area, ram_area, swap_area] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(0),
        Constraint::Length(3),
        Constraint::Length(3),
    ])
    .areas(left);

    draw_mem_table(frame, table_area, app);
    draw_mem_details(frame, detail_area, app);
    draw_gauge(frame, ram_area, "RAM", app.used_ratio(), app.used_kb(), app.snapshot.mem.total_kb);
    draw_gauge(
        frame,
        swap_area,
        "Swap",
        app.swap_ratio(),
        app.swap_used_kb(),
        app.snapshot.mem.swap_total_kb,
    );

    if let Some(right) = right {
        // Cap the sparkline height so a flat history does not become a wall of
        // solid blocks; the summary panel takes whatever is left.
        let [spark_area, summary_area] =
            Layout::vertical([Constraint::Length(10), Constraint::Min(0)]).areas(right);
        draw_used_sparkline(frame, spark_area, app);
        draw_summary(frame, summary_area, app);
    }
}

fn draw_mem_table(frame: &mut Frame, area: Rect, app: &App) {
    let m = &app.snapshot.mem;
    let mb = |kb: u64| (kb / 1024).to_string();
    let buff_cache = m.buffers_kb.saturating_add(m.cached_kb);
    // `free -m` semantics: used = total - free - buff/cache.
    let used = m.total_kb.saturating_sub(m.free_kb).saturating_sub(buff_cache);

    let header = Row::new(vec!["MB", "total", "used", "free", "buff/cache", "available"])
        .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));

    let right = |s: String| Cell::from(Text::from(s).alignment(Alignment::Right));

    let mut rows = vec![Row::new(vec![
        Cell::from(Span::styled("Mem:", Style::default().fg(VALUE).add_modifier(Modifier::BOLD))),
        right(mb(m.total_kb)),
        right(mb(used)),
        right(mb(m.free_kb)),
        right(mb(buff_cache)),
        right(mb(m.available_kb)),
    ])];
    if m.swap_total_kb > 0 {
        rows.push(Row::new(vec![
            Cell::from(Span::styled(
                "Swap:",
                Style::default().fg(VALUE).add_modifier(Modifier::BOLD),
            )),
            right(mb(m.swap_total_kb)),
            right(mb(app.swap_used_kb())),
            right(mb(m.swap_free_kb)),
            right(String::new()),
            right(String::new()),
        ]));
    }

    // buff/cache and available get double width so their headers are not clipped.
    let widths = [
        Constraint::Length(6),
        Constraint::Fill(2),
        Constraint::Fill(2),
        Constraint::Fill(2),
        Constraint::Fill(4),
        Constraint::Fill(4),
    ];
    frame.render_widget(
        Table::new(rows, widths).header(header).column_spacing(1).block(panel("Memory")),
        area,
    );
}

/// The kilobyte breakdown that the classic `Memory Usage` panel prints.
fn draw_mem_details(frame: &mut Frame, area: Rect, app: &App) {
    let m = &app.snapshot.mem;
    let bar_w = (area.width.saturating_sub(24)).clamp(0, 32) as usize;
    let used_r = app.used_ratio();

    let mut lines = vec![
        Line::from(vec![
            label(&format!("{:<12}", "Used")),
            Span::styled(format!("{:>9}", fmt_kb(app.used_kb())), Style::default().fg(VALUE)),
            Span::raw("  "),
            text_bar(used_r, bar_w),
        ]),
        kv2("Available", &fmt_kb(m.available_kb)),
        kv2("Free", &fmt_kb(m.free_kb)),
        kv2("Buffers", &fmt_kb(m.buffers_kb)),
        kv2("Cached", &fmt_kb(m.cached_kb)),
    ];
    if m.swap_total_kb > 0 {
        lines.push(Line::from(vec![
            label(&format!("{:<12}", "Swap used")),
            Span::styled(
                format!("{:>9}", fmt_kb(app.swap_used_kb())),
                Style::default().fg(VALUE),
            ),
            Span::raw("  "),
            text_bar(app.swap_ratio(), bar_w),
        ]));
        lines.push(kv2("Swap total", &fmt_kb(m.swap_total_kb)));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)).block(panel("Breakdown")), area);
}

/// A `label + right-aligned value` line for the breakdown panel.
fn kv2(k: &str, v: &str) -> Line<'static> {
    Line::from(vec![
        label(&format!("{k:<12}")),
        Span::styled(format!("{v:>9}"), Style::default().fg(VALUE)),
    ])
}

fn draw_gauge(frame: &mut Frame, area: Rect, title: &str, r: f64, used: u64, total: u64) {
    let text = if total > 0 {
        format!("{} / {}  {:.0}%", fmt_kb(used), fmt_kb(total), r * 100.0)
    } else {
        format!("no {} configured", title.to_lowercase())
    };
    let gauge = Gauge::default()
        .ratio(r.clamp(0.0, 1.0))
        .label(Span::styled(text, Style::default().fg(VALUE)))
        .gauge_style(Style::default().fg(usage_color(r)).bg(Color::Black))
        .use_unicode(true)
        .block(panel(title));
    frame.render_widget(gauge, area);
}

fn draw_used_sparkline(frame: &mut Frame, area: Rect, app: &App) {
    let block = panel("Used % history");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let spark = Sparkline::default()
        .data(app.used_history.tail(inner.width as usize))
        .max(100)
        .style(Style::default().fg(usage_color(app.used_ratio())));
    frame.render_widget(spark, inner);
}

fn draw_summary(frame: &mut Frame, area: Rect, app: &App) {
    let s = &app.snapshot;
    let mut lines = vec![
        kv("In use", &format!("{}  ({:.0}%)", fmt_kb(app.used_kb()), app.used_ratio() * 100.0)),
        kv("Available", &fmt_kb(s.mem.available_kb)),
    ];

    match s.top_consumers.first() {
        Some(p) => lines.push(Line::from(vec![
            label(&format!("{:<14}", "Top process")),
            value(&format!("{} ({})", p.name, fmt_kb(p.rss_kb))),
        ])),
        None => lines.push(kv("Top process", "—")),
    }

    match app.max_temp() {
        Some(c) => lines.push(Line::from(vec![
            label(&format!("{:<14}", "Max temp")),
            Span::styled(format!("{c:.1}°C"), temp_style(c)),
        ])),
        None => lines.push(kv("Max temp", "n/a")),
    }

    if let Some(pi) = &s.pi {
        lines.push(kv("Board", &pi.model));
    } else if s.dimms.is_empty() {
        lines.push(kv("Modules", "run with sudo"));
    } else {
        let installed: u64 = s.dimms.iter().fold(0u64, |a, d| a.saturating_add(d.size_mb));
        lines.push(kv(
            "Modules",
            &format!("{} × {}", s.dimms.len(), fmt_size(installed / s.dimms.len() as u64)),
        ));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)).block(panel("Summary")), area);
}

// ─── Tab 2: Hardware ──────────────────────────────────────────────────────────

fn draw_hardware(frame: &mut Frame, area: Rect, app: &App) {
    let s = &app.snapshot;
    let show_info = area.height >= 9;
    let (main, info) = if show_info {
        let [m, i] = Layout::vertical([Constraint::Min(3), Constraint::Length(6)]).areas(area);
        (m, Some(i))
    } else {
        (area, None)
    };

    match (&s.pi, s.dimms.is_empty()) {
        (Some(pi), _) => draw_pi_board(frame, main, pi),
        (None, false) => draw_dimm_table(frame, main, &s.dimms),
        (None, true) => centered_note(
            frame,
            main,
            "DIMM Slots",
            Text::from(vec![
                Line::from(Span::styled(
                    "No DIMM hardware data",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "run with sudo for slot / model details",
                    Style::default().fg(LABEL),
                )),
                Line::from(Span::styled(
                    "(needs dmidecode on Linux)",
                    Style::default().fg(BORDER),
                )),
            ]),
        ),
    }

    if let Some(info) = info {
        draw_hardware_info(frame, info, app);
    }
}

fn draw_dimm_table(frame: &mut Frame, area: Rect, dimms: &[DimmSlot]) {
    let dash = |s: &str| -> String {
        let t = s.trim();
        if t.is_empty() || t == "Unknown" { "—".to_string() } else { t.to_string() }
    };
    let num = |n: u64| -> String { if n > 0 { n.to_string() } else { "—".to_string() } };
    let right = |s: String| Cell::from(Text::from(s).alignment(Alignment::Right));

    let header = Row::new(vec![
        Cell::from("Slot"),
        Cell::from("Part Number"),
        Cell::from("Vendor"),
        right("Size".to_string()),
        Cell::from("Type"),
        right("Max MT/s".to_string()),
        right("Cfg MT/s".to_string()),
        right("Voltage".to_string()),
    ])
    .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = dimms
        .iter()
        .map(|d| {
            Row::new(vec![
                Cell::from(Span::styled(
                    d.locator.clone(),
                    Style::default().fg(VALUE).add_modifier(Modifier::BOLD),
                )),
                Cell::from(dash(&d.part_number)),
                Cell::from(dash(&d.manufacturer)),
                right(fmt_size(d.size_mb)),
                Cell::from(Span::styled(dash(&d.mem_type), Style::default().fg(ACCENT))),
                right(num(d.speed_mhz)),
                right(num(d.configured_speed)),
                right(dash(&d.voltage)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(16),
        Constraint::Fill(2),
        Constraint::Fill(1),
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Length(9),
        Constraint::Length(9),
        Constraint::Length(8),
    ];
    frame.render_widget(
        Table::new(rows, widths).header(header).column_spacing(1).block(panel("DIMM Slots")),
        area,
    );
}

fn draw_pi_board(frame: &mut Frame, area: Rect, pi: &PiBoardInfo) {
    let freq = match pi.freq_mhz {
        Some(mt) if mt > 0 => format!("{mt} MT/s"),
        _ if pi.model.contains("Pi 5") => "4267 MT/s (default)".to_string(),
        _ if pi.model.contains("Pi 4") => "3200 MT/s (default)".to_string(),
        _ => "—".to_string(),
    };
    let mut lines = vec![
        Line::from(Span::styled(
            pi.model.clone(),
            Style::default().fg(VALUE).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            label(&format!("{:<14}", "Type")),
            Span::styled(pi.mem_type.clone(), Style::default().fg(ACCENT)),
            Span::styled("  (soldered, no slots)", Style::default().fg(BORDER)),
        ]),
        kv("Frequency", &freq),
    ];
    if let Some(v) = &pi.voltage {
        lines.push(kv("Voltage", v));
    }
    frame.render_widget(Paragraph::new(Text::from(lines)).block(panel("Board Memory")), area);
}

fn draw_hardware_info(frame: &mut Frame, area: Rect, app: &App) {
    let s = &app.snapshot;
    let used_slots = s.dimms.len() as u32;
    let installed_mb: u64 = s.dimms.iter().fold(0u64, |a, d| a.saturating_add(d.size_mb));
    let free_slots = s.array.total_slots.saturating_sub(used_slots);

    let slots = if s.array.total_slots > 0 {
        format!(
            "{} used / {} total  {}{}",
            used_slots,
            s.array.total_slots,
            "●".repeat(used_slots.min(64) as usize),
            "○".repeat(free_slots.min(64) as usize)
        )
    } else {
        "—".to_string()
    };
    let max_cap = if s.array.max_capacity_mb > 0 {
        let headroom = s.array.max_capacity_mb.saturating_sub(installed_mb);
        if headroom > 0 {
            format!("{} ({} free)", fmt_size(s.array.max_capacity_mb), fmt_size(headroom))
        } else {
            format!("{} (fully populated)", fmt_size(s.array.max_capacity_mb))
        }
    } else {
        "—".to_string()
    };

    let lines = vec![kv("Slots", &slots), kv("Max capacity", &max_cap), kv("Motherboard", &mobo_text(&s.mobo))];
    frame.render_widget(Paragraph::new(Text::from(lines)).block(panel("System")), area);
}

fn mobo_text(m: &MoboInfo) -> String {
    match (m.manufacturer.trim(), m.product.trim()) {
        ("", "") => "—".to_string(),
        ("", p) => p.to_string(),
        (v, "") => v.to_string(),
        (v, p) => format!("{v} {p}"),
    }
}

// ─── Tab 3: Processes ─────────────────────────────────────────────────────────

fn draw_processes(frame: &mut Frame, area: Rect, app: &App) {
    let procs = &app.snapshot.top_consumers;
    if procs.is_empty() {
        centered_note(
            frame,
            area,
            "Top Consumers",
            Text::from(Line::from(Span::styled(
                "No process data available",
                Style::default().fg(LABEL),
            ))),
        );
        return;
    }

    let total = app.snapshot.mem.total_kb;
    let bar_w = 14usize;
    let right = |s: String| Cell::from(Text::from(s).alignment(Alignment::Right));

    let header = Row::new(vec![
        right("#".to_string()),
        right("PID".to_string()),
        Cell::from("Process"),
        right("RSS".to_string()),
        right("Share".to_string()),
        Cell::from(""),
    ])
    .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = procs
        .iter()
        .enumerate()
        .map(|(i, p): (usize, &ProcessMem)| {
            let r = ratio(p.rss_kb, total);
            Row::new(vec![
                Cell::from(Text::from(format!("{}.", i + 1)).alignment(Alignment::Right))
                    .style(Style::default().fg(BORDER)),
                right(p.pid.to_string()),
                Cell::from(Span::styled(
                    p.name.clone(),
                    Style::default().fg(VALUE).add_modifier(Modifier::BOLD),
                )),
                right(fmt_kb(p.rss_kb)),
                right(format!("{:.0}%", r * 100.0)),
                Cell::from(Line::from(text_bar(r, bar_w))),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(4),
        Constraint::Length(8),
        Constraint::Fill(1),
        Constraint::Length(10),
        Constraint::Length(6),
        Constraint::Length(bar_w as u16),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .row_highlight_style(Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD))
        .highlight_symbol("▌")
        .highlight_spacing(HighlightSpacing::Always)
        .block(panel("Top Consumers"));

    let mut state = app.table_state.borrow_mut();
    frame.render_stateful_widget(table, area, &mut state);
}

// ─── Tab 4: Temps ─────────────────────────────────────────────────────────────

fn draw_temps(frame: &mut Frame, area: Rect, app: &App) {
    let temps = &app.snapshot.temps;
    if temps.is_empty() {
        centered_note(
            frame,
            area,
            "RAM Temperatures",
            Text::from(vec![
                Line::from(Span::styled(
                    "No RAM temperature sensors",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "DDR5 spd5118 on Linux only",
                    Style::default().fg(LABEL),
                )),
            ]),
        );
        return;
    }

    let show_spark = area.height >= 10 && !app.temp_history.is_empty();
    let (table_area, spark_area) = if show_spark {
        let [t, s] = Layout::vertical([Constraint::Min(3), Constraint::Length(6)]).areas(area);
        (t, Some(s))
    } else {
        (area, None)
    };

    draw_temp_table(frame, table_area, temps);

    if let Some(spark_area) = spark_area {
        let block = panel("Max °C history");
        let inner = block.inner(spark_area);
        frame.render_widget(block, spark_area);
        if inner.width > 0 && inner.height > 0 {
            let color = app.max_temp().map(temp_style).unwrap_or_default().fg.unwrap_or(Color::Green);
            let spark = Sparkline::default()
                .data(app.temp_history.tail(inner.width as usize))
                .max(app.temp_history.max().max(100))
                .style(Style::default().fg(color));
            frame.render_widget(spark, inner);
        }
    }
}

fn draw_temp_table(frame: &mut Frame, area: Rect, temps: &[TempReading]) {
    let header = Row::new(vec![
        Cell::from("Sensor"),
        Cell::from(Text::from("Temp").alignment(Alignment::Right)),
        Cell::from("Level"),
    ])
    .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = temps
        .iter()
        .map(|t| {
            // Scale the level bar over a 30–100 °C working range.
            let r = ratio((t.celsius.max(30.0) as u64).saturating_sub(30), 70);
            Row::new(vec![
                Cell::from(Span::styled(t.label.clone(), Style::default().fg(VALUE))),
                Cell::from(
                    Text::from(Span::styled(format!("{:.1}°C", t.celsius), temp_style(t.celsius)))
                        .alignment(Alignment::Right),
                ),
                Cell::from(Line::from(Span::styled(
                    {
                        let filled = ((r * 20.0).round() as usize).min(20);
                        let mut s = "█".repeat(filled);
                        s.push_str(&"░".repeat(20 - filled));
                        s
                    },
                    temp_style(t.celsius),
                ))),
            ])
        })
        .collect();

    let widths = [Constraint::Fill(1), Constraint::Length(10), Constraint::Length(20)];
    frame.render_widget(
        Table::new(rows, widths).header(header).column_spacing(2).block(panel("RAM Temperatures")),
        area,
    );
}

// ─── Help overlay ─────────────────────────────────────────────────────────────

fn draw_help(frame: &mut Frame, area: Rect) {
    let rows: [(&str, &str); 8] = [
        ("1 / 2 / 3 / 4", "select Overview / Hardware / Processes / Temps"),
        ("Tab / ← / →", "cycle tabs (Shift+Tab goes back)"),
        ("↑ / ↓", "scroll the process list"),
        ("PgUp / PgDn", "scroll by ten rows"),
        ("Home / End", "jump to first / last process"),
        ("r", "refresh now"),
        ("+ / -", "slower / faster refresh (1-60s)"),
        ("q / Esc / Ctrl+C", "quit"),
    ];
    let mut lines: Vec<Line> = rows
        .iter()
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(
                    format!("  {k:<18}"),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                label(v),
            ])
        })
        .collect();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  press ? to close",
        Style::default().fg(BORDER),
    )));

    // Stay inside the outer frame: an overlay that paints over the app border
    // reads as a rendering glitch rather than a popup.
    let width = 62u16.min(area.width.saturating_sub(2));
    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(2));
    if width < 4 || height < 3 {
        return;
    }
    let [h] = Layout::horizontal([Constraint::Length(width)]).flex(Flex::Center).areas(area);
    let [popup] = Layout::vertical([Constraint::Length(height)]).flex(Flex::Center).areas(h);

    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).block(panel("Help")),
        popup,
    );
}
