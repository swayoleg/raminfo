//! The interactive full-screen ratatui application.
//!
//! [`run`] owns all the terminal I/O and data collection; the state machine
//! ([`app::App`]) and the rendering ([`ui::draw`]) are pure and unit/render
//! tested. Static hardware data (DIMMs, memory array, motherboard, Raspberry Pi
//! board) is collected exactly once at startup — only
//! [`collect_dynamic`](crate::parsers::collect_dynamic) runs on each tick, so
//! `dmidecode` / WMI are never re-queried while the app is open.

pub mod app;
pub mod history;
pub mod ui;

use std::io;
use std::time::{Duration, Instant};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::parsers::{collect_dynamic, collect_snapshot};

use app::App;

/// How long to wait for a key press before re-checking the tick deadline.
const POLL: Duration = Duration::from_millis(150);

/// Run the interactive TUI until the user quits.
///
/// `interval` is the data refresh rate; it can be changed at runtime with
/// `+`/`-`. The caller must have verified that stdout is a terminal.
pub fn run(interval: Duration) -> io::Result<()> {
    // Collect the (potentially slow) full snapshot *before* switching to the
    // alternate screen, so any sudo/dmidecode output stays on the normal screen.
    let mut app = App::new(collect_snapshot(), interval);

    // `try_init` enables raw mode, switches to the alternate screen and installs
    // a panic hook that restores the terminal first.
    let terminal = ratatui::try_init()?;
    let result = event_loop(terminal, &mut app);
    ratatui::restore();
    result
}

/// The draw / poll / tick loop. Never sleeps: the tick is an elapsed-time check
/// so key presses stay responsive regardless of the refresh interval.
fn event_loop(mut terminal: DefaultTerminal, app: &mut App) -> io::Result<()> {
    let mut last_tick = Instant::now();

    while !app.quit {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(POLL)?
            && let Event::Key(key) = event::read()?
            // Windows delivers Press *and* Release; without this filter every
            // key would be handled twice.
            && key.kind == KeyEventKind::Press
        {
            app.on_key(key);
        }

        // Quitting takes precedence: never pay for one more collection pass
        // after the user has pressed `q`.
        if app.quit {
            break;
        }

        if app.force_refresh || last_tick.elapsed() >= app.interval {
            app.force_refresh = false;
            last_tick = Instant::now();
            app.apply_dynamic(collect_dynamic());
        }
    }

    Ok(())
}
