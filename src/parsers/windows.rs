//! Windows parsers.
//!
//! Gathers system data by shelling out to PowerShell (`powershell`, falling
//! back to `pwsh`) with small `Get-CimInstance` queries whose output is
//! formatted as culture-invariant `Key=Value` lines, one property per line,
//! with multi-object results separated by a line containing only `---`.
//! The pure text parsers below are compiled (and unit-tested) on every
//! platform; only the collectors actually invoke PowerShell, and every one of
//! them degrades gracefully to empty/default data when it is unavailable.

use std::process::Command;
use crate::types::*;

// ─── PowerShell plumbing ──────────────────────────────────────────────────────

/// Run a PowerShell script via `powershell -NoProfile -NonInteractive -Command`,
/// falling back to `pwsh` with the same arguments. Returns stdout on success,
/// `None` if neither shell is available or the script fails.
fn run_powershell(script: &str) -> Option<String> {
    for shell in ["powershell", "pwsh"] {
        if let Ok(out) = Command::new(shell)
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
        {
            if out.status.success() {
                return Some(String::from_utf8_lossy(&out.stdout).to_string());
            }
        }
    }
    None
}

/// Split `Key=Value` output into per-object blocks (separated by a line
/// containing only `---`), each block a list of `(key, value)` pairs.
/// Lines without `=` are ignored; keys and values are trimmed.
fn parse_kv_blocks(text: &str) -> Vec<Vec<(&str, &str)>> {
    let mut blocks: Vec<Vec<(&str, &str)>> = vec![Vec::new()];
    for line in text.lines() {
        let line = line.trim();
        if line == "---" {
            blocks.push(Vec::new());
            continue;
        }
        let Some((key, val)) = line.split_once('=') else { continue };
        if let Some(last) = blocks.last_mut() {
            last.push((key.trim(), val.trim()));
        }
    }
    blocks.retain(|b| !b.is_empty());
    blocks
}

/// Parse a `Key=Value` value as `u64`, returning 0 on garbage or empty input.
fn parse_u64(val: &str) -> u64 {
    val.trim().parse().unwrap_or(0)
}

// ─── Collectors ───────────────────────────────────────────────────────────────

/// Collect a complete [`Snapshot`] of all memory-related system data in one call.
///
/// Windows counterpart of the Linux entry point: gathers memory stats (with
/// pagefile usage as swap), DIMM slots, the physical memory array, top memory
/// consumers, and motherboard details via PowerShell CIM queries. Temperatures
/// are not exposed through CIM, so `temps` is always empty, and `pi` is `None`.
/// Every underlying collector degrades gracefully (returning empty/default
/// data) when PowerShell or a CIM class is unavailable, so this never panics.
pub fn collect_snapshot() -> Snapshot {
    Snapshot {
        mem: collect_mem_stats(),
        dimms: collect_dimms(),
        array: collect_memory_array(),
        temps: Vec::new(),
        top_consumers: collect_top_consumers(10),
        pi: None,
        mobo: collect_mobo(),
    }
}

/// Collect only the frequently-changing data — memory stats and top consumers —
/// into a [`Snapshot`], leaving the static hardware fields (`dimms`, `array`,
/// `mobo`, `pi`) at their defaults.
///
/// Intended for monitor mode: static DIMM/motherboard details don't change
/// between refreshes, and re-running the WMI hardware queries every cycle is
/// wasteful, so this skips them entirely.
pub fn collect_dynamic() -> Snapshot {
    Snapshot {
        mem: collect_mem_stats(),
        top_consumers: collect_top_consumers(10),
        ..Default::default()
    }
}

/// Collect memory usage statistics from `Win32_OperatingSystem`, plus swap
/// (pagefile) totals from `Win32_PageFileUsage`. Returns default (zeroed)
/// stats if PowerShell is unavailable.
pub fn collect_mem_stats() -> MemStats {
    let script = r#"Get-CimInstance Win32_OperatingSystem | ForEach-Object { "TotalVisibleMemorySize=$($_.TotalVisibleMemorySize)"; "FreePhysicalMemory=$($_.FreePhysicalMemory)" }"#;
    let mut stats = match run_powershell(script) {
        Some(text) => parse_memory_status(&text),
        None => MemStats::default(),
    };

    let script = r#"Get-CimInstance Win32_PageFileUsage | ForEach-Object { "AllocatedBaseSize=$($_.AllocatedBaseSize)"; "CurrentUsage=$($_.CurrentUsage)"; "---" }"#;
    if let Some(text) = run_powershell(script) {
        let (swap_total, swap_free) = parse_pagefile(&text);
        stats.swap_total_kb = swap_total;
        stats.swap_free_kb = swap_free;
    }
    stats
}

/// Query `Win32_PhysicalMemory` and parse the installed DIMM modules.
/// Returns an empty vec if PowerShell is unavailable or the query fails.
fn collect_dimms() -> Vec<DimmSlot> {
    let script = r#"Get-CimInstance Win32_PhysicalMemory | ForEach-Object { "DeviceLocator=$($_.DeviceLocator)"; "Capacity=$($_.Capacity)"; "Speed=$($_.Speed)"; "ConfiguredClockSpeed=$($_.ConfiguredClockSpeed)"; "Manufacturer=$($_.Manufacturer)"; "PartNumber=$($_.PartNumber)"; "SMBIOSMemoryType=$($_.SMBIOSMemoryType)"; "ConfiguredVoltage=$($_.ConfiguredVoltage)"; "---" }"#;
    match run_powershell(script) {
        Some(text) => parse_physical_memory(&text),
        None => Vec::new(),
    }
}

/// Query `Win32_PhysicalMemoryArray` for slot count and maximum capacity.
/// Returns default (zeroed) info if PowerShell is unavailable.
fn collect_memory_array() -> MemArrayInfo {
    let script = r#"Get-CimInstance Win32_PhysicalMemoryArray | ForEach-Object { "MemoryDevices=$($_.MemoryDevices)"; "MaxCapacityEx=$($_.MaxCapacityEx)"; "MaxCapacity=$($_.MaxCapacity)"; "---" }"#;
    match run_powershell(script) {
        Some(text) => parse_memory_array(&text),
        None => MemArrayInfo::default(),
    }
}

/// Query `Win32_BaseBoard` for motherboard identification.
/// Returns default (empty) info if PowerShell is unavailable.
fn collect_mobo() -> MoboInfo {
    let script = r#"Get-CimInstance Win32_BaseBoard | ForEach-Object { "Manufacturer=$($_.Manufacturer)"; "Product=$($_.Product)" }"#;
    match run_powershell(script) {
        Some(text) => parse_baseboard(&text),
        None => MoboInfo::default(),
    }
}

/// Query `Get-Process` for the top memory consumers by working set.
/// Returns an empty vec if PowerShell is unavailable.
fn collect_top_consumers(n: usize) -> Vec<ProcessMem> {
    let script = r#"Get-Process | Sort-Object WS -Descending | Select-Object -First 15 | ForEach-Object { "Id=$($_.Id)"; "Name=$($_.Name)"; "WS=$($_.WS)"; "---" }"#;
    match run_powershell(script) {
        Some(text) => parse_process_list(&text, n),
        None => Vec::new(),
    }
}

// ─── Pure parsers ─────────────────────────────────────────────────────────────

/// Parse `Win32_OperatingSystem` `Key=Value` output into a [`MemStats`].
///
/// Expects `TotalVisibleMemorySize=<KB>` and `FreePhysicalMemory=<KB>`.
/// Windows exposes no direct equivalents of Linux buffers/cached or a separate
/// "available" figure through this class, so `available_kb` mirrors `free_kb`,
/// buffers/cached are 0, and swap fields are left untouched (0) — the caller
/// fills swap from [`parse_pagefile`].
pub fn parse_memory_status(text: &str) -> MemStats {
    let mut s = MemStats::default();
    for block in parse_kv_blocks(text) {
        for (key, val) in block {
            match key {
                "TotalVisibleMemorySize" => s.total_kb = parse_u64(val),
                "FreePhysicalMemory"     => s.free_kb = parse_u64(val),
                _ => {}
            }
        }
    }
    s.available_kb = s.free_kb;
    s
}

/// Parse `Win32_PageFileUsage` `Key=Value` output into `(swap_total_kb, swap_free_kb)`.
///
/// Expects `AllocatedBaseSize=<MB>` and `CurrentUsage=<MB>` per pagefile, with
/// multiple pagefiles separated by `---` lines (their sizes are summed).
/// Free space is allocated minus used, converted from MB to KB. Returns
/// `(0, 0)` on empty or garbage input.
pub fn parse_pagefile(text: &str) -> (u64, u64) {
    let mut alloc_mb: u64 = 0;
    let mut usage_mb: u64 = 0;
    for block in parse_kv_blocks(text) {
        for (key, val) in block {
            match key {
                "AllocatedBaseSize" => alloc_mb += parse_u64(val),
                "CurrentUsage"      => usage_mb += parse_u64(val),
                _ => {}
            }
        }
    }
    let total_kb = alloc_mb * 1024;
    let free_kb = alloc_mb.saturating_sub(usage_mb) * 1024;
    (total_kb, free_kb)
}

/// Map an SMBIOS memory type code (`SMBIOSMemoryType`) to a display name.
fn smbios_mem_type(code: u64) -> &'static str {
    match code {
        20 => "DDR",
        21 => "DDR2",
        24 => "DDR3",
        26 => "DDR4",
        27 => "LPDDR",
        28 => "LPDDR2",
        29 => "LPDDR3",
        30 => "LPDDR4",
        34 => "DDR5",
        35 => "LPDDR5",
        _  => "RAM",
    }
}

/// Parse `Win32_PhysicalMemory` `Key=Value` output into DIMM slots.
///
/// Modules are separated by `---` lines. `Capacity` is in bytes (converted to
/// megabytes), `Speed`/`ConfiguredClockSpeed` in MT/s, `SMBIOSMemoryType` is a
/// numeric SMBIOS code mapped to a display name (unknown codes become `RAM`),
/// and `ConfiguredVoltage` is in millivolts, formatted as e.g. `1.2 V` (empty
/// string if zero or absent). Slots with zero capacity are dropped.
pub fn parse_physical_memory(text: &str) -> Vec<DimmSlot> {
    let mut slots = Vec::new();
    for block in parse_kv_blocks(text) {
        let mut slot = DimmSlot { mem_type: "RAM".to_string(), ..Default::default() };
        for (key, val) in block {
            match key {
                "DeviceLocator"        => slot.locator = val.to_string(),
                "Capacity"             => slot.size_mb = parse_u64(val) / (1024 * 1024),
                "Speed"                => slot.speed_mhz = parse_u64(val),
                "ConfiguredClockSpeed" => slot.configured_speed = parse_u64(val),
                "Manufacturer"         => slot.manufacturer = val.to_string(),
                "PartNumber"           => slot.part_number = val.trim().to_string(),
                "SMBIOSMemoryType"     => slot.mem_type = smbios_mem_type(parse_u64(val)).to_string(),
                "ConfiguredVoltage" => {
                    let mv = parse_u64(val);
                    if mv > 0 {
                        slot.voltage = format!("{:.1} V", mv as f64 / 1000.0);
                    }
                }
                _ => {}
            }
        }
        if slot.size_mb > 0 {
            slots.push(slot);
        }
    }
    slots
}

/// Parse `Win32_PhysicalMemoryArray` `Key=Value` output into a [`MemArrayInfo`].
///
/// Expects `MemoryDevices=<count>` and a maximum capacity in kilobytes from
/// `MaxCapacityEx` (preferred; supports > 2 TB) with `MaxCapacity` as a
/// fallback when the extended field is absent or zero.
pub fn parse_memory_array(text: &str) -> MemArrayInfo {
    let mut info = MemArrayInfo::default();
    let mut max_ex_kb: u64 = 0;
    let mut max_kb: u64 = 0;
    for block in parse_kv_blocks(text) {
        for (key, val) in block {
            match key {
                "MemoryDevices" => info.total_slots = val.trim().parse().unwrap_or(0),
                "MaxCapacityEx" => max_ex_kb = parse_u64(val),
                "MaxCapacity"   => max_kb = parse_u64(val),
                _ => {}
            }
        }
    }
    info.max_capacity_mb = if max_ex_kb > 0 { max_ex_kb } else { max_kb } / 1024;
    info
}

/// Parse `Win32_BaseBoard` `Key=Value` output into a [`MoboInfo`].
///
/// Expects `Manufacturer=` and `Product=` lines.
pub fn parse_baseboard(text: &str) -> MoboInfo {
    let mut info = MoboInfo::default();
    for block in parse_kv_blocks(text) {
        for (key, val) in block {
            match key {
                "Manufacturer" => info.manufacturer = val.to_string(),
                "Product"      => info.product = val.to_string(),
                _ => {}
            }
        }
    }
    info
}

/// Parse `Get-Process` `Key=Value` output into the top `n` memory consumers.
///
/// Processes are separated by `---` lines, each with `Id=`, `Name=`, and
/// `WS=<bytes>` (working set, converted to `rss_kb`). Entries with zero
/// working set are dropped; the result is sorted by memory descending and
/// truncated to `n`.
pub fn parse_process_list(text: &str, n: usize) -> Vec<ProcessMem> {
    let mut procs: Vec<ProcessMem> = Vec::new();
    for block in parse_kv_blocks(text) {
        let mut pid: u32 = 0;
        let mut name = String::new();
        let mut rss_kb: u64 = 0;
        for (key, val) in block {
            match key {
                "Id"   => pid = val.trim().parse().unwrap_or(0),
                "Name" => name = val.to_string(),
                "WS"   => rss_kb = parse_u64(val) / 1024,
                _ => {}
            }
        }
        if rss_kb > 0 {
            procs.push(ProcessMem { pid, name, rss_kb });
        }
    }
    procs.sort_by(|a, b| b.rss_kb.cmp(&a.rss_kb));
    procs.truncate(n);
    procs
}
