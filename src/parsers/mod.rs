//! Platform-dispatching parsers.
//!
//! Each supported OS has its own submodule ([`linux`], [`macos`], [`windows`])
//! exposing the same two entry points, `collect_snapshot()` and
//! `collect_dynamic()`, plus pure text-parsing functions that are compiled (and
//! unit-tested) on every platform. The module-level `collect_*` functions here
//! dispatch to the current target OS at compile time; on any other OS they
//! return default (empty) data, never panicking.

pub mod linux;
pub mod macos;
pub mod windows;

// Pure parsing helpers are re-exported at the historical `parsers::` paths so
// existing library consumers and the mock-input tests keep working unchanged.
pub use linux::{
    detect_raspberry_pi, parse_cpuinfo_for_pi, parse_dmidecode, parse_dmidecode_array,
    parse_dmidecode_array_output, parse_dmidecode_output, parse_meminfo_content,
    parse_mobo_output, parse_proc_meminfo, read_mobo_info, read_ram_temps, top_mem_consumers,
};

use crate::types::MemStats;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use crate::types::Snapshot;

/// Collect a complete [`Snapshot`](crate::types::Snapshot) for the current OS.
///
/// Dispatches to [`linux`], [`macos`], or [`windows`] at compile time. On any
/// other OS this returns an all-default snapshot.
#[cfg(target_os = "linux")]
pub use linux::{collect_dynamic, collect_dynamic_top, collect_snapshot};
#[cfg(target_os = "macos")]
pub use macos::{collect_dynamic, collect_dynamic_top, collect_snapshot};
#[cfg(target_os = "windows")]
pub use windows::{collect_dynamic, collect_dynamic_top, collect_snapshot};

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn collect_snapshot() -> Snapshot {
    Snapshot::default()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn collect_dynamic() -> Snapshot {
    Snapshot::default()
}

/// Like [`collect_dynamic`] but with `n` top consumers (`usize::MAX` = all).
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn collect_dynamic_top(_n: usize) -> Snapshot {
    Snapshot::default()
}

/// Collect only the memory usage statistics for the current OS — the cheapest
/// possible read, used by the default short (`free -m`-style) output mode.
pub fn collect_mem_stats() -> MemStats {
    #[cfg(target_os = "linux")]
    {
        linux::parse_proc_meminfo()
    }
    #[cfg(target_os = "macos")]
    {
        macos::collect_mem_stats()
    }
    #[cfg(target_os = "windows")]
    {
        windows::collect_mem_stats()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        MemStats::default()
    }
}
