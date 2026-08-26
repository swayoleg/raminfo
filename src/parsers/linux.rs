use std::fs;
use std::process::Command;
use crate::types::*;

// ─── Parsers ──────────────────────────────────────────────────────────────────

/// Collect a complete [`Snapshot`] of all memory-related system data in one call.
///
/// This is the primary entry point for library consumers. It gathers memory
/// stats, DIMM slots, the physical memory array, temperatures, top memory
/// consumers, Raspberry Pi board info, and motherboard details. Every underlying
/// parser degrades gracefully (returning empty/default data) when a source is
/// unavailable, so this never panics — DIMM/motherboard details simply require
/// `dmidecode` (typically via sudo).
///
/// # Example
///
/// ```no_run
/// let snap = raminfo::parsers::collect_snapshot();
/// println!("Total RAM: {} kB", snap.mem.total_kb);
/// ```
pub fn collect_snapshot() -> Snapshot {
    Snapshot {
        mem: parse_proc_meminfo(),
        dimms: parse_dmidecode(),
        array: parse_dmidecode_array(),
        temps: read_ram_temps(),
        top_consumers: top_mem_consumers(10),
        pi: detect_raspberry_pi(),
        mobo: read_mobo_info(),
    }
}

/// Collect only the frequently-changing data — memory stats, temperatures, and
/// top consumers — into a [`Snapshot`], leaving the static hardware fields
/// (`dimms`, `array`, `mobo`, `pi`) at their defaults.
///
/// Intended for monitor mode: static DIMM/motherboard details don't change
/// between refreshes, and re-running `dmidecode` (a sudo subprocess) every cycle
/// is wasteful, so this skips it entirely.
pub fn collect_dynamic() -> Snapshot {
    collect_dynamic_top(10)
}

/// Like [`collect_dynamic`] but with `n` top consumers (`usize::MAX` = every
/// process with a resident set), sorted by RSS descending.
pub fn collect_dynamic_top(n: usize) -> Snapshot {
    Snapshot {
        mem: parse_proc_meminfo(),
        temps: read_ram_temps(),
        top_consumers: top_mem_consumers(n),
        ..Default::default()
    }
}

/// Read and parse `/proc/meminfo` into a [`MemStats`].
pub fn parse_proc_meminfo() -> MemStats {
    let content = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    parse_meminfo_content(&content)
}

/// Parse `/proc/meminfo`-formatted text into a [`MemStats`]. Unknown keys are ignored.
pub fn parse_meminfo_content(content: &str) -> MemStats {
    let mut s = MemStats::default();
    for line in content.lines() {
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 { continue; }
        let key = parts[0].trim();
        let val: u64 = parts[1].split_whitespace().next()
            .unwrap_or("0").parse().unwrap_or(0);
        match key {
            "MemTotal"     => s.total_kb = val,
            "MemFree"      => s.free_kb = val,
            "MemAvailable" => s.available_kb = val,
            "Buffers"      => s.buffers_kb = val,
            "Cached"       => s.cached_kb = val,
            "SwapTotal"    => s.swap_total_kb = val,
            "SwapFree"     => s.swap_free_kb = val,
            _ => {}
        }
    }
    s
}

/// Run `dmidecode -t 17` (via `sudo -n`, falling back to a direct call) and parse
/// the DIMM slots. Returns an empty vec if `dmidecode` is unavailable or fails.
pub fn parse_dmidecode() -> Vec<DimmSlot> {
    let output = Command::new("sudo")
        .args(["-n", "dmidecode", "-t", "17"])
        .output()
        .or_else(|_| Command::new("dmidecode").args(["-t", "17"]).output());

    let text = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return vec![],
    };

    parse_dmidecode_output(&text)
}

/// Parse `dmidecode -t 17` output text into DIMM slots. Slots with zero size are dropped.
pub fn parse_dmidecode_output(text: &str) -> Vec<DimmSlot> {
    let mut slots: Vec<DimmSlot> = Vec::new();
    let mut current: Option<DimmSlot> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("Memory Device") && !line.contains("Mapped") {
            if let Some(s) = current.take() { slots.push(s); }
            current = Some(DimmSlot::default());
            continue;
        }
        let Some(ref mut slot) = current else { continue };
        let kv: Vec<&str> = line.splitn(2, ':').collect();
        if kv.len() != 2 { continue; }
        let (key, val) = (kv[0].trim(), kv[1].trim().to_string());
        match key {
            "Locator"                 => slot.locator = val,
            "Type"                    => slot.mem_type = val,
            "Manufacturer"            => slot.manufacturer = val,
            "Part Number"             => slot.part_number = val.trim().to_string(),
            "Configured Voltage"      => slot.voltage = val,
            "Size" => {
                if val.contains("MB") {
                    slot.size_mb = val.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
                } else if val.contains("GB") {
                    slot.size_mb = val.split_whitespace().next().unwrap_or("0")
                        .parse::<u64>().unwrap_or(0) * 1024;
                }
            }
            "Speed" => {
                slot.speed_mhz = val.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
            }
            "Configured Memory Speed" => {
                slot.configured_speed = val.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
            }
            _ => {}
        }
    }
    if let Some(s) = current { slots.push(s); }
    slots.retain(|s| s.size_mb > 0);
    slots
}

/// Run `dmidecode -t 16` (via `sudo -n`, falling back to a direct call) and parse
/// the Physical Memory Array for total slot count and max capacity.
pub fn parse_dmidecode_array() -> MemArrayInfo {
    let output = Command::new("sudo")
        .args(["-n", "dmidecode", "-t", "16"])
        .output()
        .or_else(|_| Command::new("dmidecode").args(["-t", "16"]).output());

    let text = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return MemArrayInfo::default(),
    };

    parse_dmidecode_array_output(&text)
}

/// Parse `dmidecode -t 16` output text into a [`MemArrayInfo`] (slot count, max capacity).
pub fn parse_dmidecode_array_output(text: &str) -> MemArrayInfo {
    let mut info = MemArrayInfo::default();

    for line in text.lines() {
        let line = line.trim();
        let kv: Vec<&str> = line.splitn(2, ':').collect();
        if kv.len() != 2 { continue; }
        let (key, val) = (kv[0].trim(), kv[1].trim());
        match key {
            "Number Of Devices" => {
                info.total_slots = val.parse().unwrap_or(0);
            }
            "Maximum Capacity" => {
                // Format is e.g. "128 GB" or "64 GB" or "512 MB"
                if val.contains("GB") {
                    info.max_capacity_mb = val.split_whitespace().next()
                        .unwrap_or("0").parse::<u64>().unwrap_or(0) * 1024;
                } else if val.contains("MB") {
                    info.max_capacity_mb = val.split_whitespace().next()
                        .unwrap_or("0").parse().unwrap_or(0);
                } else if val.contains("TB") {
                    info.max_capacity_mb = val.split_whitespace().next()
                        .unwrap_or("0").parse::<u64>().unwrap_or(0) * 1024 * 1024;
                }
            }
            _ => {}
        }
    }
    info
}

/// Detect if running on a Raspberry Pi and return board info.
/// Returns None on non-Pi hardware.
pub fn detect_raspberry_pi() -> Option<PiBoardInfo> {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let mut info = parse_cpuinfo_for_pi(&cpuinfo)?;
    info.freq_mhz = read_vcgencmd_freq();
    info.voltage  = read_vcgencmd_voltage();
    Some(info)
}

/// Parse /proc/cpuinfo content and detect Raspberry Pi model.
/// Returns PiBoardInfo with freq_mhz=None and voltage=None (caller adds those).
pub fn parse_cpuinfo_for_pi(cpuinfo: &str) -> Option<PiBoardInfo> {
    let model = cpuinfo
        .lines()
        .find(|l| l.starts_with("Model"))?
        .splitn(2, ':')
        .nth(1)?
        .trim()
        .to_string();

    if !model.contains("Raspberry Pi") {
        return None;
    }

    // Map model string to LPDDR type:
    // Pi 5 → LPDDR4X, Pi 4 → LPDDR4, older → LPDDR2
    let mem_type = if model.contains("Pi 5") {
        "LPDDR4X"
    } else if model.contains("Pi 4") || model.contains("Pi Compute Module 4") {
        "LPDDR4"
    } else if model.contains("Pi 3") || model.contains("Pi Compute Module 3") {
        "LPDDR2"
    } else {
        "LPDDR"
    }.to_string();

    Some(PiBoardInfo { model, mem_type, freq_mhz: None, voltage: None })
}

/// Run `vcgencmd measure_clock sdram_c` and return frequency in MT/s.
fn read_vcgencmd_freq() -> Option<u64> {
    let out = Command::new("vcgencmd")
        .args(["measure_clock", "sdram_c"])
        .output()
        .ok()?;

    if !out.status.success() { return None; }

    // Output format: "frequency(0)=1066000000"
    let text = String::from_utf8_lossy(&out.stdout);
    let hz: u64 = text.trim().split('=').nth(1)?.parse().ok()?;
    if hz == 0 { return None; }

    // hz is the core clock; MT/s = (hz / 1_000_000) * 8 for DDR
    let mhz = hz / 1_000_000;
    Some(mhz * 8)
}

/// Run `vcgencmd measure_volts sdram_c` and return voltage string.
fn read_vcgencmd_voltage() -> Option<String> {
    let out = Command::new("vcgencmd")
        .args(["measure_volts", "sdram_c"])
        .output()
        .ok()?;

    if !out.status.success() { return None; }

    // Output format: "volt=1.1000V"
    let text = String::from_utf8_lossy(&out.stdout);
    let volt = text.trim().split('=').nth(1)?.trim().to_string();
    if volt.is_empty() { None } else { Some(volt) }
}

/// Read DIMM temperatures from `/sys/class/hwmon`. Returns a [`TempReading`] per
/// sensor. DDR5 modules expose temperatures via the `spd5118` driver; DDR4 does
/// not, so this is commonly empty.
pub fn read_ram_temps() -> Vec<TempReading> {
    let mut results = Vec::new();
    let hwmon_base = "/sys/class/hwmon";

    let dirs = match fs::read_dir(hwmon_base) {
        Ok(d) => d,
        Err(_) => return results,
    };

    for entry in dirs.flatten() {
        let hwmon_path = entry.path();

        let dev_name = fs::read_to_string(hwmon_path.join("name"))
            .unwrap_or_default()
            .trim()
            .to_lowercase();

        // spd5118 = DDR5, dimmtemp = older DDR DIMM
        let is_ram_device = dev_name.contains("dimm")
            || dev_name.contains("spd")
            || dev_name.contains("mem");

        if !is_ram_device { continue; }

        let sensor_dirs = match fs::read_dir(&hwmon_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        for f in sensor_dirs.flatten() {
            let fname = f.file_name().to_string_lossy().to_string();
            if !fname.starts_with("temp") || !fname.ends_with("_input") { continue; }

            let prefix = fname.trim_end_matches("_input");

            let label = fs::read_to_string(hwmon_path.join(format!("{}_label", prefix)))
                .unwrap_or_else(|_| format!("{} {}", dev_name, prefix.trim_start_matches("temp")))
                .trim()
                .to_string();

            let raw: i64 = fs::read_to_string(f.path())
                .unwrap_or_default()
                .trim()
                .parse()
                .unwrap_or(0);

            if raw > 0 {
                results.push(TempReading { label, celsius: raw as f64 / 1000.0 });
            }
        }
    }
    results
}

/// Collect top N processes by RSS from /proc.
pub fn top_mem_consumers(n: usize) -> Vec<ProcessMem> {
    let proc = match fs::read_dir("/proc") {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let mut procs: Vec<ProcessMem> = proc
        .flatten()
        .filter_map(|entry| {
            let fname = entry.file_name().to_string_lossy().to_string();
            let pid: u32 = fname.parse().ok()?;

            let status = fs::read_to_string(entry.path().join("status")).ok()?;
            let mut name = String::new();
            let mut rss_kb: u64 = 0;

            for line in status.lines() {
                if let Some(rest) = line.strip_prefix("Name:") {
                    name = rest.trim().to_string();
                } else if let Some(rest) = line.strip_prefix("VmRSS:") {
                    rss_kb = rest.split_whitespace().next()
                        .unwrap_or("0").parse().unwrap_or(0);
                }
            }

            if rss_kb == 0 { return None; }
            Some(ProcessMem { pid, name, rss_kb })
        })
        .collect();

    procs.sort_by(|a, b| b.rss_kb.cmp(&a.rss_kb));
    procs.truncate(n);
    procs
}

// ─── Motherboard ──────────────────────────────────────────────────────────────


/// Run `dmidecode -t 2` (via `sudo -n`, falling back to a direct call) and parse
/// motherboard identification. Returns default (empty) info on failure.
pub fn read_mobo_info() -> MoboInfo {
    let output = Command::new("sudo")
        .args(["-n", "dmidecode", "-t", "2"])
        .output()
        .or_else(|_| Command::new("dmidecode").args(["-t", "2"]).output());

    let text = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return MoboInfo::default(),
    };

    parse_mobo_output(&text)
}

/// Parse `dmidecode -t 2` output text into a [`MoboInfo`].
pub fn parse_mobo_output(text: &str) -> MoboInfo {
    let mut info = MoboInfo::default();
    for line in text.lines() {
        let kv: Vec<&str> = line.splitn(2, ':').collect();
        if kv.len() != 2 { continue; }
        let (key, val) = (kv[0].trim(), kv[1].trim().to_string());
        match key {
            "Manufacturer" => info.manufacturer = val,
            "Product Name" => info.product = val,
            _ => {}
        }
    }
    info
}
