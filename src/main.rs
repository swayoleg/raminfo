use std::io::{self, IsTerminal, Write};
use std::process::exit;
use std::thread::sleep;
use std::time::Duration;

use raminfo::json;
use raminfo::parsers::{collect_dynamic, collect_snapshot};
use raminfo::render::{render_monitor, render_snapshot};

// ─── CLI ────────────────────────────────────────────────────────────────────────

const USAGE: &str = "\
raminfo — Linux RAM inspector

USAGE:
    raminfo [OPTIONS]

OPTIONS:
    --json                Output a single JSON snapshot instead of the TUI
    --monitor             Continuously refresh (TUI, or ndjson stream with --json)
    --interval <seconds>  Refresh rate for --monitor mode (default: 2)
    -h, --help            Print this help and exit

Run with sudo for DIMM slot / motherboard details (requires dmidecode).";

#[derive(Debug, PartialEq)]
struct Cli {
    json: bool,
    monitor: bool,
    interval: u64,
}

/// Parse command-line arguments (excluding argv[0]) into a [`Cli`].
///
/// Returns `Err` with a human-readable message on unknown flags or an invalid
/// `--interval` value. `--help`/`-h` is handled by the caller before this runs.
fn parse_args(args: &[String]) -> Result<Cli, String> {
    let mut cli = Cli { json: false, monitor: false, interval: 2 };
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--json" => cli.json = true,
            "--monitor" => cli.monitor = true,
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

/// Parse and validate an `--interval` value: a positive integer number of seconds.
fn parse_interval(val: &str) -> Result<u64, String> {
    match val.parse::<u64>() {
        Ok(0) => Err("--interval must be at least 1 second".to_string()),
        Ok(n) => Ok(n),
        Err(_) => Err(format!("invalid --interval value: {val}")),
    }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
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
    } else {
        let snapshot = collect_snapshot();
        if cli.json {
            println!("{}", json::to_json(&snapshot));
        } else {
            render_snapshot(&snapshot);
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
        assert_eq!(cli, Cli { json: false, monitor: false, interval: 2 });
    }

    #[test]
    fn flags() {
        let cli = parse_args(&args(&["--json", "--monitor"])).unwrap();
        assert!(cli.json);
        assert!(cli.monitor);
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
