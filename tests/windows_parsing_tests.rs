use raminfo::parsers::windows::*;

// ─── parse_memory_status ─────────────────────────────────────────────────────

#[test]
fn parse_memory_status_basic() {
    let text = "\
TotalVisibleMemorySize=33356184
FreePhysicalMemory=12582912
";
    let s = parse_memory_status(text);
    assert_eq!(s.total_kb, 33356184);
    assert_eq!(s.free_kb, 12582912);
    assert_eq!(s.available_kb, 12582912); // mirrors free_kb on Windows
    assert_eq!(s.buffers_kb, 0);
    assert_eq!(s.cached_kb, 0);
    assert_eq!(s.swap_total_kb, 0); // swap comes from parse_pagefile, not here
    assert_eq!(s.swap_free_kb, 0);
}

#[test]
fn parse_memory_status_empty_input() {
    let s = parse_memory_status("");
    assert_eq!(s.total_kb, 0);
    assert_eq!(s.free_kb, 0);
    assert_eq!(s.available_kb, 0);
}

#[test]
fn parse_memory_status_garbage_values() {
    let text = "\
TotalVisibleMemorySize=
FreePhysicalMemory=not-a-number
Unrelated line without equals
";
    let s = parse_memory_status(text);
    assert_eq!(s.total_kb, 0);
    assert_eq!(s.free_kb, 0);
}

// ─── parse_pagefile ──────────────────────────────────────────────────────────

#[test]
fn parse_pagefile_single() {
    let text = "\
AllocatedBaseSize=16384
CurrentUsage=1024
---
";
    let (total, free) = parse_pagefile(text);
    assert_eq!(total, 16384 * 1024);
    assert_eq!(free, (16384 - 1024) * 1024);
}

#[test]
fn parse_pagefile_multiple_summed() {
    let text = "\
AllocatedBaseSize=8192
CurrentUsage=512
---
AllocatedBaseSize=4096
CurrentUsage=256
---
";
    let (total, free) = parse_pagefile(text);
    assert_eq!(total, (8192 + 4096) * 1024);
    assert_eq!(free, (8192 + 4096 - 512 - 256) * 1024);
}

#[test]
fn parse_pagefile_empty_input() {
    assert_eq!(parse_pagefile(""), (0, 0));
}

#[test]
fn parse_pagefile_garbage() {
    let text = "\
AllocatedBaseSize=banana
CurrentUsage=
random noise
";
    assert_eq!(parse_pagefile(text), (0, 0));
}

// ─── parse_physical_memory ───────────────────────────────────────────────────

const PHYSICAL_MEMORY_TWO_DIMMS: &str = "\
DeviceLocator=ChannelA-DIMM0
Capacity=17179869184
Speed=3200
ConfiguredClockSpeed=2933
Manufacturer=Samsung
PartNumber=M471A2K43DB1-CWE   \x20
SMBIOSMemoryType=26
ConfiguredVoltage=1200
---
DeviceLocator=Controller0-ChannelA
Capacity=8589934592
Speed=6400
ConfiguredClockSpeed=6400
Manufacturer=Micron Technology
PartNumber=MTC4C10163S1SC64
SMBIOSMemoryType=34
ConfiguredVoltage=1100
---
";

#[test]
fn parse_physical_memory_two_dimms() {
    let slots = parse_physical_memory(PHYSICAL_MEMORY_TWO_DIMMS);
    assert_eq!(slots.len(), 2);

    assert_eq!(slots[0].locator, "ChannelA-DIMM0");
    assert_eq!(slots[0].size_mb, 16384); // 17179869184 bytes = 16 GB
    assert_eq!(slots[0].speed_mhz, 3200);
    assert_eq!(slots[0].configured_speed, 2933);
    assert_eq!(slots[0].mem_type, "DDR4"); // SMBIOS type 26
    assert_eq!(slots[0].manufacturer, "Samsung");
    assert_eq!(slots[0].part_number, "M471A2K43DB1-CWE"); // trailing spaces trimmed
    assert_eq!(slots[0].voltage, "1.2 V"); // 1200 mV

    assert_eq!(slots[1].locator, "Controller0-ChannelA");
    assert_eq!(slots[1].size_mb, 8192);
    assert_eq!(slots[1].mem_type, "DDR5"); // SMBIOS type 34
    assert_eq!(slots[1].voltage, "1.1 V"); // 1100 mV
}

#[test]
fn parse_physical_memory_drops_zero_capacity() {
    let text = "\
DeviceLocator=DIMM_A1
Capacity=8589934592
Speed=2666
SMBIOSMemoryType=26
ConfiguredVoltage=1200
---
DeviceLocator=DIMM_A2
Capacity=0
Speed=0
SMBIOSMemoryType=26
ConfiguredVoltage=0
---
";
    let slots = parse_physical_memory(text);
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].locator, "DIMM_A1");
    assert_eq!(slots[0].size_mb, 8192);
}

#[test]
fn parse_physical_memory_unknown_type_and_no_voltage() {
    let text = "\
DeviceLocator=DIMM0
Capacity=4294967296
Speed=1600
SMBIOSMemoryType=99
ConfiguredVoltage=0
---
";
    let slots = parse_physical_memory(text);
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].mem_type, "RAM"); // unmapped SMBIOS code
    assert_eq!(slots[0].voltage, ""); // 0 mV → empty string
}

#[test]
fn parse_physical_memory_empty_input() {
    assert!(parse_physical_memory("").is_empty());
}

// ─── parse_memory_array ──────────────────────────────────────────────────────

#[test]
fn parse_memory_array_prefers_max_capacity_ex() {
    let text = "\
MemoryDevices=4
MaxCapacityEx=134217728
MaxCapacity=67108864
---
";
    let info = parse_memory_array(text);
    assert_eq!(info.total_slots, 4);
    assert_eq!(info.max_capacity_mb, 131072); // 134217728 KB = 128 GB, Ex wins
}

#[test]
fn parse_memory_array_falls_back_to_max_capacity() {
    let text = "\
MemoryDevices=2
MaxCapacityEx=
MaxCapacity=33554432
---
";
    let info = parse_memory_array(text);
    assert_eq!(info.total_slots, 2);
    assert_eq!(info.max_capacity_mb, 32768); // 33554432 KB = 32 GB
}

#[test]
fn parse_memory_array_empty_input() {
    let info = parse_memory_array("");
    assert_eq!(info.total_slots, 0);
    assert_eq!(info.max_capacity_mb, 0);
}

// ─── parse_baseboard ─────────────────────────────────────────────────────────

#[test]
fn parse_baseboard_standard() {
    let text = "\
Manufacturer=ASUSTeK COMPUTER INC.
Product=ROG STRIX B550-F GAMING
";
    let info = parse_baseboard(text);
    assert_eq!(info.manufacturer, "ASUSTeK COMPUTER INC.");
    assert_eq!(info.product, "ROG STRIX B550-F GAMING");
}

#[test]
fn parse_baseboard_empty_input() {
    let info = parse_baseboard("");
    assert!(info.manufacturer.is_empty());
    assert!(info.product.is_empty());
}

// ─── parse_process_list ──────────────────────────────────────────────────────

const PROCESS_LIST: &str = "\
Id=1234
Name=chrome
WS=524288000
---
Id=5678
Name=firefox
WS=1048576000
---
Id=42
Name=idleghost
WS=0
---
Id=910
Name=explorer
WS=104857600
---
";

#[test]
fn parse_process_list_sorted_desc() {
    let procs = parse_process_list(PROCESS_LIST, 10);
    assert_eq!(procs.len(), 3); // zero-WS entry dropped

    assert_eq!(procs[0].name, "firefox");
    assert_eq!(procs[0].pid, 5678);
    assert_eq!(procs[0].rss_kb, 1048576000 / 1024);

    assert_eq!(procs[1].name, "chrome");
    assert_eq!(procs[1].rss_kb, 524288000 / 1024);

    assert_eq!(procs[2].name, "explorer");
    assert_eq!(procs[2].rss_kb, 104857600 / 1024);
}

#[test]
fn parse_process_list_truncates() {
    let procs = parse_process_list(PROCESS_LIST, 2);
    assert_eq!(procs.len(), 2);
    assert_eq!(procs[0].name, "firefox");
    assert_eq!(procs[1].name, "chrome");
}

#[test]
fn parse_process_list_empty_input() {
    assert!(parse_process_list("", 10).is_empty());
}
