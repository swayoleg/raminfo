//! Application state for the interactive TUI.
//!
//! Everything here is pure state manipulation — no terminal I/O and no data
//! collection — so the whole module is testable off-screen. The event loop in
//! [`super::run`] owns the I/O and feeds fresh snapshots in via
//! [`App::apply_dynamic`].

use std::cell::RefCell;
use std::time::Duration;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;

use crate::types::{ProcessMem, Snapshot};

use super::history::History;

/// Number of samples kept for the sparklines (comfortably wider than any
/// realistic terminal, so the visible window is always full).
pub const HISTORY_LEN: usize = 512;

/// Smallest allowed refresh interval, in seconds.
pub const MIN_INTERVAL_SECS: u64 = 1;
/// Largest allowed refresh interval, in seconds.
pub const MAX_INTERVAL_SECS: u64 = 60;

/// The four top-level tabs, in selection order.
pub const TAB_TITLES: [&str; 4] = ["(1)Overview", "(2)Hardware", "(3)Processes", "(4)Temps"];

/// Which tab is currently shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Hardware,
    Processes,
    Temps,
}

impl Tab {
    /// Map a zero-based tab index onto a [`Tab`], clamping out-of-range values
    /// to [`Tab::Overview`].
    pub fn from_index(i: usize) -> Tab {
        match i {
            1 => Tab::Hardware,
            2 => Tab::Processes,
            3 => Tab::Temps,
            _ => Tab::Overview,
        }
    }

    /// The zero-based index of this tab.
    pub fn index(self) -> usize {
        match self {
            Tab::Overview => 0,
            Tab::Hardware => 1,
            Tab::Processes => 2,
            Tab::Temps => 3,
        }
    }
}

/// Complete TUI state: the latest snapshot, sparkline histories, the selected
/// tab, process-table scroll position and the refresh interval.
#[derive(Debug)]
pub struct App {
    /// Latest data. Static fields (`dimms`, `array`, `mobo`, `pi`) come from
    /// the one-off startup collection and are never overwritten.
    pub snapshot: Snapshot,
    /// Currently selected tab.
    pub tab: Tab,
    /// Data refresh interval.
    pub interval: Duration,
    /// Set once the user asks to quit; the event loop exits on the next pass.
    pub quit: bool,
    /// Whether the help overlay is visible.
    pub show_help: bool,
    /// Processes tab: aggregate rows by process name (toggled with `g`).
    pub grouped: bool,
    /// Set by `r`; the event loop refreshes immediately and clears it.
    pub force_refresh: bool,
    /// RAM usage percentage (0–100) over the last [`HISTORY_LEN`] ticks.
    pub used_history: History,
    /// Maximum RAM temperature (whole °C) over the last [`HISTORY_LEN`] ticks.
    pub temp_history: History,
    /// Number of data refreshes applied since startup.
    pub updates: u64,
    /// Scroll/selection state for the Processes table.
    ///
    /// `RefCell` because `Table` is a stateful widget (rendering adjusts the
    /// scroll offset) while `ui::draw` deliberately takes `&App`.
    pub table_state: RefCell<TableState>,
}

impl Default for App {
    fn default() -> Self {
        App::new(Snapshot::default(), Duration::from_secs(2))
    }
}

impl App {
    /// Build the initial state from the startup snapshot and refresh interval.
    pub fn new(snapshot: Snapshot, interval: Duration) -> Self {
        let mut app = App {
            snapshot,
            tab: Tab::Overview,
            interval: floor_interval(interval),
            quit: false,
            show_help: false,
            grouped: false,
            force_refresh: false,
            used_history: History::new(HISTORY_LEN),
            temp_history: History::new(HISTORY_LEN),
            updates: 0,
            table_state: RefCell::new(TableState::new().with_selected(Some(0))),
        };
        app.record_history();
        app.clamp_selection();
        app
    }

    /// Merge a freshly collected dynamic snapshot (`mem`, `temps`,
    /// `top_consumers`) into the state, keeping the static hardware data.
    /// True when DIMM / motherboard details are missing and this is not a
    /// Raspberry Pi — i.e. the hardware tab would be populated under sudo.
    pub fn needs_sudo(&self) -> bool {
        self.snapshot.pi.is_none() && self.snapshot.dimms.is_empty()
    }

    pub fn apply_dynamic(&mut self, dynamic: Snapshot) {
        self.snapshot.mem = dynamic.mem;
        self.snapshot.temps = dynamic.temps;
        self.snapshot.top_consumers = dynamic.top_consumers;
        self.updates = self.updates.saturating_add(1);
        self.record_history();
        self.clamp_selection();
    }

    /// Push the current usage / temperature samples onto the sparkline rings.
    fn record_history(&mut self) {
        // Stored in tenths of a percent so small drifts are visible when the
        // sparkline auto-scales to the observed range.
        self.used_history.push((self.used_ratio() * 1000.0).round() as u64);
        if let Some(max) = self.max_temp() {
            self.temp_history.push(max.round().max(0.0) as u64);
        }
    }

    /// Keep the process-table selection inside the current row count.
    /// Number of rows the Processes tab currently shows (grouped or flat).
    pub fn row_count(&self) -> usize {
        if self.grouped {
            group_by_name(&self.snapshot.top_consumers).len()
        } else {
            self.snapshot.top_consumers.len()
        }
    }

    /// Toggle grouping by process name and reset the selection to the top.
    pub fn toggle_grouped(&mut self) {
        self.grouped = !self.grouped;
        self.table_state.get_mut().select(Some(0));
        *self.table_state.get_mut().offset_mut() = 0;
        self.clamp_selection();
    }

    fn clamp_selection(&mut self) {
        let len = self.row_count();
        let state = self.table_state.get_mut();
        match state.selected() {
            _ if len == 0 => state.select(None),
            None => state.select(Some(0)),
            Some(i) if i >= len => state.select(Some(len - 1)),
            Some(_) => {}
        }
    }

    // ─── Derived values ───────────────────────────────────────────────────

    /// RAM in use (total − available), in kilobytes.
    pub fn used_kb(&self) -> u64 {
        self.snapshot.mem.total_kb.saturating_sub(self.snapshot.mem.available_kb)
    }

    /// Swap in use (total − free), in kilobytes.
    pub fn swap_used_kb(&self) -> u64 {
        self.snapshot.mem.swap_total_kb.saturating_sub(self.snapshot.mem.swap_free_kb)
    }

    /// RAM usage as a ratio in `0.0..=1.0` (never NaN, even with no data).
    pub fn used_ratio(&self) -> f64 {
        ratio(self.used_kb(), self.snapshot.mem.total_kb)
    }

    /// Swap usage as a ratio in `0.0..=1.0` (never NaN, even with no swap).
    pub fn swap_ratio(&self) -> f64 {
        ratio(self.swap_used_kb(), self.snapshot.mem.swap_total_kb)
    }

    /// The hottest current temperature reading, if any sensors were found.
    pub fn max_temp(&self) -> Option<f64> {
        self.snapshot
            .temps
            .iter()
            .map(|t| t.celsius)
            .filter(|c| c.is_finite())
            .fold(None, |acc: Option<f64>, c| Some(acc.map_or(c, |m: f64| m.max(c))))
    }

    /// Refresh interval in whole seconds (as shown in the footer).
    pub fn interval_secs(&self) -> u64 {
        self.interval.as_secs().max(MIN_INTERVAL_SECS)
    }

    // ─── State transitions ────────────────────────────────────────────────

    /// Select a tab by zero-based index (out-of-range values are ignored).
    pub fn select_tab(&mut self, index: usize) {
        if index < TAB_TITLES.len() {
            self.tab = Tab::from_index(index);
        }
    }

    /// Move to the next tab, wrapping around.
    pub fn next_tab(&mut self) {
        self.tab = Tab::from_index((self.tab.index() + 1) % TAB_TITLES.len());
    }

    /// Move to the previous tab, wrapping around.
    pub fn prev_tab(&mut self) {
        self.tab = Tab::from_index((self.tab.index() + TAB_TITLES.len() - 1) % TAB_TITLES.len());
    }

    /// Increase the refresh interval by one second, up to [`MAX_INTERVAL_SECS`].
    ///
    /// A no-op once the interval is at (or, via an explicit `--interval`,
    /// beyond) the maximum, so `+` can never *shorten* the interval.
    pub fn slower(&mut self) {
        let secs = self.interval_secs();
        if secs < MAX_INTERVAL_SECS {
            self.interval = Duration::from_secs(secs + 1);
        }
    }

    /// Decrease the refresh interval by one second, floored at
    /// [`MIN_INTERVAL_SECS`].
    ///
    /// An interval set above [`MAX_INTERVAL_SECS`] on the command line is
    /// pulled back into the adjustable range on the first press.
    pub fn faster(&mut self) {
        self.interval =
            clamp_interval(Duration::from_secs(self.interval_secs().saturating_sub(1)));
    }

    /// Handle one key press. Pure state mutation — the event loop only decides
    /// *when* to call this.
    pub fn on_key(&mut self, key: KeyEvent) {
        // Ctrl+C has no dedicated KeyCode; it arrives as a modified 'c'.
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
        {
            self.quit = true;
            return;
        }

        match key.code {
            // Esc closes the help overlay first; only a second Esc quits.
            KeyCode::Esc if self.show_help => self.show_help = false,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('g') | KeyCode::Char('G') => self.toggle_grouped(),
            KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::F(1) => {
                self.show_help = !self.show_help
            }
            KeyCode::Char('r') | KeyCode::Char('R') => self.force_refresh = true,
            KeyCode::Char('+') | KeyCode::Char('=') => self.slower(),
            KeyCode::Char('-') | KeyCode::Char('_') => self.faster(),
            KeyCode::Char(c @ '1'..='4') => self.select_tab(c as usize - '1' as usize),
            KeyCode::Tab | KeyCode::Right => self.next_tab(),
            KeyCode::BackTab | KeyCode::Left => self.prev_tab(),
            KeyCode::Down => self.scroll_down(1),
            KeyCode::Up => self.scroll_up(1),
            KeyCode::PageDown => self.scroll_down(10),
            KeyCode::PageUp => self.scroll_up(10),
            KeyCode::Home => self.scroll_home(),
            KeyCode::End => self.scroll_end(),
            _ => {}
        }
    }

    fn scroll_down(&mut self, by: u16) {
        if self.row_count() == 0 {
            return;
        }
        let state = self.table_state.get_mut();
        if by == 1 { state.select_next() } else { state.scroll_down_by(by) }
        self.clamp_selection();
    }

    fn scroll_up(&mut self, by: u16) {
        if self.row_count() == 0 {
            return;
        }
        let state = self.table_state.get_mut();
        if by == 1 { state.select_previous() } else { state.scroll_up_by(by) }
    }

    fn scroll_home(&mut self) {
        if self.row_count() == 0 {
            return;
        }
        self.table_state.get_mut().select_first();
    }

    fn scroll_end(&mut self) {
        let len = self.row_count();
        if len == 0 {
            return;
        }
        self.table_state.get_mut().select(Some(len - 1));
    }
}

/// One row of the grouped Processes view: every process sharing a name,
/// summed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessGroup {
    pub name: String,
    pub count: usize,
    pub rss_kb: u64,
}

/// Aggregate processes by name (e.g. every `claude` or `chrome` worker into one
/// row), sorted by total RSS descending, then by name for a stable order.
pub fn group_by_name(procs: &[ProcessMem]) -> Vec<ProcessGroup> {
    let mut groups: Vec<ProcessGroup> = Vec::new();
    for p in procs {
        match groups.iter_mut().find(|g| g.name == p.name) {
            Some(g) => {
                g.count += 1;
                g.rss_kb = g.rss_kb.saturating_add(p.rss_kb);
            }
            None => groups.push(ProcessGroup { name: p.name.clone(), count: 1, rss_kb: p.rss_kb }),
        }
    }
    groups.sort_by(|x, y| y.rss_kb.cmp(&x.rss_kb).then_with(|| x.name.cmp(&y.name)));
    groups
}

/// `used / total` as a finite ratio in `0.0..=1.0`.
///
/// Guards the `0 / 0` case: `Gauge::ratio` panics on NaN as well as on
/// out-of-range values.
pub fn ratio(used: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    let r = used as f64 / total as f64;
    if r.is_finite() { r.clamp(0.0, 1.0) } else { 0.0 }
}

/// Clamp a requested interval into `MIN_INTERVAL_SECS..=MAX_INTERVAL_SECS`.
fn clamp_interval(d: Duration) -> Duration {
    Duration::from_secs(d.as_secs().clamp(MIN_INTERVAL_SECS, MAX_INTERVAL_SECS))
}

/// Enforce only the *minimum* interval, so an explicit `--interval 300` is
/// honoured instead of being silently shortened to [`MAX_INTERVAL_SECS`].
fn floor_interval(d: Duration) -> Duration {
    Duration::from_secs(d.as_secs().max(MIN_INTERVAL_SECS))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn pm(pid: u32, name: &str, rss_kb: u64) -> ProcessMem {
        ProcessMem { pid, name: name.to_string(), rss_kb }
    }

    #[test]
    fn group_by_name_sums_and_sorts() {
        let g = group_by_name(&[pm(1, "claude", 500), pm(2, "chrome", 900), pm(3, "claude", 600)]);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0], ProcessGroup { name: "claude".into(), count: 2, rss_kb: 1100 });
        assert_eq!(g[1], ProcessGroup { name: "chrome".into(), count: 1, rss_kb: 900 });
        assert!(group_by_name(&[]).is_empty());
    }

    #[test]
    fn g_toggles_grouping_and_resets_selection() {
        let mut app = App::new(Snapshot {
            top_consumers: vec![pm(1, "a", 1), pm(2, "a", 1), pm(3, "b", 1)],
            ..Default::default()
        }, Duration::from_secs(2));
        app.on_key(KeyEvent::from(KeyCode::End));
        assert_eq!(app.table_state.borrow().selected(), Some(2));
        app.on_key(KeyEvent::from(KeyCode::Char('g')));
        assert!(app.grouped);
        assert_eq!(app.row_count(), 2);
        assert_eq!(app.table_state.borrow().selected(), Some(0));
        app.on_key(KeyEvent::from(KeyCode::End));
        assert_eq!(app.table_state.borrow().selected(), Some(1));
        app.on_key(KeyEvent::from(KeyCode::Char('g')));
        assert!(!app.grouped);
    }

    #[test]
    fn esc_closes_help_before_quitting() {
        let mut app = App::new(Snapshot::default(), Duration::from_secs(2));
        app.on_key(KeyEvent::from(KeyCode::Char('?')));
        assert!(app.show_help);
        app.on_key(KeyEvent::from(KeyCode::Esc));
        assert!(!app.show_help);
        assert!(!app.quit, "first Esc only closes the overlay");
        app.on_key(KeyEvent::from(KeyCode::Esc));
        assert!(app.quit);
    }
    use crate::types::{MemStats, ProcessMem, TempReading};
    use ratatui::crossterm::event::KeyEventKind;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn snapshot_with_procs(n: usize) -> Snapshot {
        Snapshot {
            mem: MemStats {
                total_kb: 1000,
                available_kb: 400,
                swap_total_kb: 100,
                swap_free_kb: 40,
                ..Default::default()
            },
            top_consumers: (0..n)
                .map(|i| ProcessMem { pid: i as u32, name: format!("p{i}"), rss_kb: 10 })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn ratio_never_nan() {
        assert_eq!(ratio(0, 0), 0.0);
        assert_eq!(ratio(5, 0), 0.0);
        assert_eq!(ratio(5, 10), 0.5);
        // Over-full input clamps instead of exceeding 1.0.
        assert_eq!(ratio(20, 10), 1.0);
    }

    #[test]
    fn default_app_has_safe_ratios() {
        let app = App::default();
        assert_eq!(app.used_ratio(), 0.0);
        assert_eq!(app.swap_ratio(), 0.0);
        assert_eq!(app.max_temp(), None);
    }

    #[test]
    fn derived_usage_values() {
        let app = App::new(snapshot_with_procs(3), Duration::from_secs(2));
        assert_eq!(app.used_kb(), 600);
        assert_eq!(app.swap_used_kb(), 60);
        assert!((app.used_ratio() - 0.6).abs() < 1e-9);
        assert!((app.swap_ratio() - 0.6).abs() < 1e-9);
    }

    #[test]
    fn tab_keys_select_and_cycle() {
        let mut app = App::default();
        app.on_key(key(KeyCode::Char('3')));
        assert_eq!(app.tab, Tab::Processes);
        app.on_key(key(KeyCode::Tab));
        assert_eq!(app.tab, Tab::Temps);
        app.on_key(key(KeyCode::Right));
        assert_eq!(app.tab, Tab::Overview); // wraps
        app.on_key(key(KeyCode::BackTab));
        assert_eq!(app.tab, Tab::Temps); // wraps back
        app.on_key(key(KeyCode::Left));
        assert_eq!(app.tab, Tab::Processes);
        app.on_key(key(KeyCode::Char('1')));
        assert_eq!(app.tab, Tab::Overview);
    }

    #[test]
    fn quit_keys() {
        for code in [KeyCode::Char('q'), KeyCode::Esc] {
            let mut app = App::default();
            app.on_key(key(code));
            assert!(app.quit, "{code:?} should quit");
        }
        let mut app = App::default();
        app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.quit, "Ctrl+C should quit");
        // A plain 'c' must not quit.
        let mut app = App::default();
        app.on_key(key(KeyCode::Char('c')));
        assert!(!app.quit);
    }

    #[test]
    fn help_toggle_and_refresh() {
        let mut app = App::default();
        app.on_key(key(KeyCode::Char('?')));
        assert!(app.show_help);
        app.on_key(key(KeyCode::Char('?')));
        assert!(!app.show_help);

        assert!(!app.force_refresh);
        app.on_key(key(KeyCode::Char('r')));
        assert!(app.force_refresh);
    }

    #[test]
    fn interval_keys_clamp() {
        let mut app = App::new(Snapshot::default(), Duration::from_secs(1));
        app.on_key(key(KeyCode::Char('-')));
        assert_eq!(app.interval_secs(), MIN_INTERVAL_SECS);
        app.on_key(key(KeyCode::Char('+')));
        assert_eq!(app.interval_secs(), 2);
        app.on_key(key(KeyCode::Char('=')));
        assert_eq!(app.interval_secs(), 3);
        for _ in 0..100 {
            app.slower();
        }
        assert_eq!(app.interval_secs(), MAX_INTERVAL_SECS);
    }

    #[test]
    fn explicit_interval_above_the_maximum_is_honoured() {
        // `--interval 300` must not be silently shortened to MAX_INTERVAL_SECS:
        // the plain monitor loop honours it, so the app has to as well.
        let mut app = App::new(Snapshot::default(), Duration::from_secs(300));
        assert_eq!(app.interval_secs(), 300);
        // `+` can never make the refresh *faster*.
        app.on_key(key(KeyCode::Char('+')));
        assert_eq!(app.interval_secs(), 300);
        // `-` pulls an out-of-range value back into the adjustable window.
        app.on_key(key(KeyCode::Char('-')));
        assert_eq!(app.interval_secs(), MAX_INTERVAL_SECS);
        app.on_key(key(KeyCode::Char('-')));
        assert_eq!(app.interval_secs(), MAX_INTERVAL_SECS - 1);
    }

    #[test]
    fn sub_second_interval_is_floored() {
        let app = App::new(Snapshot::default(), Duration::from_millis(1));
        assert_eq!(app.interval_secs(), MIN_INTERVAL_SECS);
    }

    #[test]
    fn startup_without_processes_has_no_selection() {
        let app = App::new(Snapshot::default(), Duration::from_secs(2));
        assert_eq!(app.table_state.borrow().selected(), None);
        let app = App::new(snapshot_with_procs(3), Duration::from_secs(2));
        assert_eq!(app.table_state.borrow().selected(), Some(0));
    }

    #[test]
    fn scrolling_stays_in_range() {
        let mut app = App::new(snapshot_with_procs(4), Duration::from_secs(2));
        assert_eq!(app.table_state.borrow().selected(), Some(0));
        app.on_key(key(KeyCode::End));
        assert_eq!(app.table_state.borrow().selected(), Some(3));
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.table_state.borrow().selected(), Some(3), "must not run past the end");
        app.on_key(key(KeyCode::Home));
        assert_eq!(app.table_state.borrow().selected(), Some(0));
        app.on_key(key(KeyCode::Up));
        assert_eq!(app.table_state.borrow().selected(), Some(0));
        app.on_key(key(KeyCode::PageDown));
        assert!(app.table_state.borrow().selected().unwrap() <= 3);
    }

    #[test]
    fn scrolling_with_no_processes_is_safe() {
        let mut app = App::default();
        for code in [KeyCode::Down, KeyCode::Up, KeyCode::PageDown, KeyCode::PageUp,
                     KeyCode::Home, KeyCode::End] {
            app.on_key(key(code));
        }
        // An empty list has no selection at all — matching what `apply_dynamic`
        // leaves behind when the process list drains to nothing.
        assert_eq!(app.table_state.borrow().selected(), None);
    }

    #[test]
    fn apply_dynamic_keeps_static_hardware_and_shrinks_selection() {
        let mut snap = snapshot_with_procs(5);
        snap.mobo.product = "Z390".to_string();
        snap.array.total_slots = 4;
        let mut app = App::new(snap, Duration::from_secs(2));
        app.on_key(key(KeyCode::End));
        assert_eq!(app.table_state.borrow().selected(), Some(4));

        let mut fresh = snapshot_with_procs(2);
        fresh.mem.available_kb = 100;
        fresh.temps = vec![TempReading { label: "DIMM 0".into(), celsius: 44.4 }];
        app.apply_dynamic(fresh);

        assert_eq!(app.snapshot.mobo.product, "Z390", "static data must survive a tick");
        assert_eq!(app.snapshot.array.total_slots, 4);
        assert_eq!(app.snapshot.mem.available_kb, 100);
        assert_eq!(app.updates, 1);
        assert_eq!(app.table_state.borrow().selected(), Some(1), "selection clamped to new len");
        assert_eq!(app.temp_history.last(), Some(44));
        assert_eq!(app.used_history.last(), Some(900));
    }

    #[test]
    fn key_event_kind_is_available_for_the_loop_filter() {
        // Guards the import the event loop relies on.
        assert_eq!(key(KeyCode::Char('q')).kind, KeyEventKind::Press);
    }

    #[test]
    fn tab_index_round_trips() {
        for i in 0..TAB_TITLES.len() {
            assert_eq!(Tab::from_index(i).index(), i);
        }
        assert_eq!(Tab::from_index(99), Tab::Overview);
    }
}
