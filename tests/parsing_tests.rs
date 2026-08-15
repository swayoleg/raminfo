use raminfo::parsers::*;

// ─── parse_meminfo_content ───────────────────────────────────────────────────

#[test]
fn parse_meminfo_all_fields() {
    let content = "\
MemTotal:       32845696 kB
MemFree:         3012345 kB
MemAvailable:   13631488 kB
Buffers:          595968 kB
Cached:          8003584 kB
SwapCached:        12345 kB
Active:         10000000 kB
Inactive:        5000000 kB
SwapTotal:      16777216 kB
SwapFree:       13777216 kB
Dirty:              1234 kB
";
    let s = parse_meminfo_content(content);
    assert_eq!(s.total_kb, 32845696);
    assert_eq!(s.free_kb, 3012345);
    assert_eq!(s.available_kb, 13631488);
    assert_eq!(s.buffers_kb, 595968);
    assert_eq!(s.cached_kb, 8003584);
    assert_eq!(s.swap_total_kb, 16777216);
    assert_eq!(s.swap_free_kb, 13777216);
}

#[test]
fn parse_meminfo_empty_input() {
    let s = parse_meminfo_content("");
    assert_eq!(s.total_kb, 0);
    assert_eq!(s.free_kb, 0);
    assert_eq!(s.available_kb, 0);
}

#[test]
fn parse_meminfo_partial() {
    let content = "MemTotal:       8167848 kB\nMemFree:         4000000 kB\n";
    let s = parse_meminfo_content(content);
    assert_eq!(s.total_kb, 8167848);
    assert_eq!(s.free_kb, 4000000);
    assert_eq!(s.available_kb, 0); // missing → default
}

// ─── parse_dmidecode_output ──────────────────────────────────────────────────

const DMIDECODE_TWO_DIMMS: &str = "\
# dmidecode 3.5
Getting SMBIOS data from sysfs.
SMBIOS 3.1.1 present.

Handle 0x0036, DMI type 17, 84 bytes
Memory Device
\tTotal Width: 64 bits
\tData Width: 64 bits
\tSize: 16 GB
\tForm Factor: SODIMM
\tSet: None
\tLocator: ChannelA-DIMM1
\tBank Locator: BANK 0
\tType: DDR4
\tType Detail: Synchronous Unbuffered (Unregistered)
\tSpeed: 3200 MT/s
\tManufacturer: 0B94
\tSerial Number: 12345678
\tAsset Tag: None
\tPart Number: EAGET-16GB-3200
\tRank: 2
\tConfigured Memory Speed: 3200 MT/s
\tMinimum Voltage: 1.2 V
\tMaximum Voltage: 1.2 V
\tConfigured Voltage: 1.2 V
\tMemory Technology: DRAM

Handle 0x0037, DMI type 17, 84 bytes
Memory Device
\tTotal Width: 64 bits
\tData Width: 64 bits
\tSize: 16 GB
\tForm Factor: SODIMM
\tSet: None
\tLocator: ChannelB-DIMM1
\tBank Locator: BANK 2
\tType: DDR4
\tType Detail: Synchronous Unbuffered (Unregistered)
\tSpeed: 3200 MT/s
\tManufacturer: 0B94
\tSerial Number: 87654321
\tAsset Tag: None
\tPart Number: EAGET-16GB-3200
\tRank: 2
\tConfigured Memory Speed: 3200 MT/s
\tMinimum Voltage: 1.2 V
\tMaximum Voltage: 1.2 V
\tConfigured Voltage: 1.2 V
\tMemory Technology: DRAM
";

#[test]
fn parse_dmidecode_two_dimms() {
    let slots = parse_dmidecode_output(DMIDECODE_TWO_DIMMS);
    assert_eq!(slots.len(), 2);

    assert_eq!(slots[0].locator, "ChannelA-DIMM1");
    assert_eq!(slots[0].size_mb, 16384); // 16 GB
    assert_eq!(slots[0].speed_mhz, 3200);
    assert_eq!(slots[0].configured_speed, 3200);
    assert_eq!(slots[0].mem_type, "DDR4");
    assert_eq!(slots[0].manufacturer, "0B94");
    assert_eq!(slots[0].part_number, "EAGET-16GB-3200");
    assert_eq!(slots[0].voltage, "1.2 V");

    assert_eq!(slots[1].locator, "ChannelB-DIMM1");
    assert_eq!(slots[1].size_mb, 16384);
}

#[test]
fn parse_dmidecode_filters_empty_slots() {
    let text = "\
Handle 0x0001, DMI type 17, 84 bytes
Memory Device
\tSize: 8 GB
\tLocator: DIMM_A1
\tType: DDR4
\tSpeed: 2666 MT/s
\tManufacturer: Samsung
\tPart Number: M378A1K43CB2-CTD
\tConfigured Memory Speed: 2666 MT/s
\tConfigured Voltage: 1.2 V

Handle 0x0002, DMI type 17, 84 bytes
Memory Device
\tSize: No Module Installed
\tLocator: DIMM_A2
\tType: Unknown
";
    let slots = parse_dmidecode_output(text);
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].locator, "DIMM_A1");
    assert_eq!(slots[0].size_mb, 8192);
}

#[test]
fn parse_dmidecode_size_in_mb() {
    let text = "\
Handle 0x0001, DMI type 17, 84 bytes
Memory Device
\tSize: 512 MB
\tLocator: DIMM0
\tType: DDR3
\tSpeed: 1333 MT/s
\tManufacturer: Elpida
\tPart Number: EBJ5UE8BDS0
\tConfigured Memory Speed: 1333 MT/s
\tConfigured Voltage: 1.5 V
";
    let slots = parse_dmidecode_output(text);
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].size_mb, 512);
}

#[test]
fn parse_dmidecode_empty_input() {
    let slots = parse_dmidecode_output("");
    assert!(slots.is_empty());
}

#[test]
fn parse_dmidecode_ignores_mapped_device() {
    let text = "\
Handle 0x0001, DMI type 17, 84 bytes
Memory Device
\tSize: 8 GB
\tLocator: DIMM_A1
\tType: DDR4
\tSpeed: 3200 MT/s
\tManufacturer: Kingston
\tPart Number: KF3200C16S4
\tConfigured Memory Speed: 3200 MT/s
\tConfigured Voltage: 1.2 V

Handle 0x0002, DMI type 20, 35 bytes
Memory Device Mapped Address
\tStarting Address: 0x00000000000
\tEnding Address: 0x001FFFFFFFF
";
    let slots = parse_dmidecode_output(text);
    assert_eq!(slots.len(), 1);
}

// ─── parse_dmidecode_array_output ────────────────────────────────────────────

#[test]
fn parse_array_gb_capacity() {
    let text = "\
Handle 0x0035, DMI type 16, 23 bytes
Physical Memory Array
\tLocation: System Board Or Motherboard
\tUse: System Memory
\tError Correction Type: None
\tMaximum Capacity: 64 GB
\tNumber Of Devices: 4
";
    let info = parse_dmidecode_array_output(text);
    assert_eq!(info.total_slots, 4);
    assert_eq!(info.max_capacity_mb, 65536); // 64 * 1024
}

#[test]
fn parse_array_tb_capacity() {
    let text = "\
Physical Memory Array
\tMaximum Capacity: 2 TB
\tNumber Of Devices: 8
";
    let info = parse_dmidecode_array_output(text);
    assert_eq!(info.total_slots, 8);
    assert_eq!(info.max_capacity_mb, 2 * 1024 * 1024);
}

#[test]
fn parse_array_mb_capacity() {
    let text = "\
Physical Memory Array
\tMaximum Capacity: 512 MB
\tNumber Of Devices: 2
";
    let info = parse_dmidecode_array_output(text);
    assert_eq!(info.total_slots, 2);
    assert_eq!(info.max_capacity_mb, 512);
}

#[test]
fn parse_array_empty_input() {
    let info = parse_dmidecode_array_output("");
    assert_eq!(info.total_slots, 0);
    assert_eq!(info.max_capacity_mb, 0);
}

// ─── parse_cpuinfo_for_pi ────────────────────────────────────────────────────

#[test]
fn parse_cpuinfo_pi5() {
    let cpuinfo = "\
processor\t: 0
BogoMIPS\t: 108.00
Features\t: fp asimd evtstrm
CPU implementer\t: 0x41
CPU architecture: 8
CPU variant\t: 0x4
CPU part\t: 0xd0b
CPU revision\t: 2

Hardware\t: BCM2835
Revision\t: d04170
Serial\t\t: 100000001234abcd
Model\t\t: Raspberry Pi 5 Model B Rev 1.1
";
    let info = parse_cpuinfo_for_pi(cpuinfo).unwrap();
    assert_eq!(info.model, "Raspberry Pi 5 Model B Rev 1.1");
    assert_eq!(info.mem_type, "LPDDR4X");
    assert!(info.freq_mhz.is_none());
    assert!(info.voltage.is_none());
}

#[test]
fn parse_cpuinfo_pi4() {
    let cpuinfo = "\
processor\t: 0
Model\t\t: Raspberry Pi 4 Model B Rev 1.4
";
    let info = parse_cpuinfo_for_pi(cpuinfo).unwrap();
    assert_eq!(info.mem_type, "LPDDR4");
}

#[test]
fn parse_cpuinfo_pi3() {
    let cpuinfo = "\
processor\t: 0
Model\t\t: Raspberry Pi 3 Model B Plus Rev 1.3
";
    let info = parse_cpuinfo_for_pi(cpuinfo).unwrap();
    assert_eq!(info.mem_type, "LPDDR2");
}

#[test]
fn parse_cpuinfo_not_pi() {
    let cpuinfo = "\
processor\t: 0
vendor_id\t: GenuineIntel
model name\t: Intel(R) Core(TM) i7-9750H CPU @ 2.60GHz
";
    assert!(parse_cpuinfo_for_pi(cpuinfo).is_none());
}

#[test]
fn parse_cpuinfo_model_not_raspberry() {
    let cpuinfo = "\
processor\t: 0
Model\t\t: ODROID-N2
";
    assert!(parse_cpuinfo_for_pi(cpuinfo).is_none());
}

#[test]
fn parse_cpuinfo_empty() {
    assert!(parse_cpuinfo_for_pi("").is_none());
}

// ─── parse_mobo_output ───────────────────────────────────────────────────────

#[test]
fn parse_mobo_standard() {
    let text = "\
Handle 0x0002, DMI type 2, 15 bytes
Base Board Information
\tManufacturer: Gigabyte Technology Co., Ltd.
\tProduct Name: Z390 GAMING SLI-CF
\tVersion: x.x
\tSerial Number: Default string
";
    let info = parse_mobo_output(text);
    assert_eq!(info.manufacturer, "Gigabyte Technology Co., Ltd.");
    assert_eq!(info.product, "Z390 GAMING SLI-CF");
}

#[test]
fn parse_mobo_empty_input() {
    let info = parse_mobo_output("");
    assert!(info.manufacturer.is_empty());
    assert!(info.product.is_empty());
}

// ─── Defensive parsing edge cases ────────────────────────────────────────────

#[test]
fn parse_meminfo_garbage_values_become_zero() {
    let content = "\
MemTotal:       not-a-number kB
MemFree:
MemAvailable:   13631488 kB
";
    let s = parse_meminfo_content(content);
    assert_eq!(s.total_kb, 0);
    assert_eq!(s.free_kb, 0);
    assert_eq!(s.available_kb, 13631488);
}

#[test]
fn parse_dmidecode_unknown_speed_and_voltage() {
    // Some boards report "Unknown" for speeds/voltage — must parse to 0/kept string.
    let text = "\
Memory Device
\tSize: 8192 MB
\tLocator: DIMM_A1
\tType: DDR4
\tSpeed: Unknown
\tConfigured Memory Speed: Unknown
\tConfigured Voltage: Unknown
";
    let slots = parse_dmidecode_output(text);
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].speed_mhz, 0);
    assert_eq!(slots[0].configured_speed, 0);
    assert_eq!(slots[0].voltage, "Unknown");
}

#[test]
fn parse_array_garbage_device_count() {
    let text = "\
Physical Memory Array
\tMaximum Capacity: 64 GB
\tNumber Of Devices: many
";
    let info = parse_dmidecode_array_output(text);
    assert_eq!(info.total_slots, 0);
    assert_eq!(info.max_capacity_mb, 64 * 1024);
}

#[test]
fn parse_cpuinfo_compute_module_4() {
    let cpuinfo = "\
processor\t: 0
Model\t\t: Raspberry Pi Compute Module 4 Rev 1.1
";
    let info = parse_cpuinfo_for_pi(cpuinfo).expect("CM4 should be detected");
    assert_eq!(info.mem_type, "LPDDR4");
}
