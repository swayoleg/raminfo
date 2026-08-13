use serde::Serialize;

use crate::types::{MemStats, ProcessMem, Snapshot, TempReading};

// ─── JSON output ────────────────────────────────────────────────────────────────

/// Serialize a [`Snapshot`] to a compact, single-line JSON string.
///
/// The output is single-line so it can be used unchanged as one record in a
/// newline-delimited JSON (ndjson) stream (monitor mode) or as a standalone
/// object for piping to tools like `jq`.
///
/// # Example
///
/// ```no_run
/// let snap = raminfo::parsers::collect_snapshot();
/// println!("{}", raminfo::json::to_json(&snap));
/// ```
pub fn to_json(snapshot: &Snapshot) -> String {
    // Serialization of a plain data struct cannot fail; fall back to `{}` defensively.
    serde_json::to_string(snapshot).unwrap_or_else(|_| "{}".to_string())
}

/// The dynamic subset of a [`Snapshot`] — the fields that change between
/// refreshes. Serialized (not stored) as the ndjson record in monitor mode.
#[derive(Serialize)]
struct MonitorView<'a> {
    mem: &'a MemStats,
    temps: &'a [TempReading],
    top_consumers: &'a [ProcessMem],
}

/// Serialize only the dynamic fields of a snapshot — `mem`, `temps`, and
/// `top_consumers` — to a compact single-line JSON string.
///
/// Used for the `--monitor --json` ndjson stream, where the static hardware
/// fields (DIMM slots, memory array, motherboard) are redundant on every line.
pub fn to_json_monitor(snapshot: &Snapshot) -> String {
    let view = MonitorView {
        mem: &snapshot.mem,
        temps: &snapshot.temps,
        top_consumers: &snapshot.top_consumers,
    };
    serde_json::to_string(&view).unwrap_or_else(|_| "{}".to_string())
}
