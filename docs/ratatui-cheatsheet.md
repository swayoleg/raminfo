# ratatui 0.30.2 — API cheatsheet (verified against the pinned version)

Everything here was checked against the vendored source of `ratatui 0.30.2` and
compiled/tested in a throwaway probe crate (`cargo build` + `cargo test` green)
with rustc 1.92.0. Code from memory of 0.26/0.28 will not compile — use these.

## 1. Cargo.toml / features

```toml
[dependencies]
ratatui = "0.30.2"
```

That is all. **Do not add a separate `crossterm` dependency** — ratatui 0.30
re-exports the exact version it links against as `ratatui::crossterm`
(crossterm **0.29.0** for ratatui 0.30.2). A separate `crossterm = "0.29"` entry
usually works but any drift gives "expected `crossterm::event::Event`, found
`crossterm::event::Event`" type errors.

Default features (all enabled by plain `ratatui = "0.30.2"`):

`all-widgets`, `crossterm`, `layout-cache`, `macros`, `std`, `underline-color`,
`widget-calendar`.

Optional (off by default): `crossterm_0_28`, `crossterm_0_29` (interop shims for
apps that own their own crossterm dep), `document-features`, `palette`,
`portable-atomic`, `scrolling-regions`, `serde`, `termina`, `termion`,
`termwiz`, `unstable*`.

So the crossterm backend **is** the default, `ratatui::crossterm` **does** exist,
and `ratatui::init()` / `ratatui::restore()` **are** present.

### 0.30 modular-crate split (why docs.rs looks odd)

`ratatui` 0.30 is a facade over `ratatui-core` (0.1.2: layout, style, text,
buffer, terminal, backend traits), `ratatui-widgets` (0.3.2: every widget),
`ratatui-crossterm` (0.1.2), `ratatui-macros` (0.7.2). Re-exports keep the
familiar paths working:

| You write | Actually lives in |
|---|---|
| `ratatui::{Frame, Terminal, TerminalOptions, Viewport, DefaultTerminal}` | ratatui-core |
| `ratatui::layout::*`, `ratatui::style::*`, `ratatui::text::*`, `ratatui::symbols::*`, `ratatui::buffer::*` | ratatui-core |
| `ratatui::widgets::*` | ratatui-widgets |
| `ratatui::backend::{Backend, TestBackend, CrosstermBackend, ClearType, WindowSize}` | ratatui-core / ratatui-crossterm |
| `ratatui::crossterm::*` | ratatui-crossterm (crossterm 0.29) |

`ratatui::prelude::*` still exists and pulls in the common types.

MSRV of ratatui 0.30.2 is **rust 1.88.0** (verified in its `Cargo.toml`).

## 2. Imports that actually resolve

```rust
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Flex, Layout, Margin, Position, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::symbols;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, Gauge, HighlightSpacing, LineGauge, Padding,
    Paragraph, RenderDirection, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
    Sparkline, Table, TableState, Tabs, Wrap,
};
use ratatui::{DefaultTerminal, Frame, Terminal};
use ratatui::backend::TestBackend;   // tests only
```

`ratatui::terminal` is a **private** module in 0.30 — import `Frame`, `Terminal`,
`TerminalOptions`, `Viewport` from the **crate root**, not `ratatui::terminal::*`.

## 3. Terminal init / restore + panic hook

```rust
fn main() -> std::io::Result<()> {
    let terminal = ratatui::init();   // raw mode + alternate screen + panic hook
    let result = run(terminal);
    ratatui::restore();               // disable raw mode + leave alt screen
    result
}
```

* `ratatui::init() -> DefaultTerminal` (= `Terminal<CrosstermBackend<Stdout>>`).
  It panics on failure; `ratatui::try_init() -> io::Result<DefaultTerminal>` is
  the fallible twin. **`try_init()` already calls `set_panic_hook()` internally**
  (verified in `init.rs`: `take_hook()` → new hook that calls `restore()` then
  the previous hook). **Do not hand-roll a panic hook** — you would just
  duplicate it. If you install your own hooks (e.g. `color_eyre`), install them
  *before* calling `ratatui::init()`.
* `ratatui::restore()` prints an error to stderr on failure and never panics;
  `try_restore()` returns the `io::Result`.
* `ratatui::run(|terminal| { ... })` wraps init + closure + restore in one call —
  handy but it gives you less control over the loop; the raminfo plan's explicit
  `init`/`restore` is fine.
* `init_with_options(TerminalOptions { viewport: Viewport::Inline(n) })` enables
  raw mode but **not** the alternate screen (do that yourself if you want it).
* Guard the whole thing behind an is-a-tty check before calling `init()` — the
  plan requires `raminfo | cat` to never enter the alternate screen.

## 4. Event loop (poll + KeyEvent, Ctrl+C, Windows key repeats)

```rust
use std::time::{Duration, Instant};

let tick = Duration::from_secs(2);
let mut last_tick = Instant::now();

while !app.quit {
    terminal.draw(|frame| draw(frame, &mut app))?;

    // Non-blocking: returns Ok(true) only if an event is queued.
    if event::poll(Duration::from_millis(150))? {
        match event::read()? {
            // MUST filter on Press: Windows delivers Press *and* Release,
            // so unfiltered handlers fire twice per keystroke.
            Event::Key(key) if key.kind == KeyEventKind::Press => on_key(&mut app, key),
            Event::Resize(_w, _h) => { /* next draw() re-lays out automatically */ }
            _ => {}
        }
    }

    if last_tick.elapsed() >= tick {      // elapsed check, never sleep()
        last_tick = Instant::now();
        app.on_tick();
    }
}
```

```rust
fn on_key(app: &mut App, key: KeyEvent) {
    // Ctrl+C: check modifiers, there is no KeyCode for it.
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
    {
        app.quit = true;
        return;
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.quit = true,
        KeyCode::Char('?')                => app.show_help = !app.show_help,
        KeyCode::Char(c @ '1'..='4')      => app.tab = c as usize - '1' as usize,
        KeyCode::Tab   | KeyCode::Right   => app.next_tab(),
        KeyCode::BackTab | KeyCode::Left  => app.prev_tab(),
        KeyCode::Down     => app.table_state.select_next(),
        KeyCode::Up       => app.table_state.select_previous(),
        KeyCode::Home     => app.table_state.select_first(),
        KeyCode::End      => app.table_state.select_last(),
        KeyCode::PageDown => app.table_state.scroll_down_by(10),
        KeyCode::PageUp   => app.table_state.scroll_up_by(10),
        _ => {}
    }
}
```

* `Shift+Tab` arrives as `KeyCode::BackTab` (not `Tab` + SHIFT) on most terminals.
* `+`/`-` come through as `KeyCode::Char('+')` / `KeyCode::Char('-')`; also accept
  `KeyCode::Char('=')` since many layouts need shift for `+`.
* `key.kind` is a `KeyEventKind` (`Press` / `Repeat` / `Release`). Crossterm 0.29
  also has `KeyEvent::is_press()` / `is_release()` and `Event::is_key_press()`,
  so `Event::Key(key) if key.is_press()` is an equivalent shorthand.
* Other useful variants: `Event::Mouse`, `Event::FocusGained/Lost`, `Event::Paste`.

## 5. Layout

```rust
let area = frame.area();          // Frame::size() is DEPRECATED — use area()

// Const-generic destructuring: N constraints -> [Rect; N]. Preferred.
let [header, body, footer] = Layout::vertical([
    Constraint::Length(1),
    Constraint::Min(0),
    Constraint::Length(1),
]).areas(area);

let [left, right] = Layout::horizontal([
    Constraint::Percentage(50),
    Constraint::Percentage(50),
]).spacing(1).areas(inner);
```

Builders: `Layout::vertical(c)`, `Layout::horizontal(c)`,
`Layout::new(Direction::Vertical, c)`, then `.direction()`, `.margin(u16)`,
`.horizontal_margin()`, `.vertical_margin()`, `.spacing(u16)`, `.flex(Flex)`,
`.constraints(c)`.

Splitters:

| Method | Returns | Notes |
|---|---|---|
| `.areas::<N>(area)` | `[Rect; N]` | panics if N != constraint count; N is usually inferred from the `let [a, b] = ...` pattern |
| `.try_areas::<N>(area)` | `Result<[Rect; N], _>` | fallible version |
| `.split(area)` | `Rects` (deref to `[Rect]`, index it) | when the count is dynamic |
| `.spacers::<N>(area)` | `[Rect; N]` | the gaps between segments |

`Constraint` variants: `Length(u16)`, `Min(u16)`, `Max(u16)`, `Percentage(u16)`,
`Ratio(u32, u32)`, `Fill(u16)`. Priority when space is short:
`Min` > `Max` > `Length` > `Percentage` > `Ratio` > `Fill`.

`Flex` variants: `Legacy` (default, excess to the last constraint), `Start`,
`End`, `Center`, `SpaceBetween`, `SpaceEvenly`, `SpaceAround`.
**0.30 changed `SpaceAround`** to real flexbox semantics (outer gaps are half the
inner gaps); the old behaviour is now `SpaceEvenly`.

`Rect` helpers: `Rect::new(x, y, w, h)`, `.inner(Margin::new(h, v))`,
`.intersection(other)`, `.union`, `.contains(Position::new(x, y))`, `.area()`,
plus the plain `.x/.y/.width/.height` fields.

**Degrading gracefully at tiny sizes:** ratatui clamps rather than panicking, and
`Constraint::Min(0)` can legitimately yield a zero-height `Rect`. Widgets render
nothing into a zero-size area, so no special-casing is needed — but do guard any
of your own arithmetic like `area.height - 2` with `saturating_sub`.

`layout::Alignment` was renamed to `layout::HorizontalAlignment` in 0.30; the old
name is kept as a type alias, so `Alignment::{Left, Center, Right}` still works.

## 6. Block: borders, left + right titles, padding

```rust
let block = Block::bordered()                                   // == Block::new().borders(Borders::ALL)
    .border_type(BorderType::Rounded)                           // Plain|Rounded|Double|Thick|QuadrantInside|...
    .border_style(Style::default().fg(Color::DarkGray))
    .padding(Padding::horizontal(1))
    .title_top(Line::from(Span::styled(
        " raminfo v0.3.0 ",
        Style::default().fg(Color::Green).bold(),
    )))                                                          // left-aligned (default)
    .title_top(Line::from("|host / uptime|").right_aligned())    // RIGHT-aligned
    .title_bottom(Line::from("q:quit").centered());

let inner = block.inner(body);   // compute BEFORE render_widget (block is consumed)
frame.render_widget(block, body);
```

**The netscanner `|Title|` look**: title alignment lives on the **`Line`**, not on
the block. `TitlePosition` only has `Top` and `Bottom` — there is no
`Alignment::Right` on a block title in 0.30. Use
`.title_top(Line::from("|Temps|").right_aligned())`. `Line::left_aligned()`,
`.centered()`, `.right_aligned()` are the three helpers.

* `Block::title(t)` still exists (takes `Into<Line>`) and honours the block-wide
  `.title_position(TitlePosition::Top|Bottom)`; prefer the explicit
  `title_top`/`title_bottom`.
* The old `block::Title` struct was **removed** in 0.30 — `Title::from(...)
  .alignment(...).position(...)` will not compile.
* `.title_style(s)` sets a default style for all titles; per-title styling via
  `Span`/`Line` overrides it.
* `Borders::{ALL, NONE, TOP, BOTTOM, LEFT, RIGHT}` are bitflags: `Borders::TOP |
  Borders::BOTTOM`. `.border_set(symbols::border::THICK)` for custom sets.
* `BorderType` variants: `Plain`, `Rounded`, `Double`, `Thick`,
  `LightDoubleDashed`, `HeavyDoubleDashed`, `LightTripleDashed`,
  `HeavyTripleDashed`, `LightQuadrupleDashed`, `HeavyQuadrupleDashed`,
  `QuadrantInside`, `QuadrantOutside`.
* `Padding` constructors: `new(l, r, t, b)`, `zero()`, `uniform(n)`,
  `horizontal(n)`, `vertical(n)`, `symmetric(x, y)`, `proportional(n)`,
  `left/right/top/bottom(n)`.

## 7. Tabs

```rust
let tabs = Tabs::new(vec!["(1)Overview", "(2)Hardware", "(3)Processes", "(4)Temps"])
    .select(app.tab)                                  // takes Into<Option<usize>>: usize or None::<usize>
    .style(Style::default().fg(Color::Gray))          // unselected
    .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    .divider(" ")                                     // default is "│"
    .padding(" ", " ");                               // or .padding_left/.padding_right
frame.render_widget(tabs, header_area);
```

* `Tabs::new` takes any `IntoIterator` whose items are `Into<Line<'a>>` —
  `&str`, `String`, `Line`, `Span` all work.
* `Tabs` is one line tall; give it `Constraint::Length(1)` (or `3` if you wrap it
  in a bordered `Block` via `.block(...)`).
* `.select(None::<usize>)` renders no highlight. There is no `Tabs::select_next`;
  keep the index in your own state.

## 8. Table + TableState (scrolling)

```rust
let header = Row::new(vec!["PID", "NAME", "RSS"])
    .style(Style::default().fg(Color::Cyan).bold())
    .bottom_margin(1);

let rows = vec![
    Row::new(vec![
        Cell::from("1234"),
        Cell::from("firefox"),
        Cell::from(Text::from("1.2 GiB").alignment(Alignment::Right)), // per-cell alignment
    ]),
    Row::new(vec!["9", "kworker", "2 MiB"]).style(Style::default().fg(Color::Gray)),
];

let table = Table::new(rows, [
        Constraint::Length(6),
        Constraint::Fill(1),
        Constraint::Length(10),
    ])
    .header(header)
    .column_spacing(1)
    .row_highlight_style(Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD))
    .highlight_symbol(">> ")
    .highlight_spacing(HighlightSpacing::Always)   // don't shift columns when selecting
    .block(Block::bordered().title_top("Processes"));

frame.render_stateful_widget(table, area, &mut app.table_state);  // note: stateful
```

* **`Table::new(rows, widths)` takes the widths in the constructor.** The
  one-argument `Table::new(rows)` form from old versions does not exist; use
  `Table::default().rows(r).widths(w)` if you must build incrementally.
* **`Table::highlight_style()` is `#[deprecated]`** (→ `row_highlight_style`).
  This repo builds with zero warnings, so always use `row_highlight_style`.
  Siblings: `column_highlight_style`, `cell_highlight_style`.
* `Row::new` accepts `IntoIterator<Item: Into<Cell<'a>>>` — `&str`/`String`/
  `Text`/`Line`/`Span`/`Cell`. Row modifiers: `.height(u16)`, `.top_margin`,
  `.bottom_margin`, `.style`. `Cell` has `.content()`, `.style()`,
  `.column_span(u16)`.
* Right-align a column by wrapping the content:
  `Cell::from(Text::from(s).alignment(Alignment::Right))` (or `.right_aligned()`).
* `HighlightSpacing::{Always, WhenSelected (default), Never}`.
* `Table::flex(Flex)` controls how leftover width is distributed.

`TableState` (all the scrolling you need — no manual offset math):

```rust
let mut st = TableState::new().with_offset(0).with_selected(Some(0));
st.select(Some(3));      st.select(None);
st.select_next();        st.select_previous();
st.select_first();       st.select_last();
st.scroll_down_by(10);   st.scroll_up_by(10);      // PgDn / PgUp
let sel: Option<usize> = st.selected();
let off: usize = st.offset();  *st.offset_mut() = 0;
```

`select_next`/`select_previous` clamp at the ends and do **not** wrap; the widget
adjusts `offset` during render so the selection stays visible. Column/cell twins
exist (`select_next_column`, `selected_cell`, …).

Optional scrollbar next to the table:

```rust
let mut sb_state = ScrollbarState::new(total_rows).position(st.offset());
frame.render_stateful_widget(
    Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑")).end_symbol(Some("↓")),
    area, &mut sb_state);
```

## 9. Gauge / LineGauge

```rust
let gauge = Gauge::default()
    .ratio(used.clamp(0.0, 1.0))              // f64 in 0.0..=1.0 — PANICS outside that range
    .label(format!("{:.1}%", used * 100.0))   // Into<Span>
    .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
    .use_unicode(true)                        // 1/8th-block partial cells
    .block(Block::bordered().title_top("RAM"));
frame.render_widget(gauge, area);
```

* `.ratio(f64)` **asserts `0.0 <= ratio <= 1.0`** — always `.clamp(0.0, 1.0)`
  (the repo already clamps over-full usage bars; keep doing it).
  `.percent(u16)` is the integer alternative (also asserts `<= 100`).
* `gauge_style().fg` is the filled colour, `.bg` the unfilled one; the label is
  drawn on top and inverted where it crosses the fill.
* A `Gauge` renders fine in `Constraint::Length(1)` (no block) or `Length(3)`
  (with a bordered block).
* `LineGauge` is the single-line variant: `.ratio()`, `.filled_style()`,
  `.unfilled_style()`, `.label()`. Its `gauge_style()` and `line_set()` are
  **deprecated** — use `filled_style`/`unfilled_style` and
  `filled_symbol`/`unfilled_symbol`.

## 10. Sparkline

```rust
let spark = Sparkline::default()
    .data(&app.history)                      // &Vec<u64> / &[u64] / iterator — no collect needed
    .max(100)                                // fixed scale; omit -> scales to the max sample
    .style(Style::default().fg(Color::Green))
    .absent_value_symbol("_")                // shown for None entries
    .block(Block::bordered().title_top("used %"));
frame.render_widget(spark, area);
```

* Signature: `data<T>(self, data: T) where T: IntoIterator, T::Item: Into<SparklineBar>`.
  `From` impls exist for `u64`, `&u64`, `Option<u64>`, `&Option<u64>`, so
  `.data(vec![1u64, 2, 3])`, `.data([1u64, 2])`, `.data(&vec_of_u64)`,
  `.data(ring.iter())` and `.data(vec![Some(1), None])` all compile.
  The pre-0.29 `data(&[u64])`-only signature is gone.
* Per-bar colouring: build `SparklineBar::from(v).style(Some(style))` values.
* **Direction / ring-buffer orientation** (verified in the render impl:
  `LeftToRight => spark_area.left() + i`, `RightToLeft => spark_area.right() - i - 1`,
  so `data[0]` is drawn at the *left* edge by default and at the *right* edge
  under `RightToLeft`). Pick one of these two consistent recipes:
  - **`push_back` newest + default `LeftToRight`** — history reads oldest→newest
    left to right; when the buffer is longer than `area.width` you must drop from
    the front yourself (`while len > width { remove(0) }`), otherwise the newest
    samples fall off the right edge and are never drawn.
  - **`push_front` newest + `.direction(RenderDirection::RightToLeft)`** — the
    newest sample always hugs the right edge and older ones scroll off the left
    automatically; drop from the back.

  The first is simpler; use `VecDeque` and size it to `area.width`.
* Only the first `min(area.width, data.len())` samples are drawn
  (`.take(max_index)` in the render impl) — surplus samples are silently
  dropped, they do not scroll.

## 11. Paragraph

```rust
let para = Paragraph::new(Text::from(vec![
        Line::from("No RAM temperature sensors"),
        Line::from(vec![
            Span::raw("DDR5 "),
            Span::styled("spd5118", Style::default().fg(Color::Yellow)),
            Span::raw(" on Linux only"),
        ]),
    ]))
    .wrap(Wrap { trim: true })                 // omit .wrap() to clip instead of wrapping
    .alignment(Alignment::Center)              // or .centered()/.left_aligned()/.right_aligned()
    .scroll((0, 0))                            // (vertical, horizontal) — NOTE the order
    .block(Block::bordered());
frame.render_widget(para, area);
```

* `Paragraph::new` takes `Into<Text<'a>>`: `&str`, `String`, `Line`, `Vec<Line>`,
  `Span`, `Text`.
* `Wrap { trim: bool }` is a plain struct literal, not a builder.
* `.scroll((y, x))` — vertical first.
* `.line_count(width)` / `.line_width()` help you size or clamp scrolling.
* A bare `&str` also implements `Widget` in 0.30:
  `frame.render_widget("hello", area)` compiles.

## 12. Centered popup rect (help overlay)

`Flex::Center` makes this a two-liner in 0.30 — no percentage-margin arithmetic:

```rust
use ratatui::layout::{Constraint, Flex, Layout, Rect};

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [area] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);
    area
}

// Fixed size instead of percentages: swap in Constraint::Length(w) / Length(h).

if app.show_help {
    let popup = centered_rect(60, 40, frame.area());
    frame.render_widget(Clear, popup);                       // MUST clear first
    frame.render_widget(
        Paragraph::new(help_text).block(Block::bordered().title_top("Help")),
        popup,
    );
}
```

Without `Clear` the widget underneath shows through the popup's gaps.
`Clear` is `ratatui::widgets::Clear`.

## 13. Style / Color / Modifier

```rust
Style::new()                    // == Style::default(), const
    .fg(Color::Green)
    .bg(Color::Reset)
    .add_modifier(Modifier::BOLD | Modifier::DIM)
    .remove_modifier(Modifier::ITALIC);

Style::default().patch(other);  // layer one style over another
```

* Colors: `Reset`, `Black`, `Red`, `Green`, `Yellow`, `Blue`, `Magenta`, `Cyan`,
  `Gray`, `DarkGray`, `LightRed`, `LightGreen`, `LightYellow`, `LightBlue`,
  `LightMagenta`, `LightCyan`, `White`, `Rgb(r, g, b)`, `Indexed(u8)`.
* Modifiers (bitflags): `BOLD`, `DIM`, `ITALIC`, `UNDERLINED`, `SLOW_BLINK`,
  `RAPID_BLINK`, `REVERSED`, `HIDDEN`, `CROSSED_OUT`.
* `Stylize` gives shorthands on `&str`, `String`, `Span`, `Line`, `Text`,
  `Style` and most widgets:

```rust
use ratatui::style::Stylize;
let s: Span = "warn".yellow().bold().into();
let l = Line::from("dim").fg(Color::DarkGray);
let st = Style::default().green().on_black().italic();
```

Note: in 0.30 `Style` no longer implements `Styled` — the shorthand methods are
**inherent** on `Style`, so `Style::default().green().on_black()` works *without*
importing `Stylize` (importing it there gives an `unused_imports` warning). You
still need `use ratatui::style::Stylize;` for the `&str`/`Span`/`Line` shorthands.

Text hierarchy (all from `ratatui::text`):
`Text` (many `Line`s) → `Line` (many `Span`s + alignment) → `Span` (str + style).
`Span::raw(s)` / `Span::styled(s, style)`; `Line::from(vec![span, span])`;
`Text::from(vec![line, line])`. Alignment helpers `left_aligned()`,
`centered()`, `right_aligned()` exist on both `Line` and `Text`.

## 14. TestBackend render tests

```rust
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
    let buf = terminal.backend().buffer();
    let w = buf.area().width as usize;
    buf.content()
        .chunks(w)
        .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn overview_tab_renders_expected_strings() {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut app = App::from_mock_snapshot();
    terminal.draw(|f| draw(f, &mut app)).unwrap();

    let text = buffer_to_string(&terminal);
    assert!(text.contains("(1)Overview"), "{text}");
    assert!(text.contains("firefox"), "{text}");
}

#[test]
fn tiny_area_does_not_panic() {
    let mut terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
    let mut app = App::default();
    terminal.draw(|f| draw(f, &mut app)).unwrap();   // just must not panic
}
```

* There is **no** `Buffer::to_string()` / `Display` impl — build the string from
  `buf.content()` + `Cell::symbol()` as above (that helper is the one thing worth
  copying into `tests/tui_tests.rs`). `Buffer` does implement `Debug`.
* Exact-match alternative: `terminal.backend().assert_buffer_lines(["hello   "])`
  — every expected line must be padded to the full backend width. Also available:
  `assert_buffer(&Buffer)`, `assert_cursor_position(pos)`,
  `Buffer::with_lines([...])`, `TestBackend::with_lines([...])`,
  `TestBackend::resize(w, h)`.
* Because the `contains` check runs on a `\n`-joined string, a match that spans a
  line break will fail — assert on substrings that fit in one row (and remember a
  cell may be padded/truncated at the panel edge).
* `TestBackend`'s error type is `core::convert::Infallible` in 0.30, but
  `Terminal::draw` still returns a `Result` — `.unwrap()` it.
* Style assertions: `buf.cell(Position::new(x, y)).unwrap().style()`.

## 15. Gotchas checklist

1. `Frame::size()` is deprecated → **`frame.area()`**.
2. Right-aligned block titles come from the **`Line`**
   (`.title_top(Line::from("|x|").right_aligned())`), not from the block;
   `TitlePosition` is only `Top`/`Bottom`, and the `block::Title` struct is gone.
3. `Table::highlight_style()` is `#[deprecated]` → **`row_highlight_style()`**;
   `LineGauge::gauge_style()`/`line_set()` are deprecated too. Deprecation =
   warning = fails this repo's zero-warning rule.
4. `Gauge::ratio()` **panics** outside `0.0..=1.0` (and `percent()` above 100) —
   always clamp.
5. `Table::new(rows, widths)` needs the widths up front; `Sparkline::data()` now
   takes any `IntoIterator<Item: Into<SparklineBar>>`.
6. Filter `KeyEventKind::Press` or every key fires twice on Windows.
7. `ratatui::init()` already installs a restore-on-panic hook — don't add one
   (and install any of your own hooks *before* calling it).
8. Never add a standalone `crossterm` dependency; use `ratatui::crossterm`.
9. Don't call `block.inner(area)` after `render_widget(block, area)` — the block
   is moved; compute `inner` first (or clone the block).
10. `Layout::areas()` panics if the destructured array length differs from the
    constraint count — the `let [a, b, c] = ...` pattern must match.
11. `ratatui::terminal` is private; import `Frame`/`Terminal` from the crate root.
12. `Flex::SpaceAround` changed semantics in 0.30 (old behaviour = `SpaceEvenly`).
13. `layout::Alignment` is now an alias for `HorizontalAlignment`.

## 16. Skeleton that matches the plan's `src/tui/` layout

```rust
// src/tui/mod.rs
pub fn run(interval: std::time::Duration) -> std::io::Result<()> {
    let terminal = ratatui::init();
    let result = event_loop(terminal, interval);
    ratatui::restore();
    result
}
```

```rust
// src/tui/ui.rs — pure, no I/O, testable with TestBackend
pub fn draw(frame: &mut ratatui::Frame, app: &mut crate::tui::app::App) { /* ... */ }
```

Take `&mut App` in `draw` (not `&App`): `TableState` must be passed as
`&mut` to `render_stateful_widget`, and the widget mutates its `offset` during
rendering.

---

*Verified by compiling and testing every snippet above in a scratch crate
(`cargo build` + `cargo test`, zero warnings) against ratatui 0.30.2 /
crossterm 0.29.0 / rustc 1.92.0.*
