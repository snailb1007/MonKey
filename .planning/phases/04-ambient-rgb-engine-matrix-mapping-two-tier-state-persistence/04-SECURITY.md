---
phase: "04"
slug: ambient-rgb-engine-matrix-mapping-two-tier-state-persistence
status: verified
threats_open: 0
asvs_level: 1
created: "2026-09-14"
---

# Phase 04 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail for Ambient RGB Engine, Matrix Mapping & Two-Tier State Persistence.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| User CLI Arguments → Parameter Codecs | Command-line flags parsed via `clap` into lighting parameters | Mode, color, brightness, speed, direction, consent flags |
| Application Memory → Volatile RAM vs SPI Flash | Two-tier state management deciding memory preview vs non-volatile commit | Volatile RAM reports (`04 13`) vs 4-packet flash commit transaction |
| Profile Filesystem → Host File Storage | JSON profile backup and restore operations | Profile JSON files with schema version and lighting configuration |
| Host Driver → USB HID Composite Interface | 64-byte feature reports (`IOHIDDeviceSetReport`) sent over Interface B | Feature report packets with command opcodes and payload bytes |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-04-01 | Denial of Service / Hardware Wear | `crates/monkey-core/src/rgb/manager.rs`, `crates/monkey-core/src/protocol/safety.rs` | high | mitigate | Two-tier state management: `RgbManager::apply_preview` issues single `04 13` feature report with `WriteMode::RamPreview` (up to 30Hz, zero flash wear). Flash commits (`--commit`) are executed through 4-packet transaction sequence with mandatory 500ms debounce interval enforced by `SafetyRails::check_write_allowed`. | closed |
| T-04-02 | Device Damage / Tampering | `crates/monkey-core/src/rgb/manager.rs` | high | mitigate | Low-battery wireless safety gate: `RgbManager::apply_commit` strictly blocks flash writes when battery is below 20% on wireless connections (`MonkeyError::SafetyViolation`), preventing MCU brownout corruption during flash sector operations unless overridden by `--force`. | closed |
| T-04-03 | Elevation of Privilege / Device Mutation | `crates/monkey-cli/src/commands/rgb.rs` | high | mitigate | Mandatory hardware write-consent gate: `resolve_transport` enforces `--allow-hardware-writes` for any physical device mutations, aborting safely if ungranted; `--mock` allows full testability without touching hardware. | closed |
| T-04-04 | Tampering / Denial of Service | `crates/monkey-core/src/rgb/mode.rs`, `crates/monkey-core/src/rgb/codec.rs` | medium | mitigate | Input validation and packet bounds: `LightingConfig::validate` bounds brightness and speed (0..=100); codec normalizes to hardware scale (1..=5), formats fixed 64-byte `FeatureReportPacket`, and validates `[0xAA, 0x55]` marker on decode. | closed |
| T-04-05 | Information Disclosure / Tampering | `crates/monkey-core/src/rgb/profile.rs` | medium | mitigate | Profile schema and filesystem safety: `RgbProfile::from_json` validates `schema_version <= 1` and validates deserialized `LightingConfig`; `save_to_file` creates directory structure safely and serializes strict JSON. | closed |

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
| 2026-09-14 | 5 | 5 | 0 | gsd-security-auditor |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-14
