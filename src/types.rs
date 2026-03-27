// ─── Structs ──────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct DimmSlot {
    pub locator: String,
    pub size_mb: u64,
    pub speed_mhz: u64,
    pub mem_type: String,
    pub manufacturer: String,
    pub part_number: String,
    pub configured_speed: u64,
    pub voltage: String,
}

#[derive(Debug, Default)]
pub struct MemStats {
    pub total_kb: u64,
    pub free_kb: u64,
    pub available_kb: u64,
    pub buffers_kb: u64,
    pub cached_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
}

#[derive(Debug)]
pub struct ProcessMem {
    pub pid: u32,
    pub name: String,
    pub rss_kb: u64,
}

#[derive(Debug)]
pub struct PiBoardInfo {
    pub model: String,
    pub mem_type: String,
    pub freq_mhz: Option<u64>,
    pub voltage: Option<String>,
}

#[derive(Debug, Default)]
pub struct MemArrayInfo {
    pub total_slots: u32,
    pub max_capacity_mb: u64,
}

#[derive(Debug, Default)]
pub struct MoboInfo {
    pub manufacturer: String,
    pub product: String,
}