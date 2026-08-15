//! macOS parsers.
//!
//! Data sources: `sysctl` (total RAM, swap usage, hardware model), `vm_stat`
//! (page-level memory statistics), `ps` (per-process RSS), and
//! `system_profiler SPMemoryDataType` (memory modules — DIMM banks on Intel
//! Macs, soldered unified memory on Apple Silicon).
//!
//! Every subprocess collector degrades gracefully: if a command is missing or
//! fails (e.g. when this module is compiled on a non-macOS host), it returns
//! empty/default data and never panics. The pure text-parsing functions are
//! compiled and unit-tested on every platform.

use std::process::Command;
use crate::types::*;

// ─── Collectors ───────────────────────────────────────────────────────────────

/// Collect a complete [`Snapshot`] of all memory-related system data on macOS.
///
/// Gathers memory stats (`sysctl` + `vm_stat`), memory modules and array info
/// (`system_profiler SPMemoryDataType`), top memory consumers (`ps`), and the
/// hardware model (`sysctl hw.model`). macOS exposes no RAM temperature
/// sensors, so `temps` is always empty; `pi` is always `None`.
pub fn collect_snapshot() -> Snapshot {
    let (dimms, array) = read_memory_modules();
    Snapshot {
        mem: collect_mem_stats(),
        dimms,
        array,
        temps: vec![],
        top_consumers: top_mem_consumers(10),
        pi: None,
        mobo: read_mobo_info(),
    }
}

/// Collect only the frequently-changing data — memory stats and top consumers —
/// into a [`Snapshot`], leaving the static hardware fields (`dimms`, `array`,
/// `mobo`, `pi`) at their defaults.
///
/// Intended for monitor mode: static module/model details don't change between
/// refreshes, and re-running `system_profiler` (a slow subprocess) every cycle
/// is wasteful, so this skips it entirely.
pub fn collect_dynamic() -> Snapshot {
    Snapshot {
        mem: collect_mem_stats(),
        top_consumers: top_mem_consumers(10),
        ..Default::default()
    }
}

/// Collect memory usage statistics on macOS — the cheapest possible read:
/// total RAM from `sysctl hw.memsize`, page-level stats from `vm_stat`, and
/// swap usage from `sysctl vm.swapusage`.
pub fn collect_mem_stats() -> MemStats {
    let total_kb = read_total_mem_kb();
    let vm_stat = run_command("vm_stat", &[]).unwrap_or_default();
    let mut stats = parse_vm_stat_output(&vm_stat, total_kb);

    let swap = run_command("sysctl", &["vm.swapusage"]).unwrap_or_default();
    let (swap_total_kb, swap_free_kb) = parse_swap_usage(&swap);
    stats.swap_total_kb = swap_total_kb;
    stats.swap_free_kb = swap_free_kb;
    stats
}

// ─── Pure parsers ─────────────────────────────────────────────────────────────

/// Parse `vm_stat` output text into a [`MemStats`].
///
/// The page size is taken from the header line
/// `Mach Virtual Memory Statistics: (page size of 16384 bytes)` (defaulting to
/// 4096 bytes if absent). Statistic lines look like
/// `Pages free:                              12345.` (note the trailing dot).
///
/// Fills `free_kb` from "Pages free", `available_kb` as free + inactive +
/// speculative pages, and `cached_kb` from "File-backed pages" (0 if absent).
/// `total_kb` comes from the caller (macOS reports it via `sysctl hw.memsize`,
/// not `vm_stat`); `buffers_kb` and the swap fields are left at 0 (swap is
/// filled separately from `sysctl vm.swapusage`).
pub fn parse_vm_stat_output(text: &str, total_kb: u64) -> MemStats {
    let mut page_size: u64 = 4096;
    let mut free_pages: u64 = 0;
    let mut inactive_pages: u64 = 0;
    let mut speculative_pages: u64 = 0;
    let mut file_backed_pages: u64 = 0;

    for line in text.lines() {
        let line = line.trim();

        // Header: "Mach Virtual Memory Statistics: (page size of 16384 bytes)"
        if line.contains("page size of") {
            let size = line
                .split("page size of")
                .nth(1)
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|tok| tok.parse::<u64>().ok())
                .unwrap_or(0);
            if size > 0 { page_size = size; }
            continue;
        }

        let Some((key, val)) = line.split_once(':') else { continue };
        let pages: u64 = val.trim().trim_end_matches('.').parse().unwrap_or(0);
        match key.trim() {
            "Pages free"        => free_pages = pages,
            "Pages inactive"    => inactive_pages = pages,
            "Pages speculative" => speculative_pages = pages,
            "File-backed pages" => file_backed_pages = pages,
            _ => {}
        }
    }

    let to_kb = |pages: u64| pages.saturating_mul(page_size) / 1024;
    MemStats {
        total_kb,
        free_kb: to_kb(free_pages),
        available_kb: to_kb(free_pages + inactive_pages + speculative_pages),
        buffers_kb: 0,
        cached_kb: to_kb(file_backed_pages),
        swap_total_kb: 0,
        swap_free_kb: 0,
    }
}

/// Parse `sysctl vm.swapusage` output like
/// `vm.swapusage: total = 2048.00M  used = 1234.56M  free = 813.44M`
/// into `(swap_total_kb, swap_free_kb)`.
///
/// Values may be decimal and carry a `K`/`M`/`G` suffix. Returns `(0, 0)` if
/// the text can't be parsed.
pub fn parse_swap_usage(text: &str) -> (u64, u64) {
    let total_kb = find_swap_field_kb(text, "total").unwrap_or(0);
    if total_kb == 0 { return (0, 0); }
    let free_kb = find_swap_field_kb(text, "free").unwrap_or(0);
    (total_kb, free_kb)
}

/// Scan `text` for `<name> = <value>` and convert the value to kilobytes.
fn find_swap_field_kb(text: &str, name: &str) -> Option<u64> {
    let mut tokens = text.split_whitespace();
    while let Some(tok) = tokens.next() {
        if tok == name && tokens.next() == Some("=") {
            return parse_sized_value_kb(tokens.next()?);
        }
    }
    None
}

/// Convert a value like `2048.00M`, `813.44K`, or `1.50G` to kilobytes.
fn parse_sized_value_kb(token: &str) -> Option<u64> {
    let (num, factor) = match token.chars().last()? {
        'K' | 'k' => (&token[..token.len() - 1], 1.0),
        'M' | 'm' => (&token[..token.len() - 1], 1024.0),
        'G' | 'g' => (&token[..token.len() - 1], 1024.0 * 1024.0),
        _ => return None,
    };
    let value: f64 = num.parse().ok()?;
    if !value.is_finite() || value < 0.0 { return None; }
    Some((value * factor) as u64)
}

/// Parse `ps axo pid=,rss=,comm=` output into the top `n` processes by RSS.
///
/// Each line looks like `  512 123456 /usr/sbin/foo`; RSS is already in
/// kilobytes on macOS. The process name is the last path component of the
/// command (which may itself contain spaces). Processes with zero RSS are
/// dropped; the result is sorted by RSS descending and truncated to `n`.
pub fn parse_ps_output(text: &str, n: usize) -> Vec<ProcessMem> {
    let mut procs: Vec<ProcessMem> = text
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let (pid_str, rest) = line.split_once(char::is_whitespace)?;
            let (rss_str, comm) = rest.trim_start().split_once(char::is_whitespace)?;

            let pid: u32 = pid_str.parse().ok()?;
            let rss_kb: u64 = rss_str.parse().ok()?;
            if rss_kb == 0 { return None; }

            let name = comm.trim().rsplit('/').next().unwrap_or("").to_string();
            if name.is_empty() { return None; }
            Some(ProcessMem { pid, name, rss_kb })
        })
        .collect();

    procs.sort_by(|a, b| b.rss_kb.cmp(&a.rss_kb));
    procs.truncate(n);
    procs
}

/// Parse `system_profiler SPMemoryDataType` text into DIMM slots and a
/// [`MemArrayInfo`]. Handles both output shapes:
///
/// - **Intel Macs** — indented `BANK 0/DIMM0:` blocks, each with `Size`,
///   `Type`, `Speed`, `Manufacturer` (raw hex codes are kept as-is), and
///   `Part Number` fields. One [`DimmSlot`] per populated bank (banks with
///   `Size: Empty` are skipped); the locator is the bank header without its
///   trailing colon. `MemArrayInfo.total_slots` counts all banks, including
///   empty ones.
/// - **Apple Silicon** — a top-level `Memory:` section with `Memory: 16 GB`,
///   `Type: LPDDR5`, and `Manufacturer:` lines describing soldered unified
///   memory. Produces a single [`DimmSlot`] with locator `Soldered` and
///   `MemArrayInfo::default()` (no physical slots).
///
/// `max_capacity_mb` is always 0 — `system_profiler` does not report it.
pub fn parse_system_profiler_memory(text: &str) -> (Vec<DimmSlot>, MemArrayInfo) {
    let mut slots: Vec<DimmSlot> = Vec::new();
    let mut current: Option<DimmSlot> = None;
    let mut bank_count: u32 = 0;

    // Apple Silicon top-level fields (used only when no BANK blocks exist).
    let mut soldered_size_mb: u64 = 0;
    let mut soldered_type = String::new();
    let mut soldered_manufacturer = String::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }

        // Bank header, e.g. "BANK 0/DIMM0:" (Intel shape).
        if line.starts_with("BANK") && line.ends_with(':') {
            if let Some(s) = current.take() {
                if s.size_mb > 0 { slots.push(s); }
            }
            bank_count += 1;
            current = Some(DimmSlot {
                locator: line.trim_end_matches(':').to_string(),
                ..Default::default()
            });
            continue;
        }

        let Some((key, val)) = line.split_once(':') else { continue };
        let (key, val) = (key.trim(), val.trim());
        if val.is_empty() { continue; } // section headers like "Memory Slots:"

        if let Some(slot) = current.as_mut() {
            match key {
                "Size"         => slot.size_mb = parse_profiler_size_mb(val),
                "Type"         => slot.mem_type = val.to_string(),
                "Manufacturer" => slot.manufacturer = val.to_string(),
                "Part Number"  => slot.part_number = val.to_string(),
                "Speed" => {
                    slot.speed_mhz = val.split_whitespace().next()
                        .unwrap_or("0").parse().unwrap_or(0);
                }
                _ => {}
            }
        } else {
            // Top-level lines (Apple Silicon shape).
            match key {
                "Memory"       => soldered_size_mb = parse_profiler_size_mb(val),
                "Type"         => soldered_type = val.to_string(),
                "Manufacturer" => soldered_manufacturer = val.to_string(),
                _ => {}
            }
        }
    }
    if let Some(s) = current.take() {
        if s.size_mb > 0 { slots.push(s); }
    }

    if bank_count == 0 && soldered_size_mb > 0 {
        slots.push(DimmSlot {
            locator: "Soldered".to_string(),
            size_mb: soldered_size_mb,
            mem_type: soldered_type,
            manufacturer: soldered_manufacturer,
            ..Default::default()
        });
    }

    (slots, MemArrayInfo { total_slots: bank_count, max_capacity_mb: 0 })
}

/// Convert a `system_profiler` size value like `16 GB` or `512 MB` to
/// megabytes. Non-numeric values (e.g. `Empty`) yield 0.
fn parse_profiler_size_mb(val: &str) -> u64 {
    if val.contains("GB") {
        val.split_whitespace().next().unwrap_or("0")
            .parse::<u64>().unwrap_or(0) * 1024
    } else if val.contains("MB") {
        val.split_whitespace().next().unwrap_or("0")
            .parse().unwrap_or(0)
    } else {
        0
    }
}

// ─── Subprocess helpers ───────────────────────────────────────────────────────

/// Run a command and return its stdout, or `None` if it's missing or fails.
fn run_command(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    if !out.status.success() { return None; }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Read total physical RAM in kilobytes from `sysctl -n hw.memsize` (bytes).
fn read_total_mem_kb() -> u64 {
    run_command("sysctl", &["-n", "hw.memsize"])
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|bytes| bytes / 1024)
        .unwrap_or(0)
}

/// Collect the top `n` processes by RSS via `ps axo pid=,rss=,comm=`.
fn top_mem_consumers(n: usize) -> Vec<ProcessMem> {
    let text = run_command("ps", &["axo", "pid=,rss=,comm="]).unwrap_or_default();
    parse_ps_output(&text, n)
}

/// Read memory modules and array info via
/// `system_profiler SPMemoryDataType -detailLevel mini`.
fn read_memory_modules() -> (Vec<DimmSlot>, MemArrayInfo) {
    let text = run_command("system_profiler", &["SPMemoryDataType", "-detailLevel", "mini"])
        .unwrap_or_default();
    parse_system_profiler_memory(&text)
}

/// Read the hardware model via `sysctl -n hw.model` into a [`MoboInfo`]
/// (manufacturer `Apple`). Returns default (empty) info on failure.
fn read_mobo_info() -> MoboInfo {
    match run_command("sysctl", &["-n", "hw.model"]) {
        Some(model) if !model.trim().is_empty() => MoboInfo {
            manufacturer: "Apple".to_string(),
            product: model.trim().to_string(),
        },
        _ => MoboInfo::default(),
    }
}
