use std::io::{self, IsTerminal, Write};
use std::process::exit;
use std::thread::sleep;
use std::time::Duration;

use raminfo::json;
use raminfo::parsers::{collect_dynamic, collect_mem_stats, collect_snapshot};
use raminfo::render::{render_monitor, render_short, render_snapshot};

// ─── CLI ────────────────────────────────────────────────────────────────────────

const USAGE: &str = "\
raminfo — RAM inspector (Linux, macOS, Windows)

USAGE:
    raminfo [OPTIONS]

OPTIONS:
    --short               Compact free(1)-style summary (default without sudo)
    --full                Full report: hardware, temps, top consumers (default with sudo)
    --json                Output a single full JSON snapshot instead of the TUI
    --monitor             Continuously refresh (TUI, or ndjson stream with --json)
    --interval <seconds>  Refresh rate for --monitor mode (default: 2)
    -h, --help            Print this help and exit

Run with sudo for DIMM slot / motherboard details (requires dmidecode on Linux).";

/// Output detail level for the single-shot TUI.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    /// Compact `free -m`-style memory summary.
    Short,
    /// Full report: hardware panels, temperatures, top consumers.
    Full,
}

#[derive(Debug, PartialEq)]
struct Cli {
    json: bool,
    monitor: bool,
    interval: u64,
    /// Explicitly requested mode; `None` means auto (full as root, short otherwise).
    mode: Option<Mode>,
}

/// Parse command-line arguments (excluding argv[0]) into a [`Cli`].
///
/// Returns `Err` with a human-readable message on unknown flags, an invalid
/// `--interval` value, or conflicting `--short`/`--full`. `--help`/`-h` is
/// handled by the caller before this runs.
fn parse_args(args: &[String]) -> Result<Cli, String> {
    let mut cli = Cli { json: false, monitor: false, interval: 2, mode: None };
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--json" => cli.json = true,
            "--monitor" => cli.monitor = true,
            "--short" => set_mode(&mut cli, Mode::Short)?,
            "--full" => set_mode(&mut cli, Mode::Full)?,
            "--interval" => {
                let val = args.get(i + 1)
                    .ok_or_else(|| "--interval requires a value (seconds)".to_string())?;
                cli.interval = parse_interval(val)?;
                i += 1;
            }
            _ => {
                if let Some(val) = arg.strip_prefix("--interval=") {
                    cli.interval = parse_interval(val)?;
                } else {
                    return Err(format!("unknown argument: {arg}"));
                }
            }
        }
        i += 1;
    }
    Ok(cli)
}

/// Record an explicit `--short`/`--full` choice, rejecting contradictions.
fn set_mode(cli: &mut Cli, mode: Mode) -> Result<(), String> {
    match cli.mode {
        Some(m) if m != mode => Err("--short and --full are mutually exclusive".to_string()),
        _ => {
            cli.mode = Some(mode);
            Ok(())
        }
    }
}

/// Parse and validate an `--interval` value: a positive integer number of seconds.
fn parse_interval(val: &str) -> Result<u64, String> {
    match val.parse::<u64>() {
        Ok(0) => Err("--interval must be at least 1 second".to_string()),
        Ok(n) => Ok(n),
        Err(_) => Err(format!("invalid --interval value: {val}")),
    }
}

/// True when running with root privileges (always false on non-Unix platforms).
///
/// Decides the default output mode: root implies the full report (hardware
/// details are readable), otherwise the short summary.
#[cfg(unix)]
fn is_root() -> bool {
    // SAFETY: geteuid() has no preconditions and cannot fail.
    unsafe { libc::geteuid() == 0 }
}

#[cfg(not(unix))]
fn is_root() -> bool {
    false
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    #[cfg(windows)]
    let _ = colored::control::set_virtual_terminal(true);

    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return;
    }

    let cli = match parse_args(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("raminfo: {e}");
            eprintln!("Try 'raminfo --help' for more information.");
            exit(1);
        }
    };

    if cli.monitor {
        run_monitor(&cli);
    } else if cli.json {
        // JSON output is always the full snapshot regardless of mode.
        println!("{}", json::to_json(&collect_snapshot()));
    } else {
        let mode = cli.mode.unwrap_or(if is_root() { Mode::Full } else { Mode::Short });
        match mode {
            Mode::Short => render_short(&collect_mem_stats()),
            Mode::Full => render_snapshot(&collect_snapshot()),
        }
    }
}

/// Continuously collect and emit snapshots until interrupted (Ctrl+C).
///
/// Monitor mode only refreshes the data that actually changes — memory usage,
/// temperatures, and top consumers — via [`collect_dynamic`]; static DIMM /
/// motherboard details are omitted (and `dmidecode` is not re-run each cycle).
///
/// With `--json`, prints one compact JSON object per line (ndjson) — no screen
/// control, so it can be piped/logged. Otherwise, when stdout is a terminal, it
/// uses the alternate screen buffer (like `htop`/`btop`): the TUI redraws in
/// place with no scrollback buildup, and the original terminal contents are
/// restored on exit.
fn run_monitor(cli: &Cli) {
    let interval = Duration::from_secs(cli.interval);

    // ndjson stream: just print one object per cycle, no cursor/screen escapes.
    if cli.json {
        loop {
            println!("{}", json::to_json_monitor(&collect_dynamic()));
            let _ = io::stdout().flush();
            sleep(interval);
        }
    }

    let tty = io::stdout().is_terminal();

    if tty {
        // Enter the alternate screen buffer and hide the cursor.
        print!("\x1B[?1049h\x1B[?25l");
        let _ = io::stdout().flush();
        // Restore the terminal (show cursor, leave alt screen) on Ctrl+C.
        let _ = ctrlc::set_handler(|| {
            print!("\x1B[?25h\x1B[?1049l");
            let _ = io::stdout().flush();
            exit(0);
        });
    }

    loop {
        let snapshot = collect_dynamic();
        if tty {
            // Home the cursor, redraw, then erase anything left below.
            print!("\x1B[H");
            render_monitor(&snapshot);
            println!("  Refreshing every {}s — press Ctrl+C to stop", cli.interval);
            print!("\x1B[J");
        } else {
            render_monitor(&snapshot);
        }
        let _ = io::stdout().flush();
        sleep(interval);
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn defaults() {
        let cli = parse_args(&args(&[])).unwrap();
        assert_eq!(cli, Cli { json: false, monitor: false, interval: 2, mode: None });
    }

    #[test]
    fn flags() {
        let cli = parse_args(&args(&["--json", "--monitor"])).unwrap();
        assert!(cli.json);
        assert!(cli.monitor);
    }

    #[test]
    fn short_flag() {
        let cli = parse_args(&args(&["--short"])).unwrap();
        assert_eq!(cli.mode, Some(Mode::Short));
    }

    #[test]
    fn full_flag() {
        let cli = parse_args(&args(&["--full"])).unwrap();
        assert_eq!(cli.mode, Some(Mode::Full));
    }

    #[test]
    fn repeated_mode_flag_ok() {
        let cli = parse_args(&args(&["--short", "--short"])).unwrap();
        assert_eq!(cli.mode, Some(Mode::Short));
    }

    #[test]
    fn conflicting_modes_rejected() {
        assert!(parse_args(&args(&["--short", "--full"])).is_err());
        assert!(parse_args(&args(&["--full", "--short"])).is_err());
    }

    #[test]
    fn interval_space_form() {
        let cli = parse_args(&args(&["--interval", "5"])).unwrap();
        assert_eq!(cli.interval, 5);
    }

    #[test]
    fn interval_equals_form() {
        let cli = parse_args(&args(&["--interval=10"])).unwrap();
        assert_eq!(cli.interval, 10);
    }

    #[test]
    fn interval_zero_rejected() {
        assert!(parse_args(&args(&["--interval", "0"])).is_err());
    }

    #[test]
    fn interval_missing_value_rejected() {
        assert!(parse_args(&args(&["--interval"])).is_err());
    }

    #[test]
    fn interval_non_numeric_rejected() {
        assert!(parse_args(&args(&["--interval", "abc"])).is_err());
    }

    #[test]
    fn unknown_flag_rejected() {
        assert!(parse_args(&args(&["--bogus"])).is_err());
    }
}
