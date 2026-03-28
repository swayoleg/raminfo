use raminfo::format::*;

// ─── fmt_size ────────────────────────────────────────────────────────────────

#[test]
fn fmt_size_mb_below_1024() {
    assert_eq!(fmt_size(512), "512 MB");
    assert_eq!(fmt_size(1), "1 MB");
    assert_eq!(fmt_size(0), "0 MB");
    assert_eq!(fmt_size(1023), "1023 MB");
}

#[test]
fn fmt_size_gb_at_and_above_1024() {
    assert_eq!(fmt_size(1024), "1 GB");
    assert_eq!(fmt_size(2048), "2 GB");
    assert_eq!(fmt_size(16384), "16 GB");
    assert_eq!(fmt_size(32768), "32 GB");
}

// ─── fmt_kb ──────────────────────────────────────────────────────────────────

#[test]
fn fmt_kb_shows_mb_below_1gb() {
    assert_eq!(fmt_kb(1024), "1 MB");        // 1 MB
    assert_eq!(fmt_kb(512 * 1024), "512 MB"); // 512 MB
    assert_eq!(fmt_kb(1023 * 1024), "1023 MB");
}

#[test]
fn fmt_kb_shows_gb_at_and_above_1gb() {
    assert_eq!(fmt_kb(1024 * 1024), "1.0 GB");
    assert_eq!(fmt_kb(2 * 1024 * 1024), "2.0 GB");
    assert_eq!(fmt_kb(32 * 1024 * 1024), "32.0 GB");
}

#[test]
fn fmt_kb_fractional_gb() {
    // 1.5 GB = 1536 MB = 1572864 KB
    assert_eq!(fmt_kb(1572864), "1.5 GB");
}

// ─── bar ─────────────────────────────────────────────────────────────────────

#[test]
fn bar_zero_total_returns_dashes() {
    let result = bar(0, 0, 10);
    assert_eq!(result, "──────────");
}

#[test]
fn bar_zero_used() {
    let result = bar(0, 100, 10);
    // Should have 10 empty chars and 0 filled
    assert!(result.contains("░"));
    assert!(!result.contains("█"));
}

#[test]
fn bar_full() {
    let result = bar(100, 100, 10);
    // Should have filled chars (red at 100%)
    assert!(result.contains("█"));
    assert!(!result.contains("░"));
}

#[test]
fn bar_half_filled() {
    let result = bar(50, 100, 10);
    assert!(result.contains("█"));
    assert!(result.contains("░"));
}

// ─── temp_colored ────────────────────────────────────────────────────────────

#[test]
fn temp_colored_contains_value() {
    let cold = temp_colored(45.0);
    assert!(cold.to_string().contains("45.0°C"));

    let warm = temp_colored(75.5);
    assert!(warm.to_string().contains("75.5°C"));

    let hot = temp_colored(90.0);
    assert!(hot.to_string().contains("90.0°C"));
}
