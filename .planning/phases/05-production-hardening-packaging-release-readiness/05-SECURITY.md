---
phase: "05"
slug: production-hardening-packaging-release-readiness
status: verified
threats_open: 0
asvs_level: 1
created: "2026-09-14"
---

# Phase 05 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail for Production Hardening, Packaging & Release Readiness.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| CLI Arguments & Shell Invocation → Subcommand Handlers | User invoking `monkey doctor`, `monkey completions <shell>`, or options | CLI arguments, `--mock`, `--json`, `ShellChoice` |
| Diagnostics Probe Engine → Host OS Subsystem & USB HID | Probing OS platform, architecture, and opening Interface A/B paths | `init_hidapi()`, device enumeration, `open_device_path` |
| Error Classification Subsystem → POSIX Exit Codes & Stderr | Application error chain traversal and process exit | `anyhow::Error` cause chain, POSIX exit status, diagnostic strings |
| Dependency Supply Chain → Cargo Build & CI Packaging | Third-party crate dependencies, path dependencies, licenses | RustSec advisories, crate licenses, source registries |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-05-01 | Spoofing | `crates/monkey-core/src/doctor.rs`, `crates/monkey-core/src/device.rs` | medium | mitigate | Synthetic reports explicitly demarcate mock environment across all checks; real enumeration strictly binds to `MONKA_VID` (`0x05AC`) and `MONKA_PID` (`0x024F`). | closed |
| T-05-02 | Tampering | `crates/monkey-cli/src/commands/completions.rs` | high | mitigate | Shell completion generation utilizes `clap::ValueEnum` mapping strictly to typed `clap_complete::Shell` targets without shell interpolation or command execution. | closed |
| T-05-03 | Repudiation | `crates/monkey-cli/src/error.rs`, `crates/monkey-cli/src/main.rs` | high | mitigate | POSIX exit code integrity: `classify_error` inspects error causes, mapping safety violations to `ExitCode::Blocked` (code 5), permission failures to code 4, and missing devices to code 3. | closed |
| T-05-04 | Information Disclosure | `crates/monkey-core/src/doctor.rs`, `crates/monkey-cli/src/main.rs` | medium | mitigate | Diagnostics and log data minimization: collects only static OS/arch strings, HID backend type, and USB VID/PID; disables logging target path prefixes (`with_target(false)`). | closed |
| T-05-05 | Denial of Service | `crates/monkey-core/src/doctor.rs`, `Cargo.toml` | high | mitigate | Non-exclusive HID probing: opens device paths via `macos-shared-device` (`hid_darwin_set_open_exclusive(0)`) and drops handles immediately without bus resets or unprompted writes. | closed |
| T-05-06 | Elevation of Privilege | `deny.toml`, `crates/monkey-cli/Cargo.toml` | high | mitigate | Automated supply chain verification: `cargo-deny` enforces 0 advisories, bans wildcard dependencies, validates license compatibility, and pins workspace path dependencies. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*  
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*  
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|

*No accepted risks.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-14 | 6 | 6 | 0 | gsd-security-auditor |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-14
