# Phase 2: Protocol Codecs, Transaction Safety Rails & Benchmark Harness - Context

**Gathered:** 2026-09-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 2 establishes the core transaction safety, zero-copy protocol codecs, and benchmarking infrastructure for MonKey:
1. Zero-allocation packet codecs with `zerocopy` for 64-byte configuration feature reports and 4096-byte vendor HID OUT chunk reports.
2. Checksum engine supporting 16-bit additive/one's complement and CRC calculation/verification.
3. Default-deny `SafetyGate` enforcing capture-verified opcode whitelist (`04 18`, `04 13`, `04 20`, `04 02`, `04 F0`, `04 F5`) and ISP bootloader PID isolation (`0x7140`).
4. Session transaction lifecycle with best-effort RAII `TransactionGuard`, `ctrlc` signal handling emitting `04 F0` session cleanup, and standalone `monkey reset` command.
5. Serialized `CommandQueue` with inter-packet delay pacing (2ms–10ms) via dedicated worker thread and `crossbeam-channel`.
6. Diagnostics and performance verification through `monkey bench` CLI command measuring RTT, throughput, and chunk ACK timing with text/JSON outputs.

Display rendering/LCD streaming (Phase 3) and ambient RGB state management (Phase 4) are deferred to subsequent phases.
</domain>

<decisions>
## Implementation Decisions

### 1. SafetyGate Whitelist & ISP Bootloader Isolation (PROT-03)
- Default-deny opcode whitelist permitting only capture-verified packet prefixes:
  - `04 18`: Probe / capability query
  - `04 13`: RGB configuration
  - `04 20`: Key matrix / macro data
  - `04 02`: Flash commit command
  - `04 F0`: Session cleanup / reset frame
  - `04 F5`: State readback
- Bootloader PID isolation: Explicitly reject or quarantine PID `0x7140` (and any known ISP bootloader signatures) at device enumeration and transport opening to prevent accidental bricking.
- Safety gate check is mandatory in transport pipeline before sending any packet to hardware.

### 2. TransactionGuard & Session Cleanup (PROT-04)
- RAII `TransactionGuard` executes session cleanup frame (`04 F0`) on `Drop` unless explicitly marked completed/committed.
- Intercept OS termination signals (`ctrlc`) to ensure in-flight transactions trigger best-effort cleanup before exit.
- Provide standalone fallback CLI command: `monkey reset` to clear stuck states and emit `04 F0` cleanup frame manually.

### 3. CommandQueue & Inter-Packet Delay Pacing (PROT-05)
- Dedicated hardware worker thread owning `Transport` and communication channels via `crossbeam-channel`.
- Configurable delay profile enforcing 2ms–10ms pacing between successive packet transmissions, preventing USB microcontroller buffer overflow or bus collisions.
- Serialized execution guarantees single-flight transactions across CLI operations.

### 4. Zero-Copy Codecs & Checksum Engine (PROT-01, PROT-02)
- Zero-copy binary representations with `zerocopy` (`FromBytes`, `IntoBytes`, `Immutable`, `KnownLayout`) and endian-safe types (`U16<LittleEndian>`, etc.).
- 64-byte feature reports and 4096-byte chunk payloads with explicit validation.
- Modular checksum engine computing additive 16-bit sums, one's complement, and CRC16 implementations as required by HFD vendor protocols.

### 5. Benchmark Harness (`monkey bench`) (DIAG-01)
- CLI subcommand `monkey bench` with flags to measure:
  - Round-trip latency (RTT) for feature reports.
  - Chunk transfer throughput (KB/s).
  - Inter-packet ACK timing jitter.
- Output supports human-readable terminal formatting (progress, summary tables) and `--json` for automated regression testing.

### Claude's Discretion
- Exact layout of internal worker message enum (`CommandQueueMsg`).
- Test suite structure in `crates/monkey-core/tests/` and CLI integration tests.
- CLI argument structure for `monkey bench` iterations and packet sizes.
</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/monkey-core/src/transport`: `Transport`, `HidTransport`, `MockTransport`.
- `crates/monkey-core/src/protocol/types.rs`: Initial types and packet definitions from Phase 1.
- `crates/monkey-cli/src/output.rs`: Terminal output helper and JSON formatting.
- `crates/monkey-core/src/error.rs`: Typed error domain (`TransportError`, `ProtocolError`, etc.).

### Established Patterns
- Strongly typed domain errors with `thiserror`.
- Zero-allocation parsing where feasible using `zerocopy`.
- Mock-driven test coverage alongside hardware-capable CLI execution.

### Integration Points
- `crates/monkey-core/src/protocol/`: Codecs, SafetyGate, Checksum engine, and TransactionGuard.
- `crates/monkey-core/src/queue.rs` or `protocol/queue.rs`: Serialized `CommandQueue` and pacing worker thread.
- `crates/monkey-cli/src/commands/bench.rs`: `monkey bench` implementation.
- `crates/monkey-cli/src/commands/reset.rs`: `monkey reset` implementation.
</code_context>

<specifics>
## Specific Ideas
- Provide high benchmark precision using `std::time::Instant`.
- Ensure mock transport supports simulating latency and drop scenarios for safety testing.
</specifics>

<deferred>
## Deferred Ideas
- Phase 3: LCD frame slicing, Floyd-Steinberg dithering, and GIF playback.
- Phase 4: RGB matrix lighting persistence and low-battery lockout.
- Phase 5: `monkey doctor` USB entitlement and environment validation.
</deferred>
