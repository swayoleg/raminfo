use raminfo::json::{to_json, to_json_monitor};
use raminfo::parsers::parse_meminfo_content;
use raminfo::types::*;
use serde_json::Value;

fn sample_snapshot() -> Snapshot {
    let mem = parse_meminfo_content("\
MemTotal:       32845696 kB
MemFree:         3012345 kB
MemAvailable:   13631488 kB
SwapTotal:      16777216 kB
SwapFree:       13777216 kB
");

    Snapshot {
        mem,
        dimms: vec![DimmSlot {
            locator: "ChannelA-DIMM0".to_string(),
            size_mb: 16384,
            speed_mhz: 3200,
            mem_type: "DDR4".to_string(),
            manufacturer: "Samsung".to_string(),
            part_number: "M471A2G43CB2-CWE".to_string(),
            configured_speed: 3200,
            voltage: "1.2 V".to_string(),
        }],
        array: MemArrayInfo { total_slots: 4, max_capacity_mb: 65536 },
        temps: vec![TempReading { label: "DIMM 0".to_string(), celsius: 42.5 }],
        top_consumers: vec![ProcessMem { pid: 1234, name: "chrome".to_string(), rss_kb: 500_000 }],
        pi: None,
        mobo: MoboInfo {
            manufacturer: "Gigabyte".to_string(),
            product: "Z390".to_string(),
        },
    }
}

#[test]
fn to_json_produces_single_line() {
    let out = to_json(&sample_snapshot());
    assert!(!out.contains('\n'), "JSON output must be single-line for ndjson");
}

#[test]
fn to_json_round_trips_key_fields() {
    let out = to_json(&sample_snapshot());
    let v: Value = serde_json::from_str(&out).expect("output must be valid JSON");

    assert_eq!(v["mem"]["total_kb"], 32845696u64);
    assert_eq!(v["mem"]["swap_total_kb"], 16777216u64);

    assert_eq!(v["dimms"].as_array().unwrap().len(), 1);
    assert_eq!(v["dimms"][0]["locator"], "ChannelA-DIMM0");
    assert_eq!(v["dimms"][0]["size_mb"], 16384u64);

    assert_eq!(v["array"]["total_slots"], 4u64);
    assert_eq!(v["temps"][0]["celsius"], 42.5);
    assert_eq!(v["top_consumers"][0]["name"], "chrome");
    assert_eq!(v["mobo"]["product"], "Z390");
}

#[test]
fn pi_is_null_when_absent() {
    let out = to_json(&sample_snapshot());
    let v: Value = serde_json::from_str(&out).unwrap();
    assert!(v["pi"].is_null());
}

#[test]
fn monitor_json_omits_static_fields() {
    let out = to_json_monitor(&sample_snapshot());
    assert!(!out.contains('\n'), "monitor JSON must be single-line for ndjson");

    let v: Value = serde_json::from_str(&out).expect("output must be valid JSON");
    let obj = v.as_object().unwrap();

    // Only the dynamic fields are present.
    assert!(obj.contains_key("mem"));
    assert!(obj.contains_key("temps"));
    assert!(obj.contains_key("top_consumers"));
    // Static hardware fields are omitted.
    assert!(!obj.contains_key("dimms"));
    assert!(!obj.contains_key("array"));
    assert!(!obj.contains_key("mobo"));
    assert!(!obj.contains_key("pi"));

    // Dynamic values still round-trip.
    assert_eq!(v["mem"]["total_kb"], 32845696u64);
    assert_eq!(v["top_consumers"][0]["name"], "chrome");
}

#[test]
fn empty_snapshot_serializes() {
    let out = to_json(&Snapshot::default());
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["mem"]["total_kb"], 0u64);
    assert_eq!(v["dimms"].as_array().unwrap().len(), 0);
    assert!(v["pi"].is_null());
}
