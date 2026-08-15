use colored::*;

// ─── Formatting helpers ───────────────────────────────────────────────────────

/// Format a size in megabytes as `"N GB"` (>= 1024 MB) or `"N MB"`.
pub fn fmt_size(mb: u64) -> String {
    if mb >= 1024 { format!("{:.0} GB", mb as f64 / 1024.0) }
    else { format!("{} MB", mb) }
}

/// Format a size in kilobytes as `"N.N GB"` (>= 1 GB) or `"N MB"`.
pub fn fmt_kb(kb: u64) -> String {
    let mb = kb / 1024;
    let gb = mb as f64 / 1024.0;
    if gb >= 1.0 { format!("{:.1} GB", gb) } else { format!("{} MB", mb) }
}

/// Build a colored `width`-character usage bar (green/yellow/red by fill ratio).
pub fn bar(used: u64, total: u64, width: usize) -> String {
    if total == 0 { return "─".repeat(width); }
    let pct = used as f64 / total as f64;
    // Clamp so used > total can never draw past the requested width.
    let filled = ((pct * width as f64).round() as usize).min(width);
    let empty  = width.saturating_sub(filled);
    let filled_s = "█".repeat(filled);
    let empty_s  = "░".repeat(empty);
    let colored_fill = if pct > 0.85 { filled_s.red() }
    else if pct > 0.6 { filled_s.yellow() }
    else { filled_s.green() };
    format!("{}{}", colored_fill, empty_s.dimmed())
}

/// Format a Celsius temperature, colored by severity (green/yellow/red).
pub fn temp_colored(c: f64) -> ColoredString {
    let s = format!("{:.1}°C", c);
    if c >= 85.0 { s.red().bold() }
    else if c >= 70.0 { s.yellow() }
    else { s.green() }
}