use colored::*;
use crate::types::*;
use crate::format::*;

// ─── Renderers ────────────────────────────────────────────────────────────────

pub fn render_dimm_table(slots: &[DimmSlot]) {
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

pub fn render_upgrade_potential(slots: &[DimmSlot], array: &MemArrayInfo) {
    // Only render if we have meaningful data
    if array.total_slots == 0 && array.max_capacity_mb == 0 { return; }

    let box_w = 56usize;
    let bdr = |s: &str| s.bright_black().to_string();

    let row = |label: &str, val: &str| -> String {
        let content = format!("  {:<16}{}", label, val);
        let pad = box_w.saturating_sub(content.len());
        format!("{}{}{}{}", bdr("│"), content, " ".repeat(pad), bdr("│"))
    };

    let installed_mb: u64 = slots.iter().map(|s| s.size_mb).sum();
    let used_slots = slots.len() as u32;
    let free_slots = array.total_slots.saturating_sub(used_slots);
    let headroom_mb = array.max_capacity_mb.saturating_sub(installed_mb);
    let max_freq = slots.iter().map(|s| s.speed_mhz).max().unwrap_or(0);

    // Slots bar: filled ● empty ○
    let slots_bar = format!("{}{}",
                            "●".green().to_string().repeat(used_slots as usize),
                            "○".dimmed().to_string().repeat(free_slots as usize),
    );

    println!();
    println!("  {}", "Upgrade Potential".bold().bright_white());
    println!("{}", bdr(&format!("╭{}╮", "─".repeat(box_w))));

    // Slots row with visual bar
    let slots_label = "Slots";
    let slots_val   = format!("{} used / {} total  {}", used_slots, array.total_slots, slots_bar);
    let slots_content = format!("  {:<16}{}", slots_label, slots_val);
    let slots_visible = 2 + 16 + format!("{} used / {} total  ", used_slots, array.total_slots).len()
        + used_slots as usize + free_slots as usize; // ● and ○ are single visible chars
    let slots_pad = box_w.saturating_sub(slots_visible);
    println!("{}{}{}{}", bdr("│"), slots_content, " ".repeat(slots_pad), bdr("│"));

    println!("{}", row("Installed", &fmt_size(installed_mb)));

    if array.max_capacity_mb > 0 {
        println!("{}", row("Maximum", &fmt_size(array.max_capacity_mb)));

        if headroom_mb > 0 {
            let headroom_str = format!("{} available", fmt_size(headroom_mb));
            let content = format!("  {:<16}{}", "Headroom", headroom_str.green().to_string());
            let visible = 2 + 16 + headroom_str.len();
            let pad = box_w.saturating_sub(visible);
            println!("{}{}{}{}", bdr("│"), content, " ".repeat(pad), bdr("│"));
        } else {
            let content = format!("  {:<16}{}", "Headroom", "fully populated".dimmed().to_string());
            let visible = 2 + 16 + "fully populated".len();
            let pad = box_w.saturating_sub(visible);
            println!("{}{}{}{}", bdr("│"), content, " ".repeat(pad), bdr("│"));
        }
    }

    if max_freq > 0 {
        println!("{}", row("Max Freq", &format!("{} MT/s", max_freq)));
    }

    println!("{}", bdr(&format!("╰{}╯", "─".repeat(box_w))));
}

pub fn render_pi_board(info: &PiBoardInfo) {
    let box_w = 56usize;
    let bdr = |s: &str| s.bright_black().to_string();

    let row = |label: &str, val: &str| -> String {
        let content = format!("  {:<14}{}", label, val);
        let pad = box_w.saturating_sub(content.len());
        format!("{}{}{}{}", bdr("│"), content, " ".repeat(pad), bdr("│"))
    };

    println!();
    println!("  {}", "Board Memory".bold().bright_white());
    println!("{}", bdr(&format!("╭{}╮", "─".repeat(box_w))));

    let model_content = format!("  {}", info.model.bold());
    let model_pad = box_w.saturating_sub(2 + info.model.len());
    println!("{}{}{}{}", bdr("│"), model_content, " ".repeat(model_pad), bdr("│"));

    println!("{}", bdr(&format!("├{}┤", "─".repeat(box_w))));

    let type_val = format!("{}  {}", info.mem_type.bold().cyan(), "(soldered, no slots)".dimmed());
    let type_visible = info.mem_type.len() + 2 + "(soldered, no slots)".len();
    let type_content = format!("  {:<14}{}", "Type", type_val);
    let type_pad = box_w.saturating_sub(2 + 14 + type_visible);
    println!("{}{}{}{}", bdr("│"), type_content, " ".repeat(type_pad), bdr("│"));

    match info.freq_mhz {
        Some(mt) if mt > 0 => {
            println!("{}", row("Frequency", &format!("{} MT/s", mt)));
        }
        _ => {
            let default = if info.model.contains("Pi 5") { "4267 MT/s (default)" }
            else if info.model.contains("Pi 4") { "3200 MT/s (default)" }
            else { "—" };
            println!("{}", row("Frequency", default));
        }
    }

    if let Some(ref v) = info.voltage {
        println!("{}", row("Voltage", v));
    }

    println!("{}", bdr(&format!("╰{}╯", "─".repeat(box_w))));
}

pub fn render_mem_stats(s: &MemStats, dimms: &[DimmSlot]) {
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

pub fn render_temps(temps: &[(String, f64)]) {
    if temps.is_empty() { return; }

    let box_w = 70usize;
    let bdr = |s: &str| s.bright_black().to_string();

    println!();
    println!("  {}", "RAM Temperatures".bold().bright_white());
    println!("{}", bdr(&format!("╭{}╮", "─".repeat(box_w))));

    for (label, celsius) in temps {
        let temp_str = temp_colored(*celsius).to_string();
        let label_col = format!("  {:<30}", label);
        let visible_len = label_col.len() + 6;
        let pad = box_w.saturating_sub(visible_len);
        println!("{}{}{}{}{}", bdr("│"), label_col, temp_str, " ".repeat(pad), bdr("│"));
    }

    println!("{}", bdr(&format!("╰{}╯", "─".repeat(box_w))));
}

pub fn render_top_consumers(procs: &[ProcessMem], total_kb: u64) {
    if procs.is_empty() { return; }

    let bar_w  = 24usize;
    let box_w  = 3 + 7 + 20 + 9 + bar_w + 5 + 13;

    let bdr = |s: &str| s.bright_black().to_string();
    let top = bdr(&format!("╭{}╮", "─".repeat(box_w)));
    let mid = bdr(&format!("├{}┤", "─".repeat(box_w)));
    let bot = bdr(&format!("╰{}╯", "─".repeat(box_w)));

    println!();
    println!("  {}", "Top Memory Consumers".bold().bright_white());
    println!("{}", top);

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

        let content = format!("  {}  {}  {}  {}   {}  {}",
                              rank.dimmed(), pid.dimmed(), name.bold(), rss.bright_white(), b, pct_s);
        let visible_len = 2 + 3 + 2 + 7 + 2 + 20 + 2 + 9 + 3 + bar_w + 2 + 4;
        let pad = box_w.saturating_sub(visible_len);
        println!("{}{}{}{}", bdr("│"), content, " ".repeat(pad), bdr("│"));
    }

    println!("{}", bot);
}

// ─── Motherboard ──────────────────────────────────────────────────────────────

pub fn render_mobo_info(info: &MoboInfo) {
    if info.manufacturer.is_empty() && info.product.is_empty() { return; }

    let box_w = 70usize;
    let bdr = |s: &str| s.bright_black().to_string();

    println!();
    println!("  {}", "Motherboard".bold().bright_white());
    println!("{}", bdr(&format!("╭{}╮", "─".repeat(box_w))));

    let mfr = if info.manufacturer.is_empty() { "—" } else { &info.manufacturer };
    let prod = if info.product.is_empty() { "—" } else { &info.product };

    let mfr_row = format!("  {:<20}{}", "Manufacturer", mfr.bold().bright_white());
    let mfr_visible = 2 + 20 + mfr.len();
    let mfr_pad = box_w.saturating_sub(mfr_visible);
    println!("{}{}{}{}", bdr("│"), mfr_row, " ".repeat(mfr_pad), bdr("│"));

    let prod_row = format!("  {:<20}{}", "Product", prod.bold().bright_white());
    let prod_visible = 2 + 20 + prod.len();
    let prod_pad = box_w.saturating_sub(prod_visible);
    println!("{}{}{}{}", bdr("│"), prod_row, " ".repeat(prod_pad), bdr("│"));

    println!("{}", bdr(&format!("╰{}╯", "─".repeat(box_w))));
}