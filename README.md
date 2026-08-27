# raminfo

![CI](https://github.com/swayoleg/raminfo/actions/workflows/ci.yml/badge.svg)
![Platforms](https://img.shields.io/badge/platform-linux%20%7C%20macos%20%7C%20windows-blue)
![License MIT](https://img.shields.io/badge/license-MIT-green)

[Install](#install) · [Usage](#usage) · [Testing](#testing) · [Roadmap](#roadmap--todo) · [Contributing](CONTRIBUTING.md)

A RAM inspection tool for Linux, macOS and Windows. Shows DIMM slot details (model, vendor, frequency), memory usage stats, RAM temperatures and top consumers.

Since **0.3.0** the default is a full-screen interactive app (built with [ratatui](https://ratatui.rs)) with four tabs — Overview, Hardware, Processes, Temps — live gauges and sparkline history. Pipe or redirect the output and you get the plain text / JSON reports exactly as before, so scripts and `jq` pipelines keep working unchanged.

Raspberry Pi supported.

> DIMM slot / motherboard details need `dmidecode` (Linux, via `sudo`), `system_profiler` (macOS) or WMI (Windows). RAM temperatures are Linux + DDR5 only.

```
╭ raminfo v0.3.0 ─────────────────────────────────────────────────────────────────|linux · 31.3 GB|╮
│(1)Overview  (2)Hardware  (3)Processes  (4)Temps                                    q:quit  ?:help│
│╭────────────────────────────────────────────|Memory|╮╭──────────────────────────|Used % history|╮│
││ MB     total  used  free   buff/cache  available   ││                              ▂▃▃▄▄▅▅▅▆▆▇█││
││ Mem:    32075 20821   2941        8312       13312 ││                          ▁▂▂▃▄████████████││
││ Swap:   16384  2929  13454                         ││                    ▁▂▃▄▅▆████████████████││
│╰────────────────────────────────────────────────────╯│ ▄▄▅▅▅▆▆▆▇▇███████████████████████████████││
│╭─────────────────────────────────────────|Breakdown|╮│ █████████████████████████████████████████││
││ Used          18.3 GB  ██████████████████░░░░░░░░░ ││ █████████████████████████████████████████││
││ Available     13.0 GB                              │╰──────────────────────────────────────────╯│
││ Free           2.9 GB                              │╭─────────────────────────────────|Summary|╮│
││ Buffers        500 MB                              ││ In use        18.3 GB  (58%)             ││
││ Cached         7.6 GB                              ││ Available     13.0 GB                    ││
││ Swap used      2.9 GB  █████░░░░░░░░░░░░░░░░░░░░░░ ││ Top process   chrome (2.4 GB)            ││
││ Swap total    16.0 GB                              ││ Max temp      44.5°C                     ││
│╰────────────────────────────────────────────────────╯│ Modules       2 × 16 GB                  ││
│╭───────────────────────────────────────────────|RAM|╮│                                          ││
││ ██████████████18.3 GB / 31.3 GB  58%               ││                                          ││
│╰────────────────────────────────────────────────────╯│                                          ││
│╭──────────────────────────────────────────────|Swap|╮│                                          ││
││ █████████     2.9 GB / 16.0 GB  18%                ││                                          ││
│╰────────────────────────────────────────────────────╯╰──────────────────────────────────────────╯│
│refresh 2s • 1-4 tabs • g group • r refresh • +/- rate • ? help                                    │
╰──────────────────────────────────────────────────────────────────────────────────────────────────╯
```

<details>
<summary><b>Hardware</b>, <b>Processes</b> and <b>Temps</b> tabs</summary>

```
│╭────────────────────────────────────────────────────────────────────────────────────|DIMM Slots|╮│
││ Slot             Part Number           Vendor         Size Type    Max MT/s  Cfg MT/s  Voltage ││
││ ChannelA-DIMM0   M471A2G43CB2-CWE      Samsung       16 GB DDR4        3200      3200    1.2 V ││
││ ChannelB-DIMM0   M471A2G43CB2-CWE      Samsung       16 GB DDR4        3200      3200    1.2 V ││
│╰────────────────────────────────────────────────────────────────────────────────────────────────╯│
│╭────────────────────────────────────────────────────────────────────────────────────────|System|╮│
││ Slots         2 used / 4 total  ●●○○                                                           ││
││ Max capacity  64 GB (32 GB free)                                                               ││
││ Motherboard   Gigabyte Z390-AORUS                                                              ││
│╰────────────────────────────────────────────────────────────────────────────────────────────────╯│

│╭ press g to group by name ─────────────────────────────────|Top Consumers · 355 processes|╮│
││     #      PID Process                                               RSS  Share vs top         ││
││ ▌  1.     1234 chrome                                             2.4 GB   7.7% ██████████████ ││
││    2.     4321 rust-analyzer                                      878 MB   2.8% █████░░░░░░░░░ ││
│╰────────────────────────────────────────────────────────────────────────────────────────────────╯│

│╭──────────────────────────────────────────────────────────────────────────────|RAM Temperatures|╮│
││ Sensor                                                              Temp  Level                ││
││ spd5118 DIMM 0                                                    42.5°C  ███░░░░░░░░░░░░░░░░░ ││
││ spd5118 DIMM 1                                                    44.5°C  ████░░░░░░░░░░░░░░░░ ││
│╰────────────────────────────────────────────────────────────────────────────────────────────────╯│
```

</details>

## Keys

| Key | Action |
|---|---|
| `1` `2` `3` `4` | Jump to Overview / Hardware / Processes / Temps |
| `Tab` / `→`, `Shift+Tab` / `←` | Cycle tabs forward / back |
| `↑` `↓` `PgUp` `PgDn` `Home` `End` | Scroll the process list |
| `g` | Group / ungroup processes by name (sums RSS and counts workers) |
| `r` | Refresh now |
| `+` / `-` | Slower / faster refresh (1–60 s) |
| `?` | Toggle the help overlay |
| `Esc` | Close the help overlay, otherwise quit |
| `q` / `Ctrl+C` | Quit |

## Non-interactive output

The interactive app only starts when stdout is a terminal *and* no output mode was requested. Everything below is unchanged from 0.2.x:

```bash
raminfo | cat                  # short free(1)-style summary
raminfo --short                # same, explicitly
raminfo --full                 # full one-shot report (DIMM table, temps, consumers)
raminfo --json                 # one compact JSON object, then exit
raminfo --monitor --json --interval 2   # ndjson stream: one object every 2 s, forever
```

> **For monitoring tools:** the streaming form needs **all three** flags —
> `--monitor` (keep going), `--json` (machine-readable) and `--interval N`
> (seconds between lines). `raminfo --json --interval 5` on its own prints
> **one** snapshot and exits: `--interval` only has an effect together with
> `--monitor`. `raminfo --interval 5` on a terminal just opens the app with a
> 5-second tick.

## The classic one-shot report (`--full`)

```
  DIMM Slots
╭──────────┬────────────────────────┬────────────────┬─────────┬──────┬───────────┬───────────┬─────────╮
│ Slot     │ Part Number            │ Vendor         │    Size │ Type │  Max MT/s │  Cfg MT/s │ Voltage │
├──────────┼────────────────────────┼────────────────┼─────────┼──────┼───────────┼───────────┼─────────┤
│ ChannelA │ M471A2G43CB2-CWE       │ Samsung        │   16 GB │ DDR4 │      3200 │      3200 │  1.2 V  │
│ ChannelB │ M471A2G43CB2-CWE       │ Samsung        │   16 GB │ DDR4 │      3200 │      3200 │  1.2 V  │
╰──────────┴────────────────────────┴────────────────┴─────────┴──────┴───────────┴───────────┴─────────╯

  Memory Usage
╭──────────────────────────────────────────────────────────────────────╮
│  RAM  32 GB installed across 2 DIMM slots                   31.3 GB  │
├──────────────────────────────────────────────────────────────────────┤
│  Used            18.0 GB   █████████████████████░░░░░░░░░░░░░░░  57% │
│  Available       13.3 GB                                             │
│  Free             1.4 GB                                             │
│  Buffers          1.5 GB                                             │
│  Cached           8.5 GB                                             │
├──────────────────────────────────────────────────────────────────────┤
│  Swap Used        555 MB   █░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   3% │
│  Swap Total      16.0 GB                                             │
╰──────────────────────────────────────────────────────────────────────╯

  Top Memory Consumers
╭────────────────────────────────────────────────────────────────────────────────╮
│  #    PID      Process                     RSS                           Share │
├────────────────────────────────────────────────────────────────────────────────┤
│   1.  9703     phpstorm                 5.7 GB   ████░░░░░░░░░░░░░░░░░░░░  18% │
│   2.  9907     copilot-languag          883 MB   █░░░░░░░░░░░░░░░░░░░░░░░   2% │
│   3.  10702    chrome                   821 MB   █░░░░░░░░░░░░░░░░░░░░░░░   2% │
╰────────────────────────────────────────────────────────────────────────────────╯

 Upgrade Potential
╭────────────────────────────────────────────────────────╮
│  Slots           2 used / 4 total  ●●○○                │
│  Installed       32 GB                                 │
│  Maximum         64 GB                                 │
│  Headroom        32 GB available                       │
│  Max Freq        3200 MT/s                             │
╰────────────────────────────────────────────────────────╯

  Motherboard
╭──────────────────────────────────────────────────────────────────────╮
│  Manufacturer        Gigabyte Technology Co., Ltd.                   │
│  Product             Z390 GAMING SLI-CF                              │
╰──────────────────────────────────────────────────────────────────────╯
```

Or in RaspberryPi

```
  Board Memory
╭────────────────────────────────────────────────────────╮
│  Raspberry Pi 5 Model B Rev 1.1                        │
├────────────────────────────────────────────────────────┤
│  Type          LPDDR4X  (soldered, no slots)           │
│  Frequency     4267 MT/s (default)                     │
│  Voltage       0.6000V                                 │
╰────────────────────────────────────────────────────────╯

  Memory Usage
╭──────────────────────────────────────────────────────────────────────╮
│  RAM                                                        15.8 GB  │
├──────────────────────────────────────────────────────────────────────┤
│  Used             2.3 GB   █████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  14% │
│  Available       13.5 GB                                             │
│  Free             536 MB                                             │
│  Buffers          1.2 GB                                             │
│  Cached          11.8 GB                                             │
├──────────────────────────────────────────────────────────────────────┤
│  Swap Used        244 MB   █████████████████░░░░░░░░░░░░░░░░░░░  47% │
│  Swap Total       511 MB                                             │
╰──────────────────────────────────────────────────────────────────────╯

  Top Memory Consumers
╭─────────────────────────────────────────────────────────────────────────────────╮
│  #    PID      Process                     RSS                             Share│
├─────────────────────────────────────────────────────────────────────────────────┤
│   1.  2928     jellyfin                 528 MB   █░░░░░░░░░░░░░░░░░░░░░░░    3% │
│   2.  868      node                     171 MB   ░░░░░░░░░░░░░░░░░░░░░░░░    1% │
│   3.  892      MainThread               156 MB   ░░░░░░░░░░░░░░░░░░░░░░░░    0% │
│   4.  973599   pcmanfm                  149 MB   ░░░░░░░░░░░░░░░░░░░░░░░░    0% │
│   5.  1147     syncthing                108 MB   ░░░░░░░░░░░░░░░░░░░░░░░░    0% │
│   6.  2008011  mariadbd                 105 MB   ░░░░░░░░░░░░░░░░░░░░░░░░    0% │
│   7.  1020     labwc                    104 MB   ░░░░░░░░░░░░░░░░░░░░░░░░    0% │
│   8.  1453483  cps                       98 MB   ░░░░░░░░░░░░░░░░░░░░░░░░    0% │
│   9.  899      wayvnc                    98 MB   ░░░░░░░░░░░░░░░░░░░░░░░░    0% │
│  10.  865      node                      79 MB   ░░░░░░░░░░░░░░░░░░░░░░░░    0% │
╰─────────────────────────────────────────────────────────────────────────────────╯
```

## Screenshots

**Overview** — `free -m`-style table, breakdown bars, RAM/swap gauges and an auto-scaled used-% history:

![raminfo Overview tab](assets/tui-overview.png)

**Hardware** — DIMM slots, part numbers, speeds, motherboard and free slots (needs `sudo`):

![raminfo Hardware tab](assets/tui-hardware.png)

**Processes** — every process sorted by RSS, live; press `g` to group all workers of the same name (e.g. every `chrome` or `claude`) into one row:

![raminfo Processes tab](assets/tui-processes.png)

**Temps** — DDR5 sensor readings, or a note when the hardware has none (DDR4 here):

![raminfo Temps tab](assets/tui-temps.png)

**Raspberry Pi** — the Hardware tab shows the board and its soldered LPDDR memory (Pi 5 here, via `vcgencmd`):

![raminfo Hardware tab on a Raspberry Pi 5](assets/tui-raspberry.png)


## RAM Temperatures

Temperature support depends on hardware. The `RAM Temperatures` section is shown automatically when sensors are available and silently skipped when they are not.

| Hardware | Support |
|---|---|
| DDR5 | ✅ via `spd5118` kernel driver |
| DDR4 | ❌ no sensor chip on the DIMM itself |

On DDR4 systems the section will never appear — this is expected, not a bug.


# Install

## 1. Cargo (if you have Rust)

```bash
cargo install raminfo
```

That's it. The binary lands in `~/.cargo/bin/` which is already in your `$PATH` if Rust is set up normally.

---

## 2. Pre-compiled binaries

Download the binary for your architecture from the [Releases](https://github.com/swayoleg/raminfo/releases) page.

| File | Target |
|---|---|
| `raminfo-x86_64-linux` | 64-bit Linux (most desktops / servers) |
| `raminfo-aarch64-linux` | 64-bit ARM — modern distros (Raspberry Pi 4/5 on Ubuntu 22.04+, AWS Graviton) |
| `raminfo-aarch64-linux-static` | 64-bit ARM — older distros (Raspberry Pi 4/5 on Raspberry Pi OS Bullseye/Bookworm) |
| `raminfo-armv7-linux` | 32-bit ARM — modern distros (Raspberry Pi 2/3 on Ubuntu 22.04+) |
| `raminfo-armv7-linux-static` | 32-bit ARM — older distros (Raspberry Pi 2/3 on Raspberry Pi OS Bullseye/Bookworm) |


```bash
# example for x86_64
curl -L https://github.com/swayoleg/raminfo/releases/latest/download/raminfo-x86_64-linux \
  -o raminfo
chmod +x raminfo
sudo mv raminfo /usr/local/bin/
```

---

## 3. Compile from source
```bash
git clone https://github.com/swayoleg/raminfo
cd raminfo
cargo build --release
sudo cp target/release/raminfo /usr/local/bin/
```

**Cross-compiling for ARM** (from an x86 machine):
```bash
# install targets
rustup target add aarch64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-musl
rustup target add armv7-unknown-linux-gnueabihf
rustup target add armv7-unknown-linux-musleabihf

# install cross-linkers (Ubuntu/Debian)
sudo apt install musl-tools gcc-aarch64-linux-gnu gcc-arm-linux-gnueabihf

# build gnu (modern distros)
cargo build --release --target aarch64-unknown-linux-gnu
cargo build --release --target armv7-unknown-linux-gnueabihf

# build musl/static (older distros, Raspberry Pi OS)
cargo build --release --target aarch64-unknown-linux-musl
cargo build --release --target armv7-unknown-linux-musleabihf
```

Also add to `.cargo/config.toml`:
```toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"

[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"

[target.aarch64-unknown-linux-musl]
linker = "aarch64-linux-gnu-gcc"
rustflags = ["-C", "target-feature=+crt-static"]

[target.armv7-unknown-linux-musleabihf]
linker = "arm-linux-gnueabihf-gcc"
rustflags = ["-C", "target-feature=+crt-static"]
```

Binaries will be in `target/<target-triple>/release/raminfo`.

---

## Post-install: dmidecode

DIMM slot details require `dmidecode`:

```bash
# Debian / Ubuntu
sudo apt install dmidecode

# Arch
sudo pacman -S dmidecode

# Fedora
sudo dnf install dmidecode
```

For passwordless sudo, add to `/etc/sudoers`:
```
your_user ALL=(ALL) NOPASSWD: /usr/sbin/dmidecode
```

# Usage

```bash
raminfo             # interactive app (full-screen, four tabs)
sudo raminfo        # same, with DIMM slot / motherboard details filled in
raminfo | cat       # not a terminal → plain short summary instead
```

DIMM slot info requires `sudo` to call `dmidecode`. For passwordless use, add to `/etc/sudoers`:
```
your_user ALL=(ALL) NOPASSWD: /usr/sbin/dmidecode
```

## Options

| Flag | Description |
|---|---|
| *(none)* | Interactive app when stdout is a terminal; short plain summary when it is not |
| `--tui` | Force the interactive app. Errors out (exit `1`) if stdout is not a terminal |
| `--short` | Compact `free -m`-style summary, never interactive |
| `--full` | Full one-shot report (DIMM table, upgrade potential, temps, consumers), never interactive |
| `--json` | Print a single full JSON snapshot and exit (for scripting / `jq`) |
| `--monitor` | Keep refreshing. On a terminal this is the interactive app; with `--json` it is an ndjson stream; piped without `--json` it repeats the plain dynamic report |
| `--interval <seconds>` | Refresh / tick rate (default: `2`, minimum `1`). Also accepts `--interval=<seconds>`. Adjustable at runtime with `+`/`-`. Only meaningful with `--monitor` or the interactive app — `--json` without `--monitor` is always a single snapshot |
| `-h`, `--help` | Print usage and exit |

`--tui` is mutually exclusive with `--short`, `--full` and `--json`; `--short`
and `--full` remain mutually exclusive with each other.

### What runs where

| Invocation | stdout is a terminal | stdout is piped / redirected |
|---|---|---|
| `raminfo` | interactive app | short summary (full when run as root) |
| `raminfo --tui` | interactive app | error, exit `1` |
| `raminfo --monitor` | interactive app | repeating plain report |
| `raminfo --short` / `--full` | plain report | plain report |
| `raminfo --json` | one JSON object | one JSON object |
| `raminfo --monitor --json` | ndjson stream | ndjson stream |

```bash
raminfo --json | jq                       # single JSON object (full snapshot)
raminfo --json | jq '.mem.total_kb'       # pick out one field
raminfo --monitor                         # interactive app, refreshing live
raminfo --monitor --interval 5            # refresh every 5 seconds
raminfo --monitor --json | jq -c .        # ndjson stream, one object per refresh

# follow a single field live (jq --unbuffered so it prints each line as it arrives)
raminfo --monitor --json --interval 2 | jq --unbuffered '.mem.available_kb'

# log samples to a file for later analysis
raminfo --monitor --json >> ram-log.ndjson
```

`--json` (single-shot) prints one compact object with the **full** snapshot
(DIMM slots, motherboard, memory array, usage, consumers).

**Monitor mode only refreshes what changes** — memory usage, temperatures, and
top consumers. Static hardware details (DIMM slots, motherboard, max capacity)
are shown once by the single-shot commands and omitted from monitor output, so
`--monitor` stays focused and doesn't re-run `dmidecode` every cycle. Under
`--monitor --json` each ndjson line therefore contains just `mem`, `temps`, and
`top_consumers` — ideal for logging or streaming into tools that read a line at
a time.

# Library

The core parsing lives in a reusable `raminfo` library crate. Add it to your project:

```bash
cargo add raminfo
```

or in `Cargo.toml`:

```toml
[dependencies]
raminfo = "0.1"
```

The crate exposes two tiers of functions, depending on whether you want it to
read the system for you or just parse text you already have.

### 1. Read the system for you

The simplest entry point is `collect_snapshot`, which gathers every data source
into one serializable [`Snapshot`]:

```rust
fn main() {
    let snap = raminfo::parsers::collect_snapshot();

    println!("Total RAM: {} kB", snap.mem.total_kb);
    for dimm in &snap.dimms {
        println!("{}: {} MB {}", dimm.locator, dimm.size_mb, dimm.mem_type);
    }

    // Or serialize the whole snapshot to JSON:
    println!("{}", raminfo::json::to_json(&snap));
}
```

Prefer one source? Call a single collector — each reads its source directly:

```rust
let mem   = raminfo::parsers::parse_proc_meminfo();   // /proc/meminfo
let dimms = raminfo::parsers::parse_dmidecode();      // dmidecode -t 17 (needs sudo)
let temps = raminfo::parsers::read_ram_temps();       // /sys/class/hwmon
let top   = raminfo::parsers::top_mem_consumers(5);   // /proc/*/status
```

All of these degrade gracefully (returning empty/default data) when a source is
unavailable — they never panic. DIMM and motherboard details require `dmidecode`
(typically via sudo).

### 2. Use it as a pure parser (bring your own text)

If you already have the raw text — read from a file, captured on another machine,
or produced by your own privileged call — the `*_content` / `*_output` functions
parse a `&str` into the same structs and touch nothing on your system:

```rust
use raminfo::parsers::{parse_meminfo_content, parse_dmidecode_output};

// e.g. text you collected over SSH from a remote host
let meminfo = std::fs::read_to_string("captured/meminfo.txt").unwrap();
let stats   = parse_meminfo_content(&meminfo);
println!("remote total: {} kB", stats.total_kb);

let dmi   = std::fs::read_to_string("captured/dmidecode-17.txt").unwrap();
let dimms = parse_dmidecode_output(&dmi);
println!("remote DIMMs: {}", dimms.len());
```

Pure parsers available: `parse_meminfo_content`, `parse_dmidecode_output`,
`parse_dmidecode_array_output`, `parse_mobo_output`, and `parse_cpuinfo_for_pi`.
These are deterministic and side-effect-free, which is exactly what the test
suite exercises with mock data.

### API docs

Every public item is documented, and the crate's own doc examples are compiled
and run as doctests (`cargo test`), so the documented API stays correct. Browse
the full API locally with:

```bash
cargo doc --open --no-deps
```

# Testing

```bash
cargo test                        # run all tests
cargo test --test format_tests    # formatting helpers only
cargo test --test parsing_tests   # parser logic only
```

Tests live in the `tests/` directory and cover formatting utilities (`format.rs`), all parsing logic (per-platform `parsers/`), JSON output (`json.rs`) and the interactive app's rendering (`tui_tests.rs`, which draws every tab from a mock snapshot onto a `ratatui` `TestBackend` and asserts on the resulting text). Argument parsing and the TUI state machine / ring buffer are unit-tested inline in `main.rs` and `src/tui/`. The `src/lib.rs` file also exposes the crate as a reusable [library](#library).

```bash
cargo test --test tui_tests       # interactive app rendering only
```

# Roadmap / TODO

- [x] Cover with tests
- [x] `--monitor` — live refresh mode
- [x] `--interval <seconds>` — refresh rate for monitor mode (default: 2s)
- [x] `--monitor --json` — newline-delimited JSON stream (ndjson)
- [x] `--json` — single-shot JSON output for scripting and piping to `jq`
- [x] Windows support via WMI (`Win32_PhysicalMemory`, `Win32_OperatingSystem`)
- [x] macOS support (`sysctl`, `vm_stat`, `system_profiler`)
- [x] Interactive full-screen app (ratatui) with tabs, gauges and sparkline history
- [x] Refactor into lib + binary — expose core parsing as a reusable library crate

# Dependencies

- [`colored`](https://crates.io/crates/colored) — terminal colors
- [`serde`](https://crates.io/crates/serde) / [`serde_json`](https://crates.io/crates/serde_json) — JSON output (`--json`) and serializable data structures
- [`ratatui`](https://crates.io/crates/ratatui) — the interactive full-screen app (uses its bundled `crossterm` backend; no separate dependency)
- [`ctrlc`](https://crates.io/crates/ctrlc) — restore the terminal on Ctrl+C in the plain `--monitor` loop
- `dmidecode` — system package, required for DIMM slot details, will ignore dim slots if not installed
