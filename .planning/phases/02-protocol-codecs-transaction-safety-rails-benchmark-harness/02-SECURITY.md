# Security Verification: Phase 02

**Phase:** `02-protocol-codecs-transaction-safety-rails-benchmark-harness`
**Audited Date:** 2026-09-13
**Status:** In Progress (Blocking threats remain)

---

## 1. Executive Summary

A comprehensive security verification was conducted against Phase 02 implementation artifacts (`crates/monkey-core`, `crates/monkey-cli`, and associated test harnesses) to evaluate threat mitigations under the project's zero-brick and hardware safety constraints.

- **Threats Identified:** 18
- **Closed Mitigations:** 6
- **Open Blocking Threats (severity ≥ high):** 6
- **Open Non-Blocking Threats (severity < high):** 6

---

## 2. Threat Verification Matrix

| Threat ID | STRIDE Category | Component / Target | Severity | Mitigation Strategy | Mitigation Status | Verification Evidence / Finding |
|---|---|---|---|---|---|---|
| **S-01** | Spoofing | Device Enumeration (`device.rs`) | high | Reject ISP Bootloader PIDs & enforce vendor VID | open | Enumeration checks `MONKA_VID` / `MONKA_PID` but `open_device_path` blindly opens any path passed by callers; known ISP bootloader PID `0x7140` is not explicitly blacklisted/rejected. |
| **S-02** | Spoofing | Transport / Mock | low | Verify mock isolation from real USB stack | closed | `MockTransport` is isolated in memory with no raw USB/IOKit calls; safely tested in unit tests. |
| **T-01** | Tampering | Feature Packet Framing (`framing.rs`, `types.rs`) | medium | Strict 64-byte framing & magic validation | closed | `FeatureReportPacket::parse_from_slice` enforces exact 64-byte length, magic `0x04`, and marker `[0xAA, 0x55]`. |
| **T-02** | Tampering | Bulk Data Transfer (`framing.rs`) | medium | Chunk size validation & boundary bounds | closed | `BulkTransferPlan` and `ChunkIterator` enforce bounded chunk sizes (<= 4096) and total chunk count < 256. |
| **T-03** | Tampering | Raw Transport API (`transport/mod.rs`) | critical | Transport abstraction bypass safety rails | open | `Transport` trait exposes public, unconstrained `write_bulk` and `send_feature_report` directly without passing through `SafetyRails`. |
| **T-04** | Tampering | Checksum Integrity (`crc.rs`) | high | Hardware checksum calculation across packets | open | `crc.rs` provides standalone CRC-16 calculation, but packets and transmission pipeline do not compute or enforce checksums across chunk boundaries. |
| **T-05** | Tampering | Bulk Write Pipeline (`transaction.rs`) | high | Gate bulk write on write authorization | open | `stream_bulk_chunks` in `TransactionManager` writes directly to `transport.write_bulk` without validating hardware write authorization flags. |
| **T-06** | Tampering | Transaction Lifecycle (`transaction.rs`) | high | Full transaction sequence & cleanup fallback | open | Missing full transaction lifecycle (`0x18` -> execute -> `0x02` -> `0xF0`), RAII cleanup frame on drop, and signal/cancellation interception. |
| **T-07** | Tampering | Flash Commit Binding (`safety.rs`) | high | Bind 0x02 commit to active verified state | open | Flash commit (`0x02`) is gated by debounce timing and mode matching, but is not bound to a verified preceding transaction or RAM preview. |
| **T-08** | Tampering | LCD Frame Size (`framing.rs`) | medium | Enforce exact 32,768 byte LCD framebuffer size | closed | `slice_lcd_frame` strictly checks `frame.len() == 32768`, rejecting any mismatched length. |
| **R-01** | Repudiation | Hardware Channel Audit (`channel.rs`) | low | Log and trace transmitted hardware frames | open | Frame transmissions through `HardwareChannel` lack tracing/telemetry logs for audit trails. |
| **I-01** | Info Disclosure | Memory Leakage in Packets (`types.rs`) | low | Zero-pad unused payload buffers | closed | `FeatureReportPacket` zero-initializes payload buffers and `ChunkIterator` zero-pads final trailing chunk data. |
| **I-02** | Info Disclosure | Uninitialized Struct Memory (`types.rs`) | low | `zerocopy` derive bounds | closed | Packets derive `FromBytes, IntoBytes, KnownLayout, Immutable`, eliminating struct padding/alignment leaks. |
| **D-01** | Denial of Service | Flash Wear Exhaustion (`safety.rs`) | high | Rate-limit and debounce flash write operations | closed | `SafetyRails::check_write_allowed` debounces `WriteMode::FlashCommit` (500ms default), preventing SPI flash burnout. |
| **D-02** | Denial of Service | Deadlock / Channel Blocking (`channel.rs`) | medium | Bounded channel size and non-blocking semantics | open | Hardware worker thread uses bounded channels but lacks timeout guards against unresponsive transports. |
| **D-03** | Denial of Service | Dropped Reports on Streaming (`transaction.rs`) | medium | Inter-chunk pacing delays | open | `inter_chunk_delay` is configurable, but default values in certain workflows omit needed hardware throttle delays. |
| **E-01** | Elevation of Privileges | Bootloader/DFU Opcode Execution (`safety.rs`) | critical | Strict opcode whitelist with default-deny | closed | `SafetyRails::validate_command` enforces a strict whitelist (`0x18, 0x13, 0x20, 0xF0, 0xF5, 0x02`), rejecting all DFU/ISP opcodes. |
| **E-02** | Elevation of Privileges | CLI Hardware Write Bypass (`bench.rs`) | medium | CLI safety flag enforcement | open | `monkey bench` checks `--allow-hardware-writes`, but low-level commands and libraries do not universally enforce caller privileges. |

---

## 3. Open Blocking Threats Details (Severity ≥ High)

### T-03: Raw Transport methods bypass SafetyRails
- **Severity:** Critical
- **Location:** `crates/monkey-core/src/transport/mod.rs:9-14`
- **Finding:** The `Transport` trait declares `write_bulk` and `send_feature_report` as public methods without any inherent safety gating. Any caller holding a reference to `dyn Transport` can send arbitrary payloads or opcodes directly to hardware without passing through `SafetyRails`.
- **Remediation Plan:** Wrap raw transport handles in a safe transport abstraction or seal direct write access so that writes are only permissible via `TransactionManager` or validated channel pipelines.

### S-01: Device enumeration accepts any path and lacks explicit ISP rejection
- **Severity:** High
- **Location:** `crates/monkey-core/src/device.rs:345`
- **Finding:** `open_device_path` opens any device path provided by caller without validating the VID/PID, and device enumeration does not explicitly verify and exclude known ISP bootloader PIDs (such as `0x7140` on Ajazz/HFD OEM boards).
- **Remediation Plan:** Validate device descriptors upon opening by path and add an explicit blocklist for known ISP bootloader PIDs (`0x7140`).

### T-04: Packet structs omit boundary checksum enforcement
- **Severity:** High
- **Location:** `crates/monkey-core/src/protocol/types.rs`, `framing.rs`
- **Finding:** Although `crc.rs` provides CRC-16 CCITT calculations, bulk chunks and feature reports do not embed or verify checksums across chunk boundaries during transfer.
- **Remediation Plan:** Integrate checksum validation into packet framing and verify chunk integrity before transport transmission.

### T-05: Bulk write path lacks write authorization enforcement
- **Severity:** High
- **Location:** `crates/monkey-core/src/protocol/transaction.rs:48-83`
- **Finding:** `TransactionManager::stream_bulk_chunks` executes raw writes on Interface A (`0xFF68`) without checking whether hardware writes are authorized or whether a valid LCD transaction has been opened.
- **Remediation Plan:** Add a safety gate check inside `stream_bulk_chunks` to verify write permission and transaction context before dispatching bulk chunks.

### T-06: Missing transaction lifecycle and cancellation cleanup
- **Severity:** High
- **Location:** `crates/monkey-core/src/protocol/transaction.rs`
- **Finding:** The full transaction lifecycle (`0x18 Start` -> payload transfer -> `0x02 Commit` -> `0xF0 End`) is not orchestrated as an atomic state machine, and RAII cleanup frames / OS signal handlers (`SIGINT`/`SIGTERM`) are not in place.
- **Remediation Plan:** Introduce an RAII `TransactionGuard` that guarantees sending `0xF0 EndTransaction` on drop or abort, and ensure reset fallbacks are exposed.

### T-07: Flash commit not bound to active transaction state
- **Severity:** High
- **Location:** `crates/monkey-core/src/protocol/safety.rs:55-65`
- **Finding:** Flash commit mode (`0x02`) is verified against the opcode and debounce window, but does not enforce that an active, verified RAM preview state or completed staging transaction precedes the commit.
- **Remediation Plan:** Track transaction state in `TransactionManager` and disallow `FlashCommit` unless preceded by successful verified RAM preview operations.
