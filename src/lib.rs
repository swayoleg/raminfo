//! `raminfo` — a Linux RAM inspection library.
//!
//! This crate powers the `raminfo` CLI but is usable on its own to read
//! memory-related system data on Linux. It parses `/proc/meminfo`,
//! `/proc/*/status`, `/sys/class/hwmon/`, and `dmidecode`/`vcgencmd` output into
//! plain, serializable data structures.
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
//! Linux only. On other platforms the parsers return empty/default data.

pub mod types;
pub mod format;
pub mod parsers;
pub mod render;
pub mod json;
