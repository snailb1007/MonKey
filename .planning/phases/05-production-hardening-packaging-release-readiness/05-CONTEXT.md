# Phase 5 Context: Production Hardening, Packaging & Release Readiness

## Overview
Phase 5 hardens the MonKey driver ecosystem for end-user distribution, diagnostic visibility, and release readiness. It delivers:
1. `monkey doctor`: A pre-flight diagnostic utility inspecting OS platform, USB HID accessibility, Interface A/B permissions, and providing actionable remediation guidance for macOS TCC / Linux udev issues.
2. `monkey completions <shell>`: Shell auto-completion generation for `bash`, `zsh`, `fish`, `elvish`, and `powershell` using `clap_complete`.
3. Standardized POSIX exit codes & actionable error formatting: Replacing unformatted crashes with clear, categorized error diagnostics and standard POSIX exit codes.
4. Security & license compliance: Complete `cargo-deny` validation for advisories, licenses, bans, and sources across the entire workspace.

## Decisions

### 1. `monkey doctor` Architecture
- **Checks Performed**:
  - `Platform`: OS detection (macOS Darwin version, Linux kernel, Windows NT).
  - `HID Subsystem`: `hidapi` initialization and backend health.
  - `Device Presence`: Search for Shenzhen HFD / Monka 3075 Pro (`0x05AC:0x024F`).
  - `Interface A (Bulk Pipe)`: Open and probe Interface A (`0xFF68:0x61`). On macOS this requires no special TCC permissions.
  - `Interface B (Config Pipe)`: Open and probe Interface B (`0xFFFF:0x0001`). If access fails on macOS, detect possible TCC Input Monitoring restriction and provide exact remediation steps (`System Settings -> Privacy & Security -> Input Monitoring`).
  - `Hardware Write Permission`: Check if safety gates permit hardware access.
- **Output**:
  - Human mode: Color-coded check items (`✓ PASS`, `⚠ WARN`, `✗ FAIL`), summary tally, and actionable remediation section.
  - JSON mode (`--json`): Structured machine-readable schema for automated setups and future GUI consumption.
  - Mock mode (`--mock`): Deterministic mock environment passing all checks for testing and CI.

### 2. Shell Completions
- Adopt `clap_complete` crate in `monkey-cli`.
- Subcommand `monkey completions <shell>` where shell is an enum: `bash`, `zsh`, `fish`, `elvish`, `powershell`.
- Generates completions to `stdout`.

### 3. Standardized POSIX Exit Codes
- Define domain exit codes in `monkey_cli::error`:
  - `0`: Success
  - `1`: General application error
  - `2`: Command line usage / validation error
  - `3`: Device not found / communication timeout
  - `4`: Permission denied / OS security lockout
  - `5`: Hardware safety gate rejection / unconsented write blocked
- Implement top-level error mapping in `main.rs` that catches `anyhow::Error`, classifies the root cause, prints a user-friendly error block to `stderr`, and terminates with the designated status code.

### 4. License & Vulnerability Auditing
- Verify workspace against `deny.toml` via `cargo deny check`.
- Ensure all dependencies conform to `MIT`, `Apache-2.0`, `BSD-3-Clause`, or `Unicode-3.0`.
- Zero security advisories or unmaintained package warnings.
