# Phase 1: Foundation and Hardware Probing - Context

**Gathered:** 2026-09-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 1 establishes the core Cargo multi-crate workspace (`monkey-core`, `monkey-cli`), userspace USB HID transport abstraction with `hidapi`, strict zero-copy binary packet codec, safe dual-interface probing, read-only capability detection, and non-flash hardware smoke verification (`monkey info`, `monkey probe`). Live LCD streaming and RGB manipulation belong to subsequent phases.
</domain>

<decisions>
## Implementation Decisions

### 1. Dual-Interface Probing Strategy
- **D-01:** Device identification matches the Monka 3075 Pro / RKGK890 VID/PID pair (`0x05AC:0x024F` / `1452:591`).
- **D-02:** Interfaces are disambiguated by `(UsagePage, Usage)` tuple as primary mechanism on macOS (`IOHIDManager`), with interface number fallback for Linux/Windows:
  - **Interface A (Vendor Bulk Pipe):** `UsagePage = 0xFF68`, `Usage = 0x0061` (`PrimaryUsagePage = 65384, PrimaryUsage = 97`). Max Input: 64B, Max Output: 4096B. Uses unnumbered Report ID 0 (macOS requires prepending `0x00` buffer prefix in userspace, stripped by kernel on wire).
  - **Interface B (Standard/Vendor Control):** `UsagePage = 0x000C`, `Usage = 0x0001` (`PrimaryUsagePage = 12, PrimaryUsage = 1`, Elements include `UsagePage = 0xFFFF`). Max Input: 16B, Max Output: 1B, Max Feature: 1B/64B. Handles vendor feature reports and control commands.
- **D-03:** Both interfaces are opened with `macos-shared-device` feature flag (`hid_darwin_set_open_exclusive(0)`) to prevent composite interface lockouts against macOS system drivers.

### 2. Transport Trait & Mocking Architecture
- **D-04:** Explicit synchronous trait definition in `crates/monkey-core`:
  ```rust
  pub trait Transport: Send {
      fn write_bulk(&mut self, report_id: u8, data: &[u8]) -> Result<usize, TransportError>;
      fn send_feature_report(&mut self, data: &[u8]) -> Result<(), TransportError>;
      fn get_feature_report(&mut self, report_id: u8, buf: &mut [u8]) -> Result<usize, TransportError>;
      fn read_input_report(&mut self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, TransportError>;
  }
  ```
- **D-05:** `HidTransport` implements `Transport` wrapping `hidapi::HidDevice`.
- **D-06:** `MockTransport` provides in-memory recording and canned responses for CI without physical hardware access.

### 3. Zero-Copy Packet Codecs & Layout Safety
- **D-07:** Packet structures derive `zerocopy::FromBytes`, `zerocopy::IntoBytes`, `zerocopy::Immutable`, and `zerocopy::KnownLayout`.
- **D-08:** Explicit little-endian primitives (`U16<LittleEndian>`, `U32<LittleEndian>`) to prevent alignment traps and endianness bugs across Apple Silicon (ARM64) and x86_64.
- **D-09:** Strict header validation: fail-fast on invalid magic bytes, unexpected opcodes, or corrupted checksums before dispatching to handlers.
- **D-10:** Fixed-size structures: 64-byte feature reports and 4096-byte bulk transfer chunk structs with zero heap allocations during serialization/deserialization.

### 4. Hardware Safety Gates & Smoke Verification
- **D-11:** Phase 1 CLI commands are strictly constrained to non-destructive queries:
  - `monkey probe`: Detects device presence, enumerates composite interfaces, validates Report ID capabilities, and prints hardware details.
  - `monkey info`: Queries device firmware version, protocol compatibility, and active connection mode.
  - `--json` flag on all inspection commands for machine-readable JSON output.
- **D-12:** Zero flash commits allowed in Phase 1: no DFU/ISP mode triggers, no flash sector erase opcodes, no speculative writes. Only read-only descriptors and benign ping/version queries.

### Claude's Discretion
- Exact CLI terminal styling and color palette with `clap` and `tracing-subscriber`.
- Internal error variant hierarchy under `monkey_core::error::*`.
- Development environment detection and actionable setup instructions when Rust toolchain is absent.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### USB Architecture & Protocol
- `research/prior_art_protocol.md` — Packet formats, dual-interface specification (`0xFF68:0x0061` bulk vs `0x000C:0x0001`/`0xFFFF` control), report sizes.
- `research/protocol_notes.md` — Opcode tables, feature report layout, CRC checksum algorithms.
- `research/device_info.json` — Hardware descriptors, VID (`0x05AC`), PID (`0x024F`), OEM vendor identification.
- `research/capture_plan.md` — Safe inspection strategies and hardware validation boundaries.

### Project Requirements & Architecture
- `.planning/research/STACK.md` — Crate versions (`hidapi 2.6.7`, `zerocopy 0.8.57`, `clap 4.6.6`), macOS entitlements, and transport patterns.
- `.planning/research/ARCHITECTURE.md` — Core crate boundaries (`monkey-core` vs `monkey-cli`), synchronization, and safety model.
- `.planning/research/PITFALLS.md` — Avoid exclusive USB locks, Report ID 0 byte offsets, and unaligned memory access.
- `.planning/REQUIREMENTS.md` — Requirements for Phase 1: PROB-01, PROB-02, PROB-03, PROB-04, PROB-05.
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `research/device_info.json`: Accurate VID/PID mapping for Monka 3075 Pro / RKGK890.
- `research/layout_81keys.json`: Full physical key matrix mapping for future phases.
- Real hardware available and actively connected: `ioreg` verified VID `0x05AC` (1452), PID `0x024F` (591) with interfaces `0xFF68:0x0061` and `0x000C:0x0001`.

### Established Patterns
- Monorepo workspace layout: `crates/monkey-core` (driver library) and `crates/monkey-cli` (binary CLI).
- macOS userspace access via `IOHIDManager` without detaching Apple default keyboard kernel drivers.

### Integration Points
- `crates/monkey-core/src/transport`: Base transport abstraction layer and `hidapi` implementation.
- `crates/monkey-core/src/protocol`: Zero-copy packet encoders/decoders and checksum verifiers.
- `crates/monkey-cli/src/main.rs`: CLI entrypoint providing `probe` and `info` commands.
</code_context>

<specifics>
## Specific Ideas
- Clean, fast CLI tool startup (<10ms).
- Rich diagnostics in `monkey probe` showing exact IOHID descriptor attributes matching our `ioreg` verification.
</specifics>

<deferred>
## Deferred Ideas
- Dynamic frame buffering and 128x128 RGB565 LCD streaming — Phase 2.
- Interactive RGB backlight animations and live matrix editing — Phase 3.
- Background worker thread with crossbeam-channel message loop — Phase 2/3.
- Tauri v2 Desktop GUI integration — Post-M1.
</deferred>

---
*Phase: 01-foundation-and-hardware-probing*
*Context gathered: 2026-09-13*
