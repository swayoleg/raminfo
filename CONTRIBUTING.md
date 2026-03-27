# Contributing

## Reporting Issues

Before opening an issue, please include the following — reports missing this info will be closed without response.

**Required in every issue:**

| Field | Example |
|---|---|
| OS & distro | `Ubuntu 24.04 LTS` / `Arch Linux` / `Fedora 40` |
| Kernel version | `uname -r` → `6.8.0-41-generic` |
| Rust version | `rustc --version` → `rustc 1.85.0` |
| RAM type | `DDR4` / `DDR5` |
| Run with sudo? | Yes / No |

**Template:**

```
OS:      Ubuntu 24.04
Kernel:  6.8.0-41-generic
Rust:    1.85.0
RAM:     DDR4
Sudo:    yes

Describe the issue here.
```

---

## Pull Requests

1. **Clone** the repo and create a branch from `master`
   ```bash
   git checkout -b feature/your-feature
   ```

2. **Keep changes focused** — one feature or fix per PR, no unrelated cleanups mixed in

3. **Build must pass** with zero warnings
   ```bash
   cargo build --release
   ```

4. **Include OS and hardware info** in the PR description — same fields as the issue template above. This is mandatory because behaviour differs across kernels, distros and RAM generations (DDR4 vs DDR5 sensor availability, hwmon driver names, dmidecode output format, etc.)

5. **Describe what you tested** — which sections rendered correctly, whether you ran with and without sudo

6. Open the PR against `master`

---

## Notes

- This tool is **Linux only** — PRs adding Windows/macOS support are welcome but must live behind `#[cfg(target_os)]` and not break the Linux build
- DDR5 temperature support relies on the `spd5118` kernel driver — DDR4 has no hardware path to DIMM temps, do not report this as a bug
- If `dmidecode` output looks different on your distro, include the raw output in your issue:
  ```bash
  sudo dmidecode -t 17
  ```
