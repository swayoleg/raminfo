# Ratatui TUI plan (branch `feature/ratatui-tui`)

Goal: replace the single-shot printed report with an interactive, full-screen
ratatui application in the style of `netscanner` (bordered panels with titled
borders, a `(1)Overview (2)Hardware (3)Processes (4)Temps` tab bar, colored
tables, gauges, sparkline history) — **without** breaking the machine-readable
outputs that monitoring tools rely on.

Assumption: the request's "--timeouts" means the existing `--interval` /
`--monitor --json` ndjson stream. Those stay byte-for-byte compatible.

## Mode matrix (mandatory)

| Invocation | stdout is a tty | stdout is NOT a tty |
|---|---|---|
| `raminfo` (no flags) | **ratatui app** (new default) | short plain summary (unchanged) |
| `raminfo --monitor` | **ratatui app** (same; `--interval` = tick rate) | plain repeating `render_monitor` loop (unchanged) |
| `raminfo --json` | full JSON snapshot, one line (unchanged) | same |
| `raminfo --monitor --json [--interval N]` | ndjson stream via `to_json_monitor` (unchanged) | same |
| `raminfo --short` | legacy colored `render_short` (unchanged) | same |
| `raminfo --full` | legacy colored `render_snapshot` (unchanged) | same |
| `raminfo --tui` | ratatui app explicitly (errors if not a tty) | error message, exit 1 |

- Root no longer selects a mode. It only affects whether the Hardware tab has
  DIMM/mobo data; when `dimms` is empty show the existing "run with sudo" hint
  inside the Hardware panel.
- `--short`/`--full` remain mutually exclusive; `--tui` conflicts with both and
  with `--json`.
- Piping (`raminfo | cat`) must never enter the alternate screen or hang.

## TUI design

Layout (full screen, min 80x24; degrade gracefully below):

```
┌ raminfo v0.3.0 ──────────────────────────────────────────┤ host / uptime ┤─┐
│ (1)Overview (2)Hardware (3)Processes (4)Temps        q:quit  ↑↓:scroll  │
├──────────────────────────────────────────────────────────────────────────┤
│                       tab body (see below)                               │
├──────────────────────────────────────────────────────────────────────────┤
│ refresh 2s  •  last tick 14:09:50  •  ? help                             │
└──────────────────────────────────────────────────────────────────────────┘
```

Tabs:
1. **Overview** — left: `free -m`-style table (total/used/free/shared/buff-cache/
   available, swap row); RAM and swap `Gauge`s (green <60%, yellow <85%, red);
   right: `Sparkline` of used % over the last N ticks (ring buffer, N = width)
   plus a small summary block (top process, temp max if any).
2. **Hardware** — DIMM `Table` (same columns as `render_dimm_table`), memory
   array info (slots used/total, max capacity), motherboard, Raspberry Pi block
   when `pi.is_some()`. Static: collected once at startup via
   `collect_snapshot()`, never re-queried (no dmidecode per tick).
3. **Processes** — scrollable `Table` of `top_consumers` (pid, name, RSS, % of
   total, mini bar). ↑/↓/PgUp/PgDn/Home/End scroll, highlighted row.
4. **Temps** — `Table` of readings colored like `temp_colored`, and a
   `Sparkline`/`Chart` of the max temp history. On macOS/Windows/DDR4 render a
   centered "No RAM temperature sensors (DDR5 spd5118 on Linux only)" note.

Keys: `1`-`4` select tab, `Tab`/`Shift+Tab`/`←`/`→` cycle, `q`/`Esc`/`Ctrl+C`
quit, `r` force refresh, `+`/`-` change interval (min 1s), `?` toggle help
overlay, `↑`/`↓` scroll in Processes.

Colors: netscanner-ish — bordered `Block`s with right-aligned titles
(`|Title|`), green/yellow accents, dim gray borders, selected tab bold yellow.

## Code structure

```
src/tui/mod.rs     pub fn run(interval: Duration) -> io::Result<()>  (terminal setup, event loop)
src/tui/app.rs     App state: snapshot, dynamic, history ring buffers, tab, scroll, interval, paused
src/tui/ui.rs      fn draw(frame: &mut Frame, app: &App) + per-tab fns; pure, no I/O
src/tui/history.rs ring buffer helper (pure, unit-tested)
```

- Deps: `ratatui = "0.30"` — use `ratatui::crossterm` re-export (enable the
  `crossterm` feature) rather than a separate `crossterm` dependency.
  Verify exact API against the pinned version (the docs agent produces the
  cheatsheet); do not code from memory.
- Event loop: `event::poll(Duration::from_millis(150))` for keys; data tick
  every `interval` via `collect_dynamic()` (elapsed-time check, not sleep).
- Terminal: `ratatui::init()` / `ratatui::restore()` (or equivalent for the
  pinned version) plus a panic hook that restores the terminal first.
- `ctrlc` stays for the plain/ndjson monitor loops only. In raw mode the TUI
  handles Ctrl+C as a key event.
- No `#[cfg(target_os)]` inside `src/tui/` — it must build identically on
  Linux/macOS/Windows (CI matrix). On Windows `main` already enables ANSI.
- `src/tui/ui.rs` must be testable with `ratatui::backend::TestBackend`:
  add `tests/tui_tests.rs` rendering each tab from a mock `Snapshot` and
  asserting on buffer contents (tab titles, DIMM locator, process name,
  "No RAM temperature" note, no panic at tiny sizes like 20x5).
- Keep `render.rs`, `json.rs`, `format.rs` untouched; the TUI reuses
  `format::{fmt_kb, fmt_size}` and the same thresholds (0.6/0.85, 70/85°C).
- Zero warnings on `cargo build --release`; `cargo test` green.

## Deliverables

1. `docs/ratatui-cheatsheet.md` (scratchpad copy ok) — API notes for the pinned
   ratatui version: init/restore, event polling, Layout, Block/Tabs/Table/
   Gauge/Sparkline/Paragraph, Style/Color, TestBackend usage.
2. Implementation per above; `Cargo.toml` version → `0.3.0`, deps added.
3. `USAGE` text in `main.rs` and README updated (new default is the full-screen
   app; `--short`/`--full`/`--json` documented as the non-interactive outputs).
   Screenshots in README will be stale — flag, don't fake.
4. CLAUDE.md "Build & Run"/"Architecture" sections updated for `src/tui/`.
5. A ready-to-paste commit message (no `Co-Authored-By`, no attribution).

## Git rules for everyone touching this branch

- Never run `git commit`; never add a `Co-Authored-By` or any Claude/Anthropic
  trailer; never `git add` CLAUDE.md / AGENTS.md.
