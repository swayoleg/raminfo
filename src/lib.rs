//! `raminfo` — a cross-platform RAM inspection library.
//!
//! This crate powers the `raminfo` CLI but is usable on its own to read
//! memory-related system data. Each OS has its own parser backend under
//! [`parsers`]: Linux reads `/proc/meminfo`, `/proc/*/status`,
//! `/sys/class/hwmon/`, and `dmidecode`/`vcgencmd` output; macOS uses
//! `sysctl`, `vm_stat`, `ps`, and `system_profiler`; Windows queries CIM
//! classes via PowerShell. All backends produce the same plain, serializable
//! data structures.
//!
//! The simplest entry point is [`parsers::collect_snapshot`], which returns a
//! [`types::Snapshot`] bundling every data source in one call. All parsers
//! degrade gracefully (returning empty/default data) when a source is
//! unavailable — they never panic. DIMM and motherboard details require
//! `dmidecode`, typically run via sudo.
//!
//! # Example
//!
//! ```no_run
//! let snap = raminfo::parsers::collect_snapshot();
//! println!("Total RAM: {} kB", snap.mem.total_kb);
//! for dimm in &snap.dimms {
//!     println!("{}: {} MB {}", dimm.locator, dimm.size_mb, dimm.mem_type);
//! }
//!
//! // Or emit the whole snapshot as JSON:
//! println!("{}", raminfo::json::to_json(&snap));
//! ```
//!
//! # Platform
//!
//! Linux is fully supported; macOS and Windows are supported via their native
//! tooling (no extra dependencies). On any other OS the collectors return
//! empty/default data — they never panic.

pub mod types;
pub mod format;
pub mod parsers;
pub mod render;
pub mod json;
pub mod tui;
