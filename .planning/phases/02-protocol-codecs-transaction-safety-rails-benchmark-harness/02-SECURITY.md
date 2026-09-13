# Security Verification: Phase 02

## Threat Model Verification Matrix

| Threat ID | Threat Category | Target / Component | Severity | Mitigation Strategy | Verification Result | Status |
|---|---|---|---|---|---|---|
| T-01 | Elevation of Privilege | macOS App Sandbox / IOKit | critical | Use `IOHIDManager` via `hidapi` (`macos-shared-device`); avoid DriverKit | Verified in Phase 1 via `hidapi` bindings | closed |
| T-02 | Denial of Service | Flash wear out | critical | RAM preview for live tuning; 500ms flash debounce | Verified: debounce logic implemented and tested in `safety.rs` | closed |
| T-03 | Tampering | `transport` / raw writes | critical | Mandatory safety gate pipeline (`&SafetyRails` parameter on write methods) preventing raw unvalidated transport write bypass | Verified: `Transport::write_bulk` and `send_feature_report` enforce `SafetyRails::validate_hardware_write_allowed`; test in `mock_transport_test.rs` | closed |
| S-01 | Tampering / Destruction | Bootloader ISP mode | high | ISP bootloader PID `0x7140` explicit reject guard in device enumeration and `open_device_path` | Verified: `is_isp_bootloader` and `open_device_path` reject ISP PIDs; test in `device_discovery_test.rs` | closed |
| T-04 | Tampering | Protocol framing / corrupted chunks | high | Zero-copy validation, fixed chunk bounds (4096-byte), CRC-16 checksum engine | Verified: `calculate_chunks_crc16` and `verify_chunks_crc16` across chunk boundaries; tests in `protocol_codecs_test.rs` | closed |
| T-05 | Tampering | Bulk OUT pipe (`0xFF68`) | high | Hardware write authorization (`hardware_writes_allowed`) enforced in `stream_bulk_chunks` and transport | Verified: bulk transfers rejected unless authorized; tested in `transaction_safety_test.rs` | closed |
| T-06 | Denial of Service | Device freeze on aborted transfer | high | TransactionGuard RAII abort frame dispatch, active state tracking, emergency `monkey reset` | Verified: abort cleanup and lifecycle tracking verified in `transaction_safety_test.rs` | closed |
| T-07 | Tampering | Flash commit (`0x02`) | high | Bound commit to active transaction state and verified RAM preview | Verified: `SafetyRails::validate_write` rejects `0x02` unless `active_transaction` and `verified_ram_preview` are true; tested in `transaction_safety_test.rs` | closed |
| D-01 | Denial of Service | Worker thread backlog / out-of-order | medium | Bounded channels (`crossbeam-channel`, cap 32), single worker thread ownership | Verified: `HardwareChannel` uses bounded channel and sequential worker loop | closed |
| D-02 | Denial of Service | Transport timeout handling | medium | Timeout guards on transport read/write operations | Pending dedicated hardware timeout validation | open |
| D-03 | Denial of Service | Packet pacing / MCU buffer overflow | medium | Configurable inter-packet delay (10-25ms) | Verified: `inter_packet_delay` and `inter_chunk_delay` in TransactionManager | closed |
| E-01 | Elevation of Privilege | Unauthorized device opening | medium | Path verification and interface classification | Verified: `InterfaceRole` classification per descriptor | closed |
| E-02 | Elevation of Privilege | Accidental hardware mutation | medium | Explicit `--allow-hardware-writes` flag requirement | Verified: CLI flags and library-level `SafetyRails` checks | closed |
| I-01 | Information Disclosure | USB packet sniffing | low | Unencrypted USB HID communications | Accepted risk: local USB hardware bus limitation | accepted |
| I-02 | Information Disclosure | Device memory readback | low | Read-only access control on sensitive feature reports | Verified: probe command enforces read-only query (D-12) | closed |
| R-01 | Repudiation | Unlogged configuration changes | low | Structured tracing for all command and bulk dispatches | Pending log audit infrastructure | open |
| R-02 | Repudiation | Hardware status divergence | low | State readback after transaction completion | Verified: state readback supported via feature reports | closed |
| S-02 | Spoofing | Rogue USB device impersonation | low | VID/PID pair matching and descriptor layout check | Verified: VID `0x05AC`, PID `0x024F`, and descriptor role checks | closed |

---

### Audit Summary: Phase 02 (Updated)

| Metric | Count |
|---|---|
| Threats found | 18 |
| Closed | 15 |
| Accepted | 1 |
| Open (blocking — severity ≥ high) | 0 |
| Open (non-blocking — severity < high) | 2 |

All blocking threats (severity ≥ high) have been mitigated, tested, and closed.
