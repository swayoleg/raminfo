//! Render tests for the interactive TUI.
//!
//! Every tab is drawn from a mock [`Snapshot`] onto a `TestBackend` buffer and
//! asserted on as plain text — no terminal, no data collection, so these run
//! identically on Linux, macOS and Windows.

use std::time::Duration;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier};
use raminfo::tui::app::{App, Tab};
use raminfo::tui::ui::draw;
use raminfo::types::*;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn sample_snapshot() -> Snapshot {
    Snapshot {
        mem: MemStats {
            total_kb: 32_845_696,
            free_kb: 3_012_345,
            available_kb: 13_631_488,
            buffers_kb: 512_000,
            cached_kb: 8_000_000,
            swap_total_kb: 16_777_216,
            swap_free_kb: 13_777_216,
        },
        dimms: vec![
            DimmSlot {
                locator: "ChannelA-DIMM0".to_string(),
                size_mb: 16384,
                speed_mhz: 3200,
                mem_type: "DDR4".to_string(),
                manufacturer: "Samsung".to_string(),
                part_number: "M471A2G43".to_string(),
                configured_speed: 3200,
                voltage: "1.2 V".to_string(),
            },
            DimmSlot {
                locator: "ChannelB-DIMM0".to_string(),
                size_mb: 16384,
                speed_mhz: 3200,
                mem_type: "DDR4".to_string(),
                manufacturer: "Samsung".to_string(),
                part_number: "M471A2G43".to_string(),
                configured_speed: 3200,
                voltage: "1.2 V".to_string(),
            },
        ],
        array: MemArrayInfo { total_slots: 4, max_capacity_mb: 65536 },
        temps: vec![
            TempReading { label: "spd5118 DIMM 0".to_string(), celsius: 42.5 },
            TempReading { label: "spd5118 DIMM 1".to_string(), celsius: 88.0 },
        ],
        top_consumers: vec![
            ProcessMem { pid: 1234, name: "chrome".to_string(), rss_kb: 2_500_000 },
            ProcessMem { pid: 4321, name: "rust-analyzer".to_string(), rss_kb: 900_000 },
        ],
        pi: None,
        mobo: MoboInfo {
            manufacturer: "Gigabyte".to_string(),
            product: "Z390-AORUS".to_string(),
        },
    }
}

fn app_on(tab: Tab, snapshot: Snapshot) -> App {
    let mut app = App::new(snapshot, Duration::from_secs(2));
    app.tab = tab;
    app
}

fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
    let buf = terminal.backend().buffer();
    let w = buf.area().width as usize;
    buf.content()
        .chunks(w)
        .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn render(app: &App, w: u16, h: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    terminal.draw(|f| draw(f, app)).unwrap();
    buffer_to_string(&terminal)
}

/// Render and return the buffer, so tests can assert on styles as well as text.
fn render_buffer(app: &App, w: u16, h: u16) -> ratatui::buffer::Buffer {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    terminal.draw(|f| draw(f, app)).unwrap();
    terminal.backend().buffer().clone()
}

const W: u16 = 100;
const H: u16 = 30;

// ─── Chrome present on every tab ──────────────────────────────────────────────

#[test]
fn every_tab_shows_the_tab_bar_and_footer() {
    for tab in [Tab::Overview, Tab::Hardware, Tab::Processes, Tab::Temps] {
        let text = render(&app_on(tab, sample_snapshot()), W, H);
        for title in ["(1)Overview", "(2)Hardware", "(3)Processes", "(4)Temps"] {
            assert!(text.contains(title), "missing {title} on {tab:?}:\n{text}");
        }
        assert!(text.contains("raminfo v"), "missing app title on {tab:?}:\n{text}");
        assert!(text.contains("refresh 2s"), "missing footer on {tab:?}:\n{text}");
        assert!(text.contains("? help"), "missing help hint on {tab:?}:\n{text}");
    }
}

// ─── Tab 1: Overview ──────────────────────────────────────────────────────────

#[test]
fn overview_tab_renders_memory_table_and_gauges() {
    let text = render(&app_on(Tab::Overview, sample_snapshot()), W, H);
    assert!(text.contains("Memory"), "{text}");
    assert!(text.contains("buff/cache"), "{text}");
    assert!(text.contains("available"), "{text}");
    assert!(text.contains("Mem:"), "{text}");
    assert!(text.contains("Swap:"), "{text}");
    assert!(text.contains("|RAM|"), "{text}");
    assert!(text.contains("|Swap|"), "{text}");
    assert!(text.contains("Used % history"), "{text}");
    // Summary panel picks up the biggest consumer and the hottest sensor.
    assert!(text.contains("chrome"), "{text}");
    assert!(text.contains("88.0°C"), "{text}");
}

#[test]
fn overview_without_swap_still_renders() {
    let mut snap = sample_snapshot();
    snap.mem.swap_total_kb = 0;
    snap.mem.swap_free_kb = 0;
    let text = render(&app_on(Tab::Overview, snap), W, H);
    assert!(text.contains("Mem:"), "{text}");
    assert!(!text.contains("Swap:"), "swap row must be hidden without swap:\n{text}");
    assert!(text.contains("no swap configured"), "{text}");
}

// ─── Tab 2: Hardware ──────────────────────────────────────────────────────────

#[test]
fn hardware_tab_lists_dimms_and_system_info() {
    let text = render(&app_on(Tab::Hardware, sample_snapshot()), W, H);
    assert!(text.contains("ChannelA-DIMM0"), "{text}");
    assert!(text.contains("ChannelB-DIMM0"), "{text}");
    assert!(text.contains("Part Number"), "{text}");
    assert!(text.contains("DDR4"), "{text}");
    assert!(text.contains("Samsung"), "{text}");
    assert!(text.contains("2 used / 4 total"), "{text}");
    assert!(text.contains("Gigabyte Z390-AORUS"), "{text}");
}

#[test]
fn hardware_tab_shows_sudo_hint_without_dimms() {
    let mut snap = sample_snapshot();
    snap.dimms.clear();
    snap.array = MemArrayInfo::default();
    let text = render(&app_on(Tab::Hardware, snap), W, H);
    assert!(text.contains("No DIMM hardware data"), "{text}");
    assert!(text.contains("run as sudo to see this"), "the sudo hint must be shown:\n{text}");
    assert!(text.contains("dmidecode"), "{text}");
}

#[test]
fn footer_shows_sudo_hint_on_every_tab_without_dimms() {
    let mut snap = sample_snapshot();
    snap.dimms.clear();
    snap.pi = None;
    for tab in [Tab::Overview, Tab::Hardware, Tab::Processes, Tab::Temps] {
        let text = render(&app_on(tab, snap.clone()), W, H);
        assert!(text.contains("run as sudo to see this"), "{tab:?}:\n{text}");
    }
    let text = render(&app_on(Tab::Overview, sample_snapshot()), W, H);
    assert!(!text.contains("run as sudo"), "no hint when DIMMs are present:\n{text}");
}

#[test]
fn memory_table_headers_align_with_values() {
    let text = render(&app_on(Tab::Overview, sample_snapshot()), W, H);
    let lines: Vec<&str> = text.lines().collect();
    let h = lines.iter().position(|l| l.contains("buff/cache")).expect("header row");
    let header = lines[h];
    let mem = lines[h + 1];
    assert!(mem.contains("Mem:"), "{text}");
    // Every header word must end on the same column as the number under it.
    for word in ["total", "used", "free", "buff/cache", "available"] {
        let end = header.find(word).unwrap() + word.len();
        let under = mem.as_bytes()[end - 1];
        assert!(under.is_ascii_digit(), "'{word}' not right-aligned over its value:\n{header}\n{mem}");
    }
}

#[test]
fn processes_tab_handles_hundreds_of_rows() {
    let mut snap = sample_snapshot();
    snap.top_consumers = (1..=400u32)
        .map(|i| raminfo::types::ProcessMem { pid: i, name: format!("proc{i}"), rss_kb: 1000 * (401 - i as u64) })
        .collect();
    let text = render(&app_on(Tab::Processes, snap), W, H);
    assert!(text.contains("400 processes"), "{text}");
    assert!(text.contains("proc1 "), "first (largest) row visible:\n{text}");
}

#[test]
fn hardware_tab_shows_raspberry_pi_board() {
    let mut snap = sample_snapshot();
    snap.dimms.clear();
    snap.pi = Some(PiBoardInfo {
        model: "Raspberry Pi 5 Model B".to_string(),
        mem_type: "LPDDR4X".to_string(),
        freq_mhz: Some(4267),
        voltage: Some("1.1 V".to_string()),
    });
    let text = render(&app_on(Tab::Hardware, snap), W, H);
    assert!(text.contains("Raspberry Pi 5 Model B"), "{text}");
    assert!(text.contains("LPDDR4X"), "{text}");
    assert!(text.contains("4267 MT/s"), "{text}");
    assert!(!text.contains("No DIMM hardware data"), "pi board replaces the hint:\n{text}");
}

// ─── Tab 3: Processes ─────────────────────────────────────────────────────────

#[test]
fn processes_tab_lists_top_consumers() {
    let text = render(&app_on(Tab::Processes, sample_snapshot()), W, H);
    assert!(text.contains("Top Consumers"), "{text}");
    assert!(text.contains("chrome"), "{text}");
    assert!(text.contains("rust-analyzer"), "{text}");
    assert!(text.contains("1234"), "{text}");
    assert!(text.contains("Share"), "{text}");
}

#[test]
fn processes_tab_groups_by_name_with_g() {
    let mut snap = sample_snapshot();
    snap.top_consumers = vec![
        ProcessMem { pid: 1, name: "claude".into(), rss_kb: 500_000 },
        ProcessMem { pid: 2, name: "chrome".into(), rss_kb: 900_000 },
        ProcessMem { pid: 3, name: "claude".into(), rss_kb: 600_000 },
    ];
    let mut app = app_on(Tab::Processes, snap);
    let flat = render(&app, W, H);
    assert!(flat.contains("3 processes"), "{flat}");
    assert!(flat.contains("press g to group"), "{flat}");

    app.on_key(KeyEvent::from(KeyCode::Char('g')));
    let grouped = render(&app, W, H);
    assert!(grouped.contains("GROUPED by name"), "{grouped}");
    assert!(grouped.contains("2 groups"), "{grouped}");
    assert!(grouped.contains("press g to ungroup"), "{grouped}");
    assert!(grouped.contains("×2"), "claude worker count:\n{grouped}");
    assert!(grouped.contains("1.0 GB"), "summed claude RSS:\n{grouped}");
    assert!(grouped.contains("g ungroup"), "footer hint:\n{grouped}");
    // Largest group first.
    let c = grouped.find("claude").unwrap();
    let ch = grouped.find("chrome").unwrap();
    assert!(c < ch, "{grouped}");
}

#[test]
fn processes_tab_without_data() {
    let mut snap = sample_snapshot();
    snap.top_consumers.clear();
    let text = render(&app_on(Tab::Processes, snap), W, H);
    assert!(text.contains("No process data available"), "{text}");
}

// ─── Tab 4: Temps ─────────────────────────────────────────────────────────────

#[test]
fn temps_tab_lists_readings() {
    let text = render(&app_on(Tab::Temps, sample_snapshot()), W, H);
    assert!(text.contains("RAM Temperatures"), "{text}");
    assert!(text.contains("spd5118 DIMM 0"), "{text}");
    assert!(text.contains("42.5°C"), "{text}");
    assert!(text.contains("88.0°C"), "{text}");
}

#[test]
fn temps_tab_shows_note_when_no_sensors() {
    let mut snap = sample_snapshot();
    snap.temps.clear();
    let text = render(&app_on(Tab::Temps, snap), W, H);
    assert!(text.contains("No RAM temperature"), "{text}");
    assert!(text.contains("spd5118"), "{text}");
}

// ─── Help overlay ─────────────────────────────────────────────────────────────

#[test]
fn help_overlay_lists_key_bindings() {
    let mut app = app_on(Tab::Overview, sample_snapshot());
    app.show_help = true;
    let text = render(&app, W, H);
    assert!(text.contains("|Help|"), "{text}");
    assert!(text.contains("refresh now"), "{text}");
    assert!(text.contains("quit"), "{text}");
}

// ─── Degrading gracefully ─────────────────────────────────────────────────────

#[test]
fn tiny_sizes_never_panic() {
    let snapshots = [sample_snapshot(), Snapshot::default()];
    for snap in snapshots {
        for tab in [Tab::Overview, Tab::Hardware, Tab::Processes, Tab::Temps] {
            for (w, h) in [(20u16, 5u16), (1, 1), (1, 30), (100, 1), (6, 3), (80, 24)] {
                let mut app = app_on(tab, snap.clone());
                // Also exercise the overlay at every size.
                app.show_help = w % 2 == 0;
                let _ = render(&app, w, h);
            }
        }
    }
}

#[test]
fn empty_snapshot_renders_without_data() {
    let text = render(&app_on(Tab::Overview, Snapshot::default()), W, H);
    assert!(text.contains("(1)Overview"), "{text}");
    assert!(text.contains("no data"), "{text}");
}


// ─── Tab bar highlighting ─────────────────────────────────────────────────────

/// The style of the first cell of `needle` in the buffer, searched row by row.
fn style_of(buf: &ratatui::buffer::Buffer, needle: &str) -> ratatui::style::Style {
    let area = *buf.area();
    for y in 0..area.height {
        let row: String = (0..area.width).map(|x| buf[(x, y)].symbol()).collect();
        if let Some(col) = row.find(needle) {
            // `find` returns a byte offset; the tab titles are pure ASCII, and
            // everything before them on that row is too.
            return buf[(col as u16, y)].style();
        }
    }
    panic!("{needle:?} not found in the buffer");
}

#[test]
fn active_tab_is_highlighted_and_the_others_are_not() {
    for (tab, active) in [
        (Tab::Overview, "(1)Overview"),
        (Tab::Hardware, "(2)Hardware"),
        (Tab::Processes, "(3)Processes"),
        (Tab::Temps, "(4)Temps"),
    ] {
        let buf = render_buffer(&app_on(tab, sample_snapshot()), W, H);
        let hl = style_of(&buf, active);
        assert_eq!(hl.fg, Some(Color::Yellow), "{active} should be the active tab");
        assert!(hl.add_modifier.contains(Modifier::BOLD), "{active} should be bold");

        for other in ["(1)Overview", "(2)Hardware", "(3)Processes", "(4)Temps"] {
            if other == active {
                continue;
            }
            let st = style_of(&buf, other);
            assert_ne!(st.fg, Some(Color::Yellow), "{other} must not look active on {tab:?}");
        }
    }
}

// ─── Help overlay stays inside the frame ──────────────────────────────────────

#[test]
fn help_overlay_does_not_paint_over_the_app_border() {
    // 60 columns is narrower than the overlay's preferred 62: it must shrink to
    // fit inside the outer block instead of erasing its left/right borders.
    for (w, h) in [(60u16, 15u16), (64, 20), (100, 30), (66, 16)] {
        let mut app = app_on(Tab::Overview, sample_snapshot());
        app.show_help = true;
        let buf = render_buffer(&app, w, h);
        for y in 1..h - 1 {
            let left = buf[(0, y)].symbol().to_string();
            let right = buf[(w - 1, y)].symbol().to_string();
            assert_eq!(left, "│", "left border broken at {w}x{h} row {y}");
            assert_eq!(right, "│", "right border broken at {w}x{h} row {y}");
        }
    }
}

// ─── Hostile data ─────────────────────────────────────────────────────────────

/// Saturated counters and non-finite temperatures — the kind of nonsense a
/// broken `dmidecode` / `/proc` read can produce. `buffers + cached` used to
/// overflow here and panic the whole app in a debug build.
fn hostile_snapshot() -> Snapshot {
    Snapshot {
        mem: MemStats {
            total_kb: u64::MAX,
            free_kb: u64::MAX,
            available_kb: u64::MAX,
            buffers_kb: u64::MAX,
            cached_kb: u64::MAX,
            swap_total_kb: u64::MAX,
            swap_free_kb: 0,
        },
        dimms: (0..8)
            .map(|i| DimmSlot {
                locator: format!("L{i}"),
                size_mb: u64::MAX,
                speed_mhz: u64::MAX,
                mem_type: String::new(),
                manufacturer: "Unknown".to_string(),
                part_number: "   ".to_string(),
                configured_speed: 0,
                voltage: String::new(),
            })
            .collect(),
        array: MemArrayInfo { total_slots: u32::MAX, max_capacity_mb: u64::MAX },
        temps: vec![
            TempReading { label: "nan".to_string(), celsius: f64::NAN },
            TempReading { label: "inf".to_string(), celsius: f64::INFINITY },
            TempReading { label: "cold".to_string(), celsius: -300.0 },
        ],
        top_consumers: (0..40)
            .map(|i| ProcessMem { pid: i, name: "x".repeat(200), rss_kb: u64::MAX })
            .collect(),
        pi: None,
        mobo: MoboInfo { manufacturer: "  ".to_string(), product: String::new() },
    }
}

#[test]
fn saturated_and_non_finite_values_never_panic() {
    for tab in [Tab::Overview, Tab::Hardware, Tab::Processes, Tab::Temps] {
        for (w, h) in [(20u16, 5u16), (1, 1), (80, 24), (100, 30), (200, 60)] {
            let mut app = app_on(tab, hostile_snapshot());
            app.show_help = w % 2 == 0;
            let _ = render(&app, w, h);
        }
    }
}

#[test]
fn overview_reports_saturated_memory_without_overflowing() {
    // Regression: `buffers_kb + cached_kb` panicked with "attempt to add with
    // overflow" instead of saturating.
    let text = render(&app_on(Tab::Overview, hostile_snapshot()), W, H);
    assert!(text.contains("Mem:"), "{text}");
    assert!(text.contains("buff/cache"), "{text}");
}

// ─── Key handling against a changing process list ─────────────────────────────

#[test]
fn key_storm_with_a_changing_process_list_never_panics() {
    let codes = [
        KeyCode::Down,
        KeyCode::Up,
        KeyCode::PageDown,
        KeyCode::PageUp,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Char('1'),
        KeyCode::Char('4'),
        KeyCode::Char('r'),
        KeyCode::Char('+'),
        KeyCode::Char('-'),
        KeyCode::Char('?'),
    ];
    let base = sample_snapshot();
    let mut app = App::new(base.clone(), Duration::from_secs(2));
    // Deterministic LCG so a failure is always reproducible.
    let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
    for i in 0..2000u32 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        app.on_key(KeyEvent::new(codes[(seed >> 33) as usize % codes.len()], KeyModifiers::NONE));
        if i % 7 == 0 {
            // The list grows and shrinks under the selection between ticks.
            let mut fresh = base.clone();
            fresh.top_consumers.truncate((seed >> 17) as usize % 3);
            app.apply_dynamic(fresh);
        }
        if i % 23 == 0 {
            let _ = render(&app, 1 + (seed % 120) as u16, 1 + (seed % 40) as u16);
        }
    }
    let selected = app.table_state.borrow().selected();
    let len = app.snapshot.top_consumers.len();
    match selected {
        Some(i) => assert!(i < len.max(1), "selection {i} out of range for {len} rows"),
        None => assert_eq!(len, 0, "only an empty list may have no selection"),
    }
}
