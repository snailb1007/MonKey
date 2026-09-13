---
phase: 02
slug: protocol-codecs-transaction-safety-rails-benchmark-harness
status: draft
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 6
asvs_level: 1
created: 2026-09-13
---

# Phase 02 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| CLI / User Input | CLI flags, subcommands, arguments, and payload files | Subcommand parameters, timing values, write authorization flags |
| Core Dispatch / Channel | Asynchronous thread boundaries between application and hardware worker | In-memory message envelopes, transaction handles, framing buffers |
| Transport / Kernel HID | Userspace `IOHIDManager` / `hidapi` boundary to OS USB stack | 64-byte feature reports, 4096-byte vendor bulk reports |
| Hardware / MCU | Physical Monka 3075 Pro microcontroller, SPI NOR flash, and display RAM | Raw USB HID reports, vendor opcodes, state commands |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| S-01 | Spoofing | `device.rs` / enumeration | high | mitigate | Strict VID/PID/serial verification rejecting ISP bootloader PID `0x7140` | open |
| S-02 | Spoofing | `device.rs` / usage matching | medium | mitigate | Interface-role allowlists (`0xFFFF`, `0xFF68`) with ambiguous path rejection | closed |
| T-01 | Tampering | `protocol::types` | high | mitigate | Zerocopy exact-length parsing and `0x04` magic / `0xAA55` marker header validation | closed |
| T-02 | Tampering | `protocol::safety` | high | mitigate | Default-deny opcode whitelist (`0x18`, `0x13`, `0x20`, `0x02`, `0xF0`, `0xF5`) and write-mode checking | closed |
| T-03 | Tampering | `transport` / raw writes | critical | mitigate | Mandatory safety gate pipeline preventing raw unvalidated transport write bypass | open |
| T-04 | Tampering | `protocol::crc` | high | mitigate | Frame/packet CRC16/additive checksum calculation and verification at boundary | open |
| T-05 | Tampering | `protocol::transaction` / bulk writes | high | mitigate | Core-level interface capability check and hardware write authorization gate on bulk transfers | open |
| T-06 | Elevation of Privilege | `protocol::transaction` / lifecycle | high | mitigate | Full transaction lifecycle (`Start` -> `Execute` -> `Commit` -> `End`), RAII `0xF0` cleanup, signal handler, and `monkey reset` | open |
| T-07 | Tampering | `protocol::safety` / commit | high | mitigate | Bind flash commit (`0x02`) to active transaction state and verified RAM preview | open |
| T-08 | Tampering | `protocol::transaction` / pacing | medium | mitigate | Minimum hardware pacing delay floor to prevent bus collisions or MCU overflow | open — below high threshold (non-blocking) |
| D-01 | Denial of Service | `protocol::framing` / queue | medium | mitigate | Checked arithmetic on chunk reassembly lengths and payload size caps | open — below high threshold (non-blocking) |
| D-02 | Denial of Service | `protocol::channel` | medium | mitigate | Transport/worker timeouts and cancellation tokens to prevent indefinite thread hangs | open — below high threshold (non-blocking) |
| D-03 | Denial of Service | `protocol::transaction` | medium | mitigate | Strict chunk write verification failing fast on partial or erroneous bulk transfers | closed |
| E-02 | Elevation of Privilege | `commands::probe` | high | mitigate | Non-destructive read-only feature report queries during probing | closed |
| E-03a | Elevation of Privilege | `commands::bench` | medium | mitigate | Require explicit `--allow-hardware-writes` flag before synthetic bulk streaming to hardware | closed |
| E-03 | Elevation of Privilege | `commands::probe` | medium | mitigate | Ground `read_only_verified` in verified read paths rather than descriptor fallbacks | open — below high threshold (non-blocking) |
| R-01 | Repudiation | `protocol::safety` | low | mitigate | Security event audit trail for blocked opcodes and invalid commit attempts | open — below high threshold (non-blocking) |
| I-01 | Information Disclosure | `commands::info` | low | mitigate | Redact hardware serial numbers and physical device paths in default CLI output | open — below high threshold (non-blocking) |

*Status: open · closed · open — below high threshold (non-blocking)*  
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on (high) count toward threats_open*  
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

No accepted risks.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-13 | 18 | 6 | 12 (6 blocking, 6 non-blocking) | gsd-security-auditor (retroactive STRIDE) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [ ] `threats_open: 0` confirmed
- [ ] `status: verified` set in frontmatter

**Approval:** pending (blocked by 6 open threats at or above high threshold)
