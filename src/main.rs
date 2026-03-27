mod types;
mod format;
mod parsers;
mod render;

use parsers::*;
use render::*;
use colored::*;


// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let stats = parse_proc_meminfo();
    let dimms = parse_dmidecode();
    let array = parse_dmidecode_array();
    let temps = read_ram_temps();
    let top10 = top_mem_consumers(10);
    let pi    = detect_raspberry_pi();

    match (&pi, dimms.is_empty()) {
        (Some(info), _) => {
            render_pi_board(info);
        }
        (None, false) => {
            render_dimm_table(&dimms);  // stays at top
        }
        (None, true) => {
            println!();
            println!("  {} {}", "⚠".yellow(),
                     "No DIMM hardware data — run with sudo for slot/model details".dimmed());
        }
    }

    render_mem_stats(&stats, &dimms);
    render_temps(&temps);
    render_top_consumers(&top10, stats.total_kb);

    if pi.is_none() && !dimms.is_empty() {
        render_upgrade_potential(&dimms, &array);
        render_mobo_info(&read_mobo_info());
    }

    println!();
}