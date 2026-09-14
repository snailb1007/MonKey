# Phase 5: Production Hardening, Packaging & Release Readiness — Research

## Context & Architecture
Phase 5 is the final phase of Milestone v1.0. It delivers production hardening, system diagnostic tools (`monkey doctor`), shell autocompletions (`monkey completions`), standardized POSIX exit code mappings, and security/license compliance verification (`cargo-deny`).

## Diagnostic Architecture (`monkey doctor`)
Pre-flight diagnostics allow users and automated environments to assess whether the keyboard, OS permissions, and USB stack are ready for operation without guessing why a command might fail.

### Diagnostic Matrix
1. **Host OS & Platform**:
   - macOS (Darwin): Report OS version; verify whether sandboxed or running standalone.
   - Linux: Verify `/dev/hidraw*` presence and warn if non-root without udev rules.
   - Windows: Check Windows version and HID driver status.
2. **HID Subsystem**:
   - Initialize `hidapi` (`IOHIDManager` on macOS). Confirm no initialization faults.
3. **Monka 3075 Pro Detection**:
   - Enumerate USB devices matching VID `0x05AC`, PID `0x024F` or known HFD wireless dongle IDs.
   - Categorize whether connected via wired USB or 2.4GHz wireless dongle.
4. **Interface A Accessibility (Bulk LCD Pipe)**:
   - Vendor Usage Page `0xFF68`, Usage `0x61`.
   - Verify report buffer allocation and handle opening without permission prompts.
5. **Interface B Accessibility (Config & RGB Pipe)**:
   - Vendor Usage Page `0xFFFF`, Usage `0x0001`.
   - On macOS, co-resides with Consumer Control (`0x0C`) and Mouse (`0x01`). Requires non-exclusive shared device open (`hid_darwin_set_open_exclusive(0)`).
   - Test feature report read/write availability. If permission denied on macOS, provide specific guidance for macOS Input Monitoring (`System Settings -> Privacy & Security -> Input Monitoring`).
6. **Remediation Engine**:
   - Generates actionable instructions for each failed or warned diagnostic check.
   - Example macOS: `"Permission denied on Interface B: Grant Input Monitoring permission to your terminal emulator (e.g. Terminal, iTerm2, Kitty, Alacritty) under System Settings > Privacy & Security > Input Monitoring."`
   - Example Linux: `"Add udev rule: SUBSYSTEM==\"hidraw\", ATTRS{idVendor}==\"05ac\", ATTRS{idProduct}==\"024f\", MODE=\"0666\" to /etc/udev/rules.d/99-monka.rules and run 'udevadm control --reload-rules && udevadm trigger'."`
   - Example No Device: `"Check physical USB-C cable connection or ensure 2.4GHz receiver is plugged in. Check keyboard mode toggle on the left side."`

## Shell Completions (`monkey completions <shell>`)
- Use `clap_complete` to generate scripts dynamically from the top-level Clap CLI definition.
- Supported shells:
  - `bash`
  - `zsh`
  - `fish`
  - `elvish`
  - `powershell`
- Command format:
  `monkey completions <shell>`
- Outputs script directly to stdout for shell sourcing:
  `monkey completions zsh > ~/.zsh/completion/_monkey`
  `source <(monkey completions bash)`

## Standardized POSIX Exit Codes
Currently, errors propagate via `anyhow::Result<()>`, which exits with code 1 for all failures.
For production CLI ergonomics and scripting reliability, we define domain exit codes:
- `EXIT_SUCCESS` (`0`): Command completed successfully.
- `EXIT_GENERAL` (`1`): Unhandled error or internal failure.
- `EXIT_USAGE` (`2`): Invalid arguments, bad CLI flags, or schema validation error.
- `EXIT_NO_DEVICE` (`3`): Monka 3075 Pro keyboard / receiver not found or disconnected.
- `EXIT_PERMISSION` (`4`): OS permission denied (e.g. macOS TCC Input Monitoring lockout or Linux udev permissions).
- `EXIT_BLOCKED` (`5`): Hardware safety gate violation (e.g. write attempted without `--allow-hardware-writes`, blocked bootloader PID, or wireless low-battery lockout).

In `crates/monkey-cli/src/main.rs`:
Catch the top-level `Result<()>`. If `Err(e)`:
Classify error using downcasting to domain error types (`MonkeyError`, `TransportError`) or matching known error patterns, print a clean error box or message to stderr, and call `std::process::exit(code)`.

## Release Hardening & `cargo-deny`
- Ensure zero errors and zero warnings on `cargo deny check`.
- Configure `deny.toml` to:
  - Allow `MIT`, `Apache-2.0`, `BSD-3-Clause`, `Unicode-3.0`.
  - Ban wildcards, and skip known transitive dual-version dependencies (`syn`, `miniz_oxide`).
  - Enforce `allow-workspace = true`.
