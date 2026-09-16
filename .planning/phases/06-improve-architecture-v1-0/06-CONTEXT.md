# Phase 6: Improve Architecture v1.0 - Context

**Gathered:** 2026-09-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 6 refactors the architectural boundaries and hardware interaction patterns of MonKey v1.0. It unifies repeated discovery and open choreography across all CLI commands (`info`, `probe`, `lcd`, `bench`, `rgb`) and the diagnostic engine (`doctor`). It centralizes `SafetyRails` ownership, replaces ad-hoc test seams and production environment variable hacks with a cohesive `MonkaDevice` abstraction in `monkey-core`, and preserves true hardware interface variations (`InterfacePolicy`) while maintaining `Transport` as the single test seam.

</domain>

<decisions>
## Implementation Decisions

### 1. Unified Device Abstraction & Seam Architecture
- **D-01 (Single Seam Principle):** Keep `Transport` as the sole underlying I/O abstraction seam. `MonkaDevice` is a concrete struct in `monkey-core::device`, NOT a trait or duplicate mock seam. For tests and headless CLI flags (`--mock`), `MonkaDevice` provides `MonkaDevice::from_transport(Box<dyn Transport>)`, reusing the existing `MockTransport`.
  — **Reversibility:** costly — changes the primary hardware handle consumed by CLI commands and test suites.
- **D-02 (MonkaDevice Lifecycle & InterfacePolicy):**
  - Expose `MonkaDevice::open(policy: InterfacePolicy) -> Result<MonkaDevice, OpenError>` for standard commands.
  - Interface selection is recognized as legitimate hardware variation rather than accidental duplication. `InterfacePolicy` enum encodes variations:
    - `PreferB`: Interface B first, fallback to Interface A (used by `rgb`).
    - `RequireA`: Interface A only; hard fail if missing (used by `lcd`).
    - `RequireB`: Interface B only (used by `probe`).
    - `BulkFirst`: Interface A then Interface B (used by `bench` when running bulk transfers).
    - `ControlFirst`: Interface B then Interface A (used by `bench` when running feature/transaction transfers).
    - `Any`: Open whatever valid Monka interface is available.
  — **Reversibility:** costly — standardizes device acquisition across all 5 command suites.

### 2. Diagnostic & Error Granularity
- **D-03 (Layered OpenError & Doctor Diagnostics):**
  - `OpenError` is structured into granular enum variants corresponding to initialization stages: `HidInit(TransportError)`, `NoDevice`, `InterfaceUnavailable(InterfaceRole)`, `InterfaceOpenFailed(InterfaceRole, TransportError)`.
  - Consumer #6 (`monkey doctor`) cannot use a fail-fast `MonkaDevice::open()`. Instead, `MonkaDevice` provides `MonkaDevice::diagnose() -> DeviceDiagnostics` (or stage-by-stage probing inspection) so `doctor` records Pass/Fail/Warn with distinct remediations without early-returning.
  — **Reversibility:** costly — shapes diagnostic reporting and error recovery contracts in `monkey-core` and CLI.

### 3. Safety Rails Ownership & Deep Module Operations
- **D-04 (SafetyRails Ownership & Deep Module Operations):**
  - `SafetyRails` is owned internally by `MonkaDevice`. Callers no longer instantiate or thread `SafetyRails` manually across command functions.
  - **Forcing function:** Because `transport` requires `&mut` and `safety` requires `&`, exposing both accessors simultaneously (`device.transport_mut()` + `device.safety()`) violates Rust's borrow checker. Therefore, `MonkaDevice` acts as a deep module exposing cohesive operations rather than raw internals:
    - `device.stream_frame(&frame, config, on_progress)`
    - `device.apply_rgb_preview(&config)`
    - `device.apply_rgb_commit(&config, is_wireless, battery, force)`
    - `device.send_feature_command(&packet, write_mode)`
  - Refactor signatures of `LcdStreamer` and `RgbManager` (or delegate directly through `MonkaDevice`) to conform to this ownership model.
  — **Reversibility:** one-way — changes internal driver contracts and caller method signatures across `monkey-core` and `monkey-cli`.

### 4. Elimination of Test Seams and Env-Var Hacks
- **D-05 (Eliminate Test Seam Chaos & MONKEY_SIMULATE_EMPTY):**
  - Delete the production environment variable hacks: `probe.rs:201` and `info.rs:164` reading `std::env::var("MONKEY_SIMULATE_EMPTY")`.
  - Eliminate fragmented ad-hoc test seams across commands (`run_probe_with_transport`, `run_bench_with_transport`, `run_info_with_device_set`, `resolve_transport(mock: bool)`, and `stream_one(...)`).
  - Tests simulate empty device lists or hardware responses via `MonkaDevice::from_transport(Box::new(MockTransport::new()))` or deterministic mock device sets.
  — **Reversibility:** reversible — cleans up test plumbing and dead production branches.

### 5. Blast Radius & Blast Management
- **D-06 (Risk Mitigation & Regression Safety):**
  - GitNexus impact analysis confirms `open_device_path` and `find_monka_device_sets` have `CRITICAL` upstream impact (18–20 affected symbols, 12–13 processes, depth-1 breakages in 5 direct callers).
  - GitNexus index was refreshed (`node .gitnexus/run.cjs analyze --index-only`) ensuring all 6 consumers (`bench`, `doctor`, `info`, `lcd`, `probe`, `rgb`) are indexed.
  - Refactoring execution must be phased or atomic with strict verification: all 122 existing workspace unit/integration tests must pass, and `cargo clippy --all-targets -- -D warnings` must remain 100% clean.
  — **Reversibility:** reversible — quality enforcement.

### the agent's Discretion
- Internal layout and module location of `MonkaDevice` (e.g. `crates/monkey-core/src/device/mod.rs` or `crates/monkey-core/src/device.rs`).
- Exact enum naming for `OpenError` and `InterfacePolicy` provided the semantics match the decisions.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Architecture & Safety Specifications
- `docs/dev/architecture.md` — Core layered decoupling and interface isolation rules
- `docs/dev/hardware-safety.md` — Hardware safety rails, opcode whitelist, and flash wear mitigation
- `docs/dev/protocol-spec.md` — Packet formats and dual-interface assignments (`0xFF68` vs `0xFFFF`)

### Planning & Roadmap
- `.planning/ROADMAP.md` § Phase 6: Improve Architecture v1.0
- `.planning/REQUIREMENTS.md` § DISC-01..05, PROT-01..05, LCD-01..05, RGB-01..04, DIAG-01..02
- `.planning/phases/05-production-hardening-packaging-release-readiness/05-CONTEXT.md` — Pre-flight doctor and diagnostic check architecture

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/monkey-core/src/transport/mock.rs`: `MockTransport` with call recording, feature report mocking, error injection, and write assertion assertions (`assert_no_writes`).
- `crates/monkey-core/src/device.rs`: `find_monka_device_sets`, `group_monka_devices`, `open_device_path`, `MonkaDeviceSet`, `DiscoveredDevice`.
- `crates/monkey-core/src/protocol/safety.rs`: `SafetyRails` with hardware write permission checks and opcode validation.
- `crates/monkey-core/src/doctor.rs`: `DiagnosticCheck`, `CheckStatus`, `DoctorReport`.

### Established Patterns
- `hidapi 2.6.7`: `pub struct HidDevice { inner: Box<dyn HidDeviceBackend> }` has no lifetime parameters and supports multiple `HidApi` instances. `MonkaDevice` can safely hold device handles and the `HidApi` context without self-referential borrowing conflicts.
- Deep module interface: Rather than exposing raw mutable references (`&mut transport` and `&safety`) which borrow check concurrently, `MonkaDevice` encapsulates operations.

### Integration Points
- `crates/monkey-cli/src/commands/info.rs`: Replace manual HID scan with `MonkaDevice` discovery.
- `crates/monkey-cli/src/commands/probe.rs`: Replace custom `run_probe_with_writer` discovery and env var with `MonkaDevice::open(InterfacePolicy::RequireB)`.
- `crates/monkey-cli/src/commands/lcd.rs`: Replace `open_lcd_transport()` with `MonkaDevice::open(InterfacePolicy::RequireA)`.
- `crates/monkey-cli/src/commands/bench.rs`: Replace manual interface branching with `MonkaDevice::open(policy)`.
- `crates/monkey-cli/src/commands/rgb.rs`: Replace `resolve_transport()` with `MonkaDevice::open(InterfacePolicy::PreferB)`.
- `crates/monkey-core/src/doctor.rs`: Leverage `MonkaDevice::diagnose()` for multi-step platform and interface accessibility reporting.

</code_context>

<specifics>
## Specific Ideas

- Eradicate `MONKEY_SIMULATE_EMPTY` from `crates/monkey-cli/src/commands/probe.rs` and `crates/monkey-cli/src/commands/info.rs`.
- Replace 5 heterogeneous ad-hoc test seams (`run_probe_with_transport`, `run_bench_with_transport`, `run_info_with_device_set`, `resolve_transport`, `stream_one`) with unified `MonkaDevice::from_transport(Box::new(MockTransport::new()))`.
- Preserve interface variation via `InterfacePolicy` enum rather than flattening device logic.
- Doctor check receives non-early-returning stage diagnostic inspection (`MonkaDevice::diagnose()`).

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed strictly within the Phase 6 architecture refactoring scope.

</deferred>

---

*Phase: 06-improve-architecture-v1-0*
*Context gathered: 2026-09-16*
