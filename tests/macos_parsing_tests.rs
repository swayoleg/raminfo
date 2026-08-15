use raminfo::parsers::macos::*;

// ─── parse_vm_stat_output ────────────────────────────────────────────────────

const VM_STAT_16K: &str = "\
Mach Virtual Memory Statistics: (page size of 16384 bytes)
Pages free:                               78244.
Pages active:                            434577.
Pages inactive:                          421030.
Pages speculative:                         5384.
Pages throttled:                              0.
Pages wired down:                        135676.
Pages purgeable:                           1234.
\"Translation faults\":                 123456789.
Pages copy-on-write:                    1234567.
Pages zero filled:                     12345678.
Pages reactivated:                       123456.
Pages purged:                             12345.
File-backed pages:                       210000.
Anonymous pages:                         650991.
Pages stored in compressor:              300000.
Pages occupied by compressor:             90000.
Decompressions:                         1000000.
Compressions:                           2000000.
Pageins:                                3000000.
Pageouts:                                 40000.
Swapins:                                  50000.
Swapouts:                                 60000.
";

#[test]
fn parse_vm_stat_16k_pages() {
    let s = parse_vm_stat_output(VM_STAT_16K, 16777216);
    // Page size 16384 bytes → 16 kB per page.
    assert_eq!(s.total_kb, 16777216); // passed through
    assert_eq!(s.free_kb, 78244 * 16);
    assert_eq!(s.available_kb, (78244 + 421030 + 5384) * 16);
    assert_eq!(s.cached_kb, 210000 * 16);
    assert_eq!(s.buffers_kb, 0);
    // Swap is filled separately from `sysctl vm.swapusage`.
    assert_eq!(s.swap_total_kb, 0);
    assert_eq!(s.swap_free_kb, 0);
}

#[test]
fn parse_vm_stat_default_page_size() {
    // No header line → default 4096-byte pages (4 kB per page).
    let content = "\
Pages free:                              100000.
Pages active:                            200000.
Pages inactive:                           50000.
Pages speculative:                        10000.
";
    let s = parse_vm_stat_output(content, 8388608);
    assert_eq!(s.total_kb, 8388608);
    assert_eq!(s.free_kb, 100000 * 4);
    assert_eq!(s.available_kb, (100000 + 50000 + 10000) * 4);
    assert_eq!(s.cached_kb, 0); // "File-backed pages" absent
}

#[test]
fn parse_vm_stat_empty_input() {
    let s = parse_vm_stat_output("", 4194304);
    assert_eq!(s.total_kb, 4194304); // total still passed through
    assert_eq!(s.free_kb, 0);
    assert_eq!(s.available_kb, 0);
    assert_eq!(s.cached_kb, 0);
}

// ─── parse_swap_usage ────────────────────────────────────────────────────────

#[test]
fn parse_swap_usage_megabytes() {
    let text = "vm.swapusage: total = 2048.00M  used = 1234.56M  free = 813.44M";
    let (total, free) = parse_swap_usage(text);
    assert_eq!(total, 2048 * 1024); // 2097152
    assert_eq!(free, (813.44_f64 * 1024.0) as u64); // 832962
}

#[test]
fn parse_swap_usage_gigabytes() {
    let text = "vm.swapusage: total = 1.50G  used = 0.75G  free = 0.75G";
    let (total, free) = parse_swap_usage(text);
    assert_eq!(total, 1572864); // 1.5 * 1024 * 1024
    assert_eq!(free, 786432);
}

#[test]
fn parse_swap_usage_kilobytes() {
    let text = "vm.swapusage: total = 512.00K  used = 0.00K  free = 512.00K";
    let (total, free) = parse_swap_usage(text);
    assert_eq!(total, 512);
    assert_eq!(free, 512);
}

#[test]
fn parse_swap_usage_garbage() {
    assert_eq!(parse_swap_usage("not swap output at all"), (0, 0));
    assert_eq!(parse_swap_usage("total = banana free = apple"), (0, 0));
}

#[test]
fn parse_swap_usage_empty_input() {
    assert_eq!(parse_swap_usage(""), (0, 0));
}

// ─── parse_ps_output ─────────────────────────────────────────────────────────

const PS_OUTPUT: &str = "\
  512 123456 /usr/sbin/foo
    1  45000 /sbin/launchd
  999      0 /usr/bin/zero-rss
 2048 987654 /Applications/Google Chrome.app/Contents/MacOS/Google Chrome
  300  32000 kernel_task
";

#[test]
fn parse_ps_sorted_by_rss_desc() {
    let procs = parse_ps_output(PS_OUTPUT, 10);
    assert_eq!(procs.len(), 4); // zero-RSS entry dropped

    assert_eq!(procs[0].pid, 2048);
    assert_eq!(procs[0].rss_kb, 987654);
    assert_eq!(procs[0].name, "Google Chrome"); // basename, spaces preserved

    assert_eq!(procs[1].pid, 512);
    assert_eq!(procs[1].name, "foo");
    assert_eq!(procs[1].rss_kb, 123456);
}

#[test]
fn parse_ps_truncates_to_n() {
    let procs = parse_ps_output(PS_OUTPUT, 2);
    assert_eq!(procs.len(), 2);
    assert_eq!(procs[0].rss_kb, 987654);
    assert_eq!(procs[1].rss_kb, 123456);
}

#[test]
fn parse_ps_name_without_path() {
    let procs = parse_ps_output("  300  32000 kernel_task\n", 10);
    assert_eq!(procs.len(), 1);
    assert_eq!(procs[0].name, "kernel_task");
}

#[test]
fn parse_ps_empty_input() {
    assert!(parse_ps_output("", 10).is_empty());
}

#[test]
fn parse_ps_malformed_lines_skipped() {
    let text = "garbage\n  12 notanumber /bin/x\nabc 123 /bin/y\n";
    assert!(parse_ps_output(text, 10).is_empty());
}

// ─── parse_system_profiler_memory ────────────────────────────────────────────

const PROFILER_INTEL: &str = "\
Memory:

    Memory Slots:

      ECC: Disabled
      Upgradeable Memory: Yes

        BANK 0/DIMM0:

          Size: 16 GB
          Type: DDR4
          Speed: 2667 MHz
          Status: OK
          Manufacturer: 0x802C
          Part Number: 0x3141544631473634485A2D324733423220
          Serial Number: 0x00000000

        BANK 1/DIMM0:

          Size: 16 GB
          Type: DDR4
          Speed: 2667 MHz
          Status: OK
          Manufacturer: 0x859B
          Part Number: CT16G4SFD8266.M16FE
          Serial Number: 0x00000001

        BANK 2/DIMM0:

          Size: Empty
";

#[test]
fn parse_profiler_intel_multi_bank() {
    let (slots, array) = parse_system_profiler_memory(PROFILER_INTEL);
    assert_eq!(slots.len(), 2); // empty bank skipped

    assert_eq!(slots[0].locator, "BANK 0/DIMM0");
    assert_eq!(slots[0].size_mb, 16384); // 16 GB
    assert_eq!(slots[0].mem_type, "DDR4");
    assert_eq!(slots[0].speed_mhz, 2667);
    assert_eq!(slots[0].manufacturer, "0x802C"); // hex codes kept as-is
    assert_eq!(slots[0].part_number, "0x3141544631473634485A2D324733423220");
    assert_eq!(slots[0].configured_speed, 0);
    assert!(slots[0].voltage.is_empty());

    assert_eq!(slots[1].locator, "BANK 1/DIMM0");
    assert_eq!(slots[1].manufacturer, "0x859B");

    assert_eq!(array.total_slots, 3); // includes the empty bank
    assert_eq!(array.max_capacity_mb, 0);
}

const PROFILER_APPLE_SILICON: &str = "\
Memory:

      Memory: 16 GB
      Type: LPDDR5
      Manufacturer: Hynix
";

#[test]
fn parse_profiler_apple_silicon() {
    let (slots, array) = parse_system_profiler_memory(PROFILER_APPLE_SILICON);
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].locator, "Soldered");
    assert_eq!(slots[0].size_mb, 16384);
    assert_eq!(slots[0].mem_type, "LPDDR5");
    assert_eq!(slots[0].manufacturer, "Hynix");
    assert_eq!(slots[0].speed_mhz, 0);
    assert!(slots[0].part_number.is_empty());

    assert_eq!(array.total_slots, 0); // no physical slots on Apple Silicon
    assert_eq!(array.max_capacity_mb, 0);
}

#[test]
fn parse_profiler_size_in_mb() {
    let text = "\
Memory:

        BANK 0/DIMM0:

          Size: 512 MB
          Type: DDR2
";
    let (slots, array) = parse_system_profiler_memory(text);
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].size_mb, 512);
    assert_eq!(array.total_slots, 1);
}

#[test]
fn parse_profiler_empty_input() {
    let (slots, array) = parse_system_profiler_memory("");
    assert!(slots.is_empty());
    assert_eq!(array.total_slots, 0);
    assert_eq!(array.max_capacity_mb, 0);
}
