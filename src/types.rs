use serde::Serialize;

// ─── Structs ──────────────────────────────────────────────────────────────────

/// A single physical memory module (DIMM), as reported by `dmidecode -t 17`.
#[derive(Debug, Default, Clone, Serialize)]
pub struct DimmSlot {
    /// Slot locator, e.g. `ChannelA-DIMM0`.
    pub locator: String,
    /// Module capacity in megabytes.
    pub size_mb: u64,
    /// Rated (maximum) speed in MT/s.
    pub speed_mhz: u64,
    /// Memory type, e.g. `DDR4`, `DDR5`.
    pub mem_type: String,
    /// Module manufacturer.
    pub manufacturer: String,
    /// Manufacturer part number.
    pub part_number: String,
    /// Configured (running) speed in MT/s.
    pub configured_speed: u64,
    /// Configured voltage string, e.g. `1.2 V`.
    pub voltage: String,
}

/// System memory statistics parsed from `/proc/meminfo`. All values are in kilobytes.
#[derive(Debug, Default, Clone, Serialize)]
pub struct MemStats {
    pub total_kb: u64,
    pub free_kb: u64,
    pub available_kb: u64,
    pub buffers_kb: u64,
    pub cached_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
}

/// A process and its resident set size (RSS), from `/proc/<pid>/status`.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessMem {
    pub pid: u32,
    pub name: String,
    pub rss_kb: u64,
}

/// Raspberry Pi board memory details (soldered LPDDR, no DIMM slots).
#[derive(Debug, Clone, Serialize)]
pub struct PiBoardInfo {
    pub model: String,
    pub mem_type: String,
    pub freq_mhz: Option<u64>,
    pub voltage: Option<String>,
}

/// Physical Memory Array info from `dmidecode -t 16` (slot count and max capacity).
#[derive(Debug, Default, Clone, Serialize)]
pub struct MemArrayInfo {
    pub total_slots: u32,
    pub max_capacity_mb: u64,
}

/// Motherboard identification from `dmidecode -t 2`.
#[derive(Debug, Default, Clone, Serialize)]
pub struct MoboInfo {
    pub manufacturer: String,
    pub product: String,
}

/// A single temperature sensor reading (label and degrees Celsius).
#[derive(Debug, Clone, Serialize)]
pub struct TempReading {
    pub label: String,
    pub celsius: f64,
}

/// A complete point-in-time snapshot of all memory-related system data.
///
/// This is the shared data model produced by
/// [`collect_snapshot`](crate::parsers::collect_snapshot) and consumed by both
/// the TUI renderer ([`render_snapshot`](crate::render::render_snapshot)) and the
/// JSON serializer ([`to_json`](crate::json::to_json)).
#[derive(Debug, Default, Clone, Serialize)]
pub struct Snapshot {
    /// Memory usage statistics from `/proc/meminfo`.
    pub mem: MemStats,
    /// Installed DIMM modules (empty without sudo/`dmidecode`, or on Raspberry Pi).
    pub dimms: Vec<DimmSlot>,
    /// Physical memory array info (slot count, max capacity).
    pub array: MemArrayInfo,
    /// RAM temperature sensor readings (DDR5 only; empty otherwise).
    pub temps: Vec<TempReading>,
    /// Top memory-consuming processes, sorted by RSS descending.
    pub top_consumers: Vec<ProcessMem>,
    /// Raspberry Pi board info, if running on a Pi.
    pub pi: Option<PiBoardInfo>,
    /// Motherboard identification.
    pub mobo: MoboInfo,
}
