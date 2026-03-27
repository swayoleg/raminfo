use colored::*;
use std::fs;
use std::process::Command;

// ─── Structs ──────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct DimmSlot {
    locator: String,
    size_mb: u64,
    speed_mhz: u64,
    mem_type: String,
    manufacturer: String,
    part_number: String,
    configured_speed: u64,
    voltage: String,
}

#[derive(Debug, Default)]
struct MemStats {
    total_kb: u64,
    free_kb: u64,
    available_kb: u64,
    buffers_kb: u64,
    cached_kb: u64,
    swap_total_kb: u64,
    swap_free_kb: u64,
}

#[derive(Debug)]
struct ProcessMem {
    pid: u32,
    name: String,
    rss_kb: u64,
}

// ─── Parsing ──────────────────────────────────────────────────────────────────

fn parse_proc_meminfo() -> MemStats {
    let mut s = MemStats::default();
    let content = fs::read_to_string("/proc/meminfo").unwrap_or_default();
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

fn parse_dmidecode() -> Vec<DimmSlot> {
    let output = Command::new("sudo")
        .args(["-n", "dmidecode", "-t", "17"])
        .output()
        .or_else(|_| Command::new("dmidecode").args(["-t", "17"]).output());

    let text = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return vec![],
    };

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

/// Read DIMM temperatures from hwmon. Returns a Vec of (label, celsius).
/// Checks every hwmon device; picks sensors whose name contains "dimm" or "mem".
fn read_ram_temps() -> Vec<(String, f64)> {
    let mut results = Vec::new();
    let hwmon_base = "/sys/class/hwmon";

    let dirs = match fs::read_dir(hwmon_base) {
        Ok(d) => d,
        Err(_) => return results,
    };

    for entry in dirs.flatten() {
        let hwmon_path = entry.path();

        // Read the device name (e.g. "coretemp", "dimmtemp", "spd5118")
        let dev_name = fs::read_to_string(hwmon_path.join("name"))
            .unwrap_or_default()
            .trim()
            .to_lowercase();

        // spd5118 = DDR5 SPD hub (temp sensor), dimmtemp = older DDR DIMM
        let is_ram_device = dev_name.contains("dimm")
            || dev_name.contains("spd")
            || dev_name.contains("mem");

        if !is_ram_device { continue; }

        // Enumerate temp*_input files
        let sensor_dirs = match fs::read_dir(&hwmon_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        for f in sensor_dirs.flatten() {
            let fname = f.file_name().to_string_lossy().to_string();
            if !fname.starts_with("temp") || !fname.ends_with("_input") { continue; }

            let prefix = fname.trim_end_matches("_input");

            // Read label, fallback to hwmon device name + index
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
                results.push((label, raw as f64 / 1000.0));
            }
        }
    }
    results
}

/// Collect top N processes by RSS from /proc.
fn top_mem_consumers(n: usize) -> Vec<ProcessMem> {
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

// ─── Formatting ───────────────────────────────────────────────────────────────

fn fmt_size(mb: u64) -> String {
    if mb >= 1024 { format!("{:.0} GB", mb as f64 / 1024.0) }
    else { format!("{} MB", mb) }
}

fn fmt_kb(kb: u64) -> String {
    let mb = kb / 1024;
    let gb = mb as f64 / 1024.0;
    if gb >= 1.0 { format!("{:.1} GB", gb) } else { format!("{} MB", mb) }
}

fn bar(used: u64, total: u64, width: usize) -> String {
    if total == 0 { return "─".repeat(width); }
    let pct = used as f64 / total as f64;
    let filled = (pct * width as f64).round() as usize;
    let empty  = width.saturating_sub(filled);
    let filled_s = "█".repeat(filled);
    let empty_s  = "░".repeat(empty);
    let colored_fill = if pct > 0.85 { filled_s.red() }
        else if pct > 0.6 { filled_s.yellow() }
        else { filled_s.green() };
    format!("{}{}", colored_fill, empty_s.dimmed())
}

fn temp_colored(c: f64) -> ColoredString {
    let s = format!("{:.1}°C", c);
    if c >= 85.0 { s.red().bold() }
    else if c >= 70.0 { s.yellow() }
    else { s.green() }
}

// ─── Renderers ────────────────────────────────────────────────────────────────

fn render_dimm_table(slots: &[DimmSlot]) {
    let w = [16usize, 24, 14, 7, 6, 9, 9, 7];
    let sep = |l: &str, f: &str, m: &str, r: &str| -> String {
        let segs: Vec<String> = w.iter().map(|n| f.repeat(n + 2)).collect();
        format!("{}{}{}", l, segs.join(m), r)
    };
    let cell = |s: &str, width: usize, right: bool| -> String {
        if right { format!(" {:>w$} ", s, w = width) }
        else      { format!(" {:<w$} ", s, w = width) }
    };
    let row = |vals: &[(&str, bool)]| -> String {
        let mut out = "│".bright_black().to_string();
        for (i, (v, right)) in vals.iter().enumerate() {
            let truncated = if v.chars().count() > w[i] {
                format!("{}…", v.chars().take(w[i].saturating_sub(1)).collect::<String>())
            } else { v.to_string() };
            out.push_str(&cell(&truncated, w[i], *right));
            out.push_str(&"│".bright_black().to_string());
        }
        out
    };

    println!();
    println!("  {}", "DIMM Slots".bold().bright_white());
    println!("{}", sep("╭","─","┬","╮").bright_black());
    println!("{}", row(&[
        ("Slot",false),("Part Number",false),("Vendor",false),
        ("Size",true),("Type",false),("Max MT/s",true),("Cfg MT/s",true),("Voltage",true),
    ]));
    println!("{}", sep("├","─","┼","┤").bright_black());

    for s in slots {
        let part  = if s.part_number.is_empty() || s.part_number == "Unknown" { "—".into() } else { s.part_number.trim().to_string() };
        let mfr   = if s.manufacturer.is_empty() || s.manufacturer == "Unknown" { "—".into() } else { s.manufacturer.clone() };
        let speed = if s.speed_mhz        > 0 { s.speed_mhz.to_string()        } else { "—".into() };
        let cfgsp = if s.configured_speed > 0 { s.configured_speed.to_string() } else { "—".into() };
        let volt  = if s.voltage.is_empty() || s.voltage == "Unknown" { "—".into() } else { s.voltage.clone() };
        println!("{}", row(&[
            (&s.locator,false),(&part,false),(&mfr,false),
            (&fmt_size(s.size_mb),true),(&s.mem_type,false),
            (&speed,true),(&cfgsp,true),(&volt,true),
        ]));
    }
    println!("{}", sep("╰","─","┴","╯").bright_black());
}

fn render_mem_stats(s: &MemStats, dimms: &[DimmSlot]) {
    let bar_w   = 36usize;
    let box_w   = bar_w + 34;
    let used_kb = s.total_kb.saturating_sub(s.available_kb);
    let swap_used = s.swap_total_kb.saturating_sub(s.swap_free_kb);

    let bdr = |s: &str| s.bright_black().to_string();
    let top = bdr(&format!("╭{}╮", "─".repeat(box_w)));
    let div = bdr(&format!("├{}┤", "─".repeat(box_w)));
    let bot = bdr(&format!("╰{}╯", "─".repeat(box_w)));

    let stat_row = |label: &str, used: u64, total: u64, show_bar: bool| -> String {
        let val_str = fmt_kb(used);
        let bar_str = if show_bar { bar(used, total, bar_w) } else { " ".repeat(bar_w) };
        let pct_str = if show_bar && total > 0 {
            format!(" {:>3}%", used * 100 / total)
        } else { "     ".into() };
        let content = format!("  {:<14}{:>9}   {}{}", label, val_str, bar_str, pct_str);
        let visible_len = 2 + 14 + 9 + 3 + bar_w + pct_str.len();
        let pad = box_w.saturating_sub(visible_len);
        format!("{}{}{}{}", bdr("│"), content, " ".repeat(pad), bdr("│"))
    };

    let plain_row = |label: &str, val: &str| -> String {
        let content = format!("  {:<14}{:>9}", label, val);
        let pad = box_w.saturating_sub(content.len());
        format!("{}{}{}{}", bdr("│"), content, " ".repeat(pad), bdr("│"))
    };

    let dimm_note = if !dimms.is_empty() {
        let total_mb: u64 = dimms.iter().map(|d| d.size_mb).sum();
        format!("  {} installed across {} DIMM slot{}",
            fmt_size(total_mb), dimms.len(),
            if dimms.len() == 1 { "" } else { "s" })
    } else { String::new() };

    println!();
    println!("  {}", "Memory Usage".bold().bright_white());
    println!("{}", top);
    let title = format!("  RAM{}", dimm_note);
    let total_str = fmt_kb(s.total_kb);
    let gap = box_w.saturating_sub(title.len() + total_str.len() + 2);
    println!("{}{}{}{}{}",
        bdr("│"), title.bold(), " ".repeat(gap),
        total_str.bold().bright_white(), bdr("  │"));
    println!("{}", div);
    println!("{}", stat_row("Used",      used_kb,        s.total_kb,      true));
    println!("{}", stat_row("Available", s.available_kb, s.total_kb,      false));
    println!("{}", plain_row("Free",     &fmt_kb(s.free_kb)));
    println!("{}", plain_row("Buffers",  &fmt_kb(s.buffers_kb)));
    println!("{}", plain_row("Cached",   &fmt_kb(s.cached_kb)));
    if s.swap_total_kb > 0 {
        println!("{}", div);
        println!("{}", stat_row("Swap Used",  swap_used,       s.swap_total_kb, true));
        println!("{}", plain_row("Swap Total", &fmt_kb(s.swap_total_kb)));
    }
    println!("{}", bot);
}

fn render_temps(temps: &[(String, f64)]) {
    if temps.is_empty() { return; }

    let box_w = 70usize;
    let bdr = |s: &str| s.bright_black().to_string();

    println!();
    println!("  {}", "RAM Temperatures".bold().bright_white());
    println!("{}", bdr(&format!("╭{}╮", "─".repeat(box_w))));

    for (label, celsius) in temps {
        let temp_str = temp_colored(*celsius).to_string();
        // visible temp string length is fixed: "XX.X°C" = 6 chars
        let label_col = format!("  {:<30}", label);
        let visible_len = label_col.len() + 6;
        let pad = box_w.saturating_sub(visible_len);
        println!("{}{}{}{}{}", bdr("│"), label_col, temp_str, " ".repeat(pad), bdr("│"));
    }

    println!("{}", bdr(&format!("╰{}╯", "─".repeat(box_w))));
}

fn render_top_consumers(procs: &[ProcessMem], total_kb: u64) {
    if procs.is_empty() { return; }

    let bar_w  = 24usize;
    // cols: rank(3) pid(7) name(20) rss(9) bar(bar_w) pct(5)
    let box_w  = 3 + 7 + 20 + 9 + bar_w + 5 + 13; // paddings

    let bdr = |s: &str| s.bright_black().to_string();
    let top = bdr(&format!("╭{}╮", "─".repeat(box_w)));
    let mid = bdr(&format!("├{}┤", "─".repeat(box_w)));
    let bot = bdr(&format!("╰{}╯", "─".repeat(box_w)));

    println!();
    println!("  {}", "Top Memory Consumers".bold().bright_white());
    println!("{}", top);

    // Header
    let header = format!("  {:<3}  {:<7}  {:<20}  {:>9}   {:<bar_w$}  {}",
        "#", "PID", "Process", "RSS", "", "Share", bar_w = bar_w);
    let pad = box_w.saturating_sub(header.len());
    println!("{}{}{}{}", bdr("│"), header, " ".repeat(pad), bdr("│"));
    println!("{}", mid);

    for (i, p) in procs.iter().enumerate() {
        let rank  = format!("{:>2}.", i + 1);
        let pid   = format!("{:<7}", p.pid);
        let name  = if p.name.chars().count() > 20 {
            format!("{}…", p.name.chars().take(19).collect::<String>())
        } else { format!("{:<20}", p.name) };
        let rss   = format!("{:>9}", fmt_kb(p.rss_kb));
        let pct   = if total_kb > 0 { p.rss_kb * 100 / total_kb } else { 0 };
        let pct_s = format!("{:>3}%", pct);
        let b     = bar(p.rss_kb, total_kb, bar_w);

        // visible content length (bar has ANSI but we account for it manually)
        let content = format!("  {}  {}  {}  {}   {}  {}",
            rank.dimmed(), pid.dimmed(), name.bold(), rss.bright_white(), b, pct_s);
        let visible_len = 2 + 3 + 2 + 7 + 2 + 20 + 2 + 9 + 3 + bar_w + 2 + 4;
        let pad = box_w.saturating_sub(visible_len);
        println!("{}{}{}{}", bdr("│"), content, " ".repeat(pad), bdr("│"));
    }

    println!("{}", bot);
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let stats  = parse_proc_meminfo();
    let dimms  = parse_dmidecode();
    let temps  = read_ram_temps();
    let top10  = top_mem_consumers(10);

    if dimms.is_empty() {
        println!();
        println!("  {} {}", "⚠".yellow(),
            "No DIMM hardware data — run with sudo for slot/model details".dimmed());
    } else {
        render_dimm_table(&dimms);
    }

    render_mem_stats(&stats, &dimms);
    render_temps(&temps);
    render_top_consumers(&top10, stats.total_kb);
    println!();
}
