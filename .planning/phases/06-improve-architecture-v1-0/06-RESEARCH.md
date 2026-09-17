# Phase 6: Improve Architecture v1.0 - Research

**Researched:** 2026-09-16  
**Domain:** MonKey Mechanical Keyboard Driver Architecture (Shenzhen HFD RKGK890 / Monka 3075 Pro)  
**Primary Requirement:** ARCH-01  
**Target File:** `.planning/phases/06-improve-architecture-v1-0/06-RESEARCH.md`  

---

## User Constraints

> *The following decisions are strictly copied from [06-CONTEXT.md](06-CONTEXT.md) and govern all architectural planning.*

### 1. Unified Device Abstraction & Seam Architecture
- **D-01 (Single Seam Principle):** Keep `Transport` as the sole underlying I/O abstraction seam. `MonkaDevice` is a concrete struct in `monkey-core::device`, NOT a trait or duplicate mock seam. For tests and headless CLI flags (`--mock`), `MonkaDevice` provides `MonkaDevice::from_transport(Box<dyn Transport>)`, reusing the existing `MockTransport`.
  — *Reversibility: costly — changes the primary hardware handle consumed by CLI commands and test suites.*
- **D-02 (MonkaDevice Lifecycle & InterfacePolicy):**
  - Expose `MonkaDevice::open(policy: InterfacePolicy) -> Result<MonkaDevice, OpenError>` for standard commands.
  - Interface selection is recognized as legitimate hardware variation rather than accidental duplication. `InterfacePolicy` enum encodes variations:
    - `PreferB`: Interface B first, fallback to Interface A (used by `rgb`).
    - `RequireA`: Interface A only; hard fail if missing (used by `lcd`).
    - `RequireB`: Interface B only (used by `probe`).
    - `BulkFirst`: Interface A then Interface B (used by `bench` when running bulk transfers).
    - `ControlFirst`: Interface B then Interface A (used by `bench` when running feature/transaction transfers).
    - `Any`: Open whatever valid Monka interface is available.
  — *Reversibility: costly — standardizes device acquisition across all 5 command suites.*

### 2. Diagnostic & Error Granularity
- **D-03 (Layered OpenError & Doctor Diagnostics):**
  - `OpenError` is structured into granular enum variants corresponding to initialization stages: `HidInit(TransportError)`, `NoDevice`, `InterfaceUnavailable(InterfaceRole)`, `InterfaceOpenFailed(InterfaceRole, TransportError)`.
  - Consumer #6 (`monkey doctor`) cannot use a fail-fast `MonkaDevice::open()`. Instead, `MonkaDevice` provides `MonkaDevice::diagnose() -> DeviceDiagnostics` (or stage-by-stage probing inspection) so `doctor` records Pass/Fail/Warn with distinct remediations without early-returning.
  — *Reversibility: costly — shapes diagnostic reporting and error recovery contracts in `monkey-core` and CLI.*

### 3. Safety Rails Ownership & Deep Module Operations
- **D-04 (SafetyRails Ownership & Deep Module Operations):**
  - `SafetyRails` is owned internally by `MonkaDevice`. Callers no longer instantiate or thread `SafetyRails` manually across command functions.
  - **Forcing function:** Because `transport` requires `&mut` and `safety` requires `&`, exposing both accessors simultaneously (`device.transport_mut()` + `device.safety()`) violates Rust's borrow checker. Therefore, `MonkaDevice` acts as a deep module exposing cohesive operations rather than raw internals:
    - `device.stream_frame(&frame, config, on_progress)`
    - `device.apply_rgb_preview(&config)`
    - `device.apply_rgb_commit(&config, is_wireless, battery, force)`
    - `device.send_feature_command(&packet, write_mode)`
  - Refactor signatures of `LcdStreamer` and `RgbManager` (or delegate directly through `MonkaDevice`) to conform to this ownership model.
  — *Reversibility: one-way — changes internal driver contracts and caller method signatures across `monkey-core` and `monkey-cli`.*

### 4. Seam Leakage & Pure Transport Guard Façade (Option B)
- **D-07 (Pure Transport Trait + SafeTransport Guard Façade):**
  - **The Leakage:** `Transport::write_bulk` and `Transport::send_feature_report` currently accept `&SafetyRails`. Low-level I/O adapters (`HidTransport`, `MockTransport`) depend on a domain type solely to check a single boolean (`validate_hardware_write_permitted()`).
  - **Multi-caller safety invariant:** Write operations are invoked by BOTH `TransactionManager` (feature reports and bulk streams) and `LcdStreamer` (streaming directly to `&mut dyn Transport`). Stripping the parameter and enforcing checks only in `TransactionManager` leaves `LcdStreamer` unconstrained and drops compile-time enforcement to convention.
  - **Resolution (Option B):**
    - `Transport` becomes a pure byte I/O trait (`pub(crate)` or `pub` in `monkey-core`), stripping `&SafetyRails` from all 4 methods. `HidTransport` and `MockTransport` become clean I/O adapters with zero domain awareness; `MockTransport` no longer needs dummy safety rails to test I/O.
    - Introduce `SafeTransport<'a>` guard façade wrapping `(&'a mut dyn Transport, &'a SafetyRails)`. It validates write authorization in a single centralized location and exposes `write_bulk` / `send_feature_report`.
    - Both `TransactionManager` and `LcdStreamer` consume `SafeTransport`, preserving the type-level forcing function (cannot write to hardware without `SafetyRails`) while eliminating 3-tier redundant checks.
    - `MonkaDevice` manufactures `SafeTransport` internally for its domain operations.
  — *Reversibility: costly — modifies `Transport` trait, 3 implementations (`HidTransport`, `MockTransport`, `RecordingTransport`), and caller test sites.*

### 5. Elimination of Test Seams and Env-Var Hacks
- **D-05 (Eliminate Test Seam Chaos & MONKEY_SIMULATE_EMPTY):**
  - Delete the production environment variable hacks: `probe.rs:201` and `info.rs:164` reading `std::env::var("MONKEY_SIMULATE_EMPTY")`.
  - Eliminate fragmented ad-hoc test seams across commands (`run_probe_with_transport`, `run_bench_with_transport`, `run_info_with_device_set`, `resolve_transport(mock: bool)`, and `stream_one(...)`).
  - Tests simulate empty device lists or hardware responses via `MonkaDevice::from_transport(Box::new(MockTransport::new()))` or deterministic mock device sets.
  — *Reversibility: reversible — cleans up test plumbing and dead production branches.*

### 6. Blast Radius & Blast Management
- **D-06 (Risk Mitigation & Regression Safety):**
  - GitNexus impact analysis confirms `open_device_path` and `find_monka_device_sets` have `CRITICAL` upstream impact (18–20 affected symbols, 12–13 processes, depth-1 breakages in 5 direct callers).
  - GitNexus index was refreshed (`node .gitnexus/run.cjs analyze --index-only`) ensuring all 6 consumers (`bench`, `doctor`, `info`, `lcd`, `probe`, `rgb`) are indexed.
  - Refactoring execution must be phased or atomic with strict verification: all 122 existing workspace unit/integration tests must pass, and `cargo clippy --all-targets -- -D warnings` must remain 100% clean.
  — *Reversibility: reversible — quality enforcement.*

### Agent's Discretion
- Internal layout and module location of `MonkaDevice` (e.g. `crates/monkey-core/src/device/mod.rs` or `crates/monkey-core/src/device.rs`).
- Exact enum naming for `OpenError` and `InterfacePolicy` provided the semantics match the decisions.

### Deferred Ideas
- None — discussion stayed strictly within the Phase 6 architecture refactoring scope.

---

## Executive Summary & Target Architecture

### Problem Statement
In MonKey v1.0, hardware lifecycle and discovery logic evolved organically across five MVP phases [VERIFIED: `.planning/ROADMAP.md`]. This resulted in architectural coupling:
1. **Accidental Discovery Duplication:** Every CLI command (`info.rs`, `probe.rs`, `lcd.rs`, `bench.rs`, `rgb.rs`) and diagnostic check (`doctor.rs`) independently invoked `init_hidapi()`, `find_monka_device_sets(&api)`, and `open_device_path(&api, dev)`.
2. **Domain Leakage into Byte I/O Adapters:** The `Transport` trait required `&SafetyRails` in `write_bulk` and `send_feature_report`, forcing low-level byte adapters (`HidTransport`, `MockTransport`, `RecordingTransport`) to depend directly on protocol domain types solely to check `validate_hardware_write_permitted()` [VERIFIED: `crates/monkey-core/src/transport/mod.rs:18,25`].
3. **Double Ownership & Borrow Conflicts:** Because `Transport` required mutable access (`&mut transport`) and `SafetyRails` required immutable access (`&safety`), commands repeatedly created throwaway `SafetyRails` instances (e.g. `lcd.rs:251,313`, `rgb.rs:146,182`, `bench.rs:229,306`), defeating session-level flash debouncing [VERIFIED: `crates/monkey-cli/src/commands/lcd.rs`].
4. **Ad-hoc Test Seams & Production Hacks:** CLI commands introduced 5 heterogeneous test wrappers (`run_probe_with_transport`, `run_bench_with_transport`, `run_info_with_device_set`, `resolve_transport`, `stream_one`) and an environment variable hack (`MONKEY_SIMULATE_EMPTY`) in `probe.rs:201` and `info.rs:164` to simulate empty hardware states [VERIFIED: `crates/monkey-cli/src/commands/probe.rs:201`, `info.rs:164`].

### Target Layered Architecture
Phase 6 refactors the system into a clean, deep-module hierarchy:

```
┌────────────────────────────────────────────────────────────────────────┐
│                              monkey-cli                                │
│   info       probe        lcd         rgb          bench       doctor  │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ consumes MonkaDevice API
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        monkey-core::device                             │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                           MonkaDevice                            │  │
│  │  - owns: transport: Box<dyn Transport>, safety: SafetyRails      │  │
│  │  - manufactures: SafeTransport<'a>                               │  │
│  │  - methods: open(policy), stream_frame, apply_rgb_*, probe, ...  │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│          │                                                │            │
│          ▼ delegates                                      ▼ creates    │
│  ┌─────────────────────────┐                   ┌────────────────────┐  │
│  │   LcdStreamer           │                   │ SafeTransport<'a>  │  │
│  │   RgbManager            │                   │ Guard Façade       │  │
│  │   TransactionManager    │◄──────────────────┤ Checks SafetyRails │  │
│  └─────────────────────────┘                   └──────────┬─────────┘  │
└───────────────────────────────────────────────────────────┼────────────┘
                                                            │ calls pure
                                                            ▼ I/O
┌────────────────────────────────────────────────────────────────────────┐
│                       Pure Transport Trait (pub)                       │
│  - write_bulk(report_id, data) -> Result<usize>                        │
│  - send_feature_report(data) -> Result<()>                             │
│  - get_feature_report(report_id, buf) -> Result<usize>                 │
│  - read_input_report(buf, timeout_ms) -> Result<usize>                 │
│   (Zero SafetyRails parameters, zero domain awareness)                 │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ implements
               ┌────────────────────┴────────────────────┐
               ▼                                         ▼
   ┌───────────────────────┐                 ┌───────────────────────┐
   │     HidTransport      │                 │     MockTransport     │
   │   (hidapi + darwin)   │                 │   (pure in-memory)    │
   └───────────────────────┘                 └───────────────────────┘
```

---

## Pure Transport Trait & SafeTransport Guard Façade (Option B)

### 1. Pure Transport Trait Refactoring
In `crates/monkey-core/src/transport/mod.rs`, strip `&SafetyRails` from the trait signature [CITED: D-07]:

```rust
pub trait Transport: Send {
    /// Writes bulk data to the device (Interface A bulk pipe). Pure byte I/O.
    fn write_bulk(
        &mut self,
        report_id: u8,
        data: &[u8],
    ) -> Result<usize, TransportError>;

    /// Sends a feature report to the device (Interface B control pipe). Pure byte I/O.
    fn send_feature_report(
        &mut self,
        data: &[u8],
    ) -> Result<(), TransportError>;

    /// Reads a feature report from the device (Interface B control pipe).
    fn get_feature_report(
        &mut self,
        report_id: u8,
        buf: &mut [u8],
    ) -> Result<usize, TransportError>;

    /// Reads an input report from the device with a millisecond timeout.
    fn read_input_report(
        &mut self,
        buf: &mut [u8],
        timeout_ms: i32,
    ) -> Result<usize, TransportError>;
}
```

### 2. Low-Level Adapter Cleanliness
- **`HidTransport` (`crates/monkey-core/src/transport/hid.rs`):**
  Remove `use crate::protocol::SafetyRails;`. Both `write_bulk` and `send_feature_report` remove safety checks and execute pure HID calls (`self.device.write` and `self.device.send_feature_report`).
- **`MockTransport` (`crates/monkey-core/src/transport/mock.rs`):**
  Remove `use crate::protocol::SafetyRails;`. Both `write_bulk` and `send_feature_report` record calls to `self.calls` and return mock results without needing mock safety rails.
- **`RecordingTransport` (`crates/monkey-core/tests/transaction_safety_test.rs`):**
  Remove `&SafetyRails` parameter. Records calls purely.

### 3. SafeTransport Guard Façade Implementation
Introduce `SafeTransport<'a>` in `crates/monkey-core/src/transport/safe.rs` (and re-export in `monkey-core::transport` and `monkey_core` root) [CITED: D-07]:

```rust
use crate::error::TransportError;
use crate::protocol::SafetyRails;
use crate::transport::Transport;

/// RAII guard façade wrapping a mutable Transport reference and an immutable SafetyRails reference.
///
/// Guarantees compile-time safety: no bulk write or feature report can reach the hardware
/// or mock adapter without passing through the centralized SafetyRails hardware write gate.
pub struct SafeTransport<'a> {
    transport: &'a mut dyn Transport,
    safety: &'a SafetyRails,
}

impl<'a> SafeTransport<'a> {
    pub fn new(transport: &'a mut dyn Transport, safety: &'a SafetyRails) -> Self {
        Self { transport, safety }
    }

    /// Reborrows the underlying transport and safety references, allowing SafeTransport
    /// to be passed into nested sub-managers without losing ownership.
    pub fn reborrow<'b>(&'b mut self) -> SafeTransport<'b> {
        SafeTransport {
            transport: &mut *self.transport,
            safety: self.safety,
        }
    }

    pub fn write_bulk(
        &mut self,
        report_id: u8,
        data: &[u8],
    ) -> Result<usize, TransportError> {
        self.safety
            .validate_hardware_write_permitted()
            .map_err(|e| TransportError::ProtocolViolation(e.to_string()))?;
        self.transport.write_bulk(report_id, data)
    }

    pub fn send_feature_report(
        &mut self,
        data: &[u8],
    ) -> Result<(), TransportError> {
        self.safety
            .validate_hardware_write_permitted()
            .map_err(|e| TransportError::ProtocolViolation(e.to_string()))?;
        self.transport.send_feature_report(data)
    }

    pub fn get_feature_report(
        &mut self,
        report_id: u8,
        buf: &mut [u8],
    ) -> Result<usize, TransportError> {
        self.transport.get_feature_report(report_id, buf)
    }

    pub fn read_input_report(
        &mut self,
        buf: &mut [u8],
        timeout_ms: i32,
    ) -> Result<usize, TransportError> {
        self.transport.read_input_report(buf, timeout_ms)
    }

    pub fn safety(&self) -> &SafetyRails {
        self.safety
    }
}
```

### 4. Consumer Adaptation
- **`TransactionManager<'a>` (`crates/monkey-core/src/protocol/transaction.rs`):**
  Updated to wrap `transport: SafeTransport<'a>`.
  `send_feature_command` validates opcode/mode against `self.transport.safety()`, then invokes `self.transport.send_feature_report(...)`.
  `stream_bulk_chunks` invokes `self.transport.write_bulk(...)`.
- **`LcdStreamer<'a>` (`crates/monkey-core/src/lcd/streamer.rs`):**
  Updated to wrap `transport: SafeTransport<'a>`.
  Constructor verifies `transport.safety().validate_hardware_write_permitted()?`.
  `send_frame_with_progress` invokes `self.transport.write_bulk(LCD_INTERFACE_A_REPORT_ID, chunk)` directly.
- **`RgbManager<'a>` (`crates/monkey-core/src/rgb/manager.rs`):**
  Updated to hold `transport: SafeTransport<'a>`. Constructs `TransactionManager::new(self.transport.reborrow())`.

---

## MonkaDevice Abstraction & Interface Policy Engine

### 1. MonkaDevice Concrete Struct
Located in `crates/monkey-core/src/device.rs` [CITED: D-01, D-04]:

```rust
pub struct MonkaDevice {
    transport: Box<dyn Transport>,
    safety: SafetyRails,
    device_set: Option<MonkaDeviceSet>,
    role: Option<InterfaceRole>,
    is_wireless: bool,
}
```

### 2. InterfacePolicy Enum
Standardizes interface selection variations across the 6 consumers [CITED: D-02]:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfacePolicy {
    /// Interface B first, fallback to Interface A (used by `rgb`).
    PreferB,
    /// Interface A only; hard fail if missing (used by `lcd`).
    RequireA,
    /// Interface B only; hard fail if missing (used by `probe`).
    RequireB,
    /// Interface A then Interface B (used by `bench` when running bulk transfers).
    BulkFirst,
    /// Interface B then Interface A (used by `bench` when running feature/transaction transfers).
    ControlFirst,
    /// Open whatever valid Monka interface is available.
    Any,
}
```

#### Selection Semantics Table

| Policy | Primary Target | Fallback Target | Failure Condition | Consumer |
|---|---|---|---|---|
| `PreferB` | Interface B (`0xFFFF`) | Interface A (`0xFF68`) | Neither interface present | `rgb` |
| `RequireA` | Interface A (`0xFF68`) | *None* | Interface A missing | `lcd` |
| `RequireB` | Interface B (`0xFFFF`) | *None* | Interface B missing | `probe` |
| `BulkFirst` | Interface A (`0xFF68`) | Interface B (`0xFFFF`) | Neither interface present | `bench (bulk)` |
| `ControlFirst` | Interface B (`0xFFFF`) | Interface A (`0xFF68`) | Neither interface present | `bench (transaction)` |
| `Any` | Interface B (`0xFFFF`) | Interface A (`0xFF68`) | No Monka device detected | general/fallback |

### 3. Lifecycle & Constructors
- `MonkaDevice::open(policy: InterfacePolicy) -> Result<MonkaDevice, OpenError>`:
  1. Calls `init_hidapi()`. Maps error to `OpenError::HidInit(err)`.
  2. Calls `find_monka_device_sets(&api)`. If empty, returns `Err(OpenError::NoDevice)`.
  3. Picks primary device set `sets[0]`.
  4. Resolves `policy` to a target `DiscoveredDevice` and `InterfaceRole`. If missing, returns `Err(OpenError::InterfaceUnavailable(role))`.
  5. Opens device handle via `open_device_path(&api, target_dev)`. If fails, returns `Err(OpenError::InterfaceOpenFailed(role, err))`.
  6. Wraps in `HidTransport::new(hid_dev)`.
  7. Returns `MonkaDevice` initialized with `SafetyRails::new()`, `device_set`, `role`, and `is_wireless`.
- `MonkaDevice::from_transport(transport: Box<dyn Transport>) -> Self`:
  Constructs a headless/test `MonkaDevice` with default `SafetyRails::new()`.
  Supports fluent builder helpers:
  - `.with_device_set(set: MonkaDeviceSet) -> Self`
  - `.with_hardware_writes_allowed(allowed: bool) -> Self`
  - `.with_wireless(wireless: bool) -> Self`
- `MonkaDevice::discover() -> Result<Vec<MonkaDeviceSet>, OpenError>`:
  Static helper for `info` command, running HID init and device set scanning without opening an endpoint handle.

### 4. Deep Module Operations (Borrow-Checker Safe)
Exposes cohesive hardware workflows that manufacture `SafeTransport` internally, preventing simultaneous `&mut transport` and `&safety` access from callers [CITED: D-04]:

- `device.safe_transport(&mut self) -> SafeTransport<'_>`:
  Manufactures `SafeTransport::new(&mut *self.transport, &self.safety)`.
- `device.stream_frame(&mut self, frame: &[u8; LCD_FRAME_BYTES], config: LcdPacingConfig) -> Result<LcdStreamMetrics>`
- `device.stream_frame_with_progress<F>(&mut self, frame: &[u8; LCD_FRAME_BYTES], config: LcdPacingConfig, on_chunk: F) -> Result<LcdStreamMetrics> where F: FnMut(usize, usize)`
- `device.apply_rgb_preview(&mut self, config: &LightingConfig) -> Result<()>`
- `device.apply_rgb_commit(&mut self, config: &LightingConfig, is_wireless: bool, battery: Option<u8>, force: bool) -> Result<()>`
- `device.readback_rgb_status(&mut self) -> Result<Option<LightingConfig>>`
- `device.send_feature_command(&mut self, packet: &FeatureReportPacket, mode: WriteMode) -> Result<()>`
- `device.probe(&mut self) -> Result<ProbeOutput, MonkeyError>`:
  Executes non-destructive read query on report 0, verifies 0 writes, detects connection state, and computes capabilities.
- `device.run_bulk_benchmark<F>(&mut self, config: &BenchmarkConfig, on_frame: F) -> Result<ThroughputReport> where F: FnMut(usize, usize)`
- `device.run_transaction_benchmark<F>(&mut self, config: &BenchmarkConfig, on_sample: F) -> Result<LatencyReport> where F: FnMut(usize, usize)`

---

## Diagnostic Architecture & Granular Error Handling

### 1. Granular OpenError Definition
In `crates/monkey-core/src/device.rs` [CITED: D-03]:

```rust
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("Failed to initialize HID subsystem: {0}")]
    HidInit(#[from] TransportError),

    #[error("No Monka 3075 Pro / RKGK890 keyboard detected (VID: 0x{MONKA_VID:04x}, PID: 0x{MONKA_PID:04x}). Please check USB connection.")]
    NoDevice,

    #[error("Requested interface {0:?} is not available on detected keyboard")]
    InterfaceUnavailable(InterfaceRole),

    #[error("Failed to open interface {0:?}: {1}")]
    InterfaceOpenFailed(InterfaceRole, TransportError),
}
```

#### Exit Code Classification Compatibility
In `crates/monkey-cli/src/error.rs`, `classify_error(&anyhow::Error)` inspects error chains:
- `OpenError::NoDevice` contains `"No Monka 3075 Pro"` -> maps to `ExitCode::NoDevice` (`3`) [VERIFIED: `error.rs:56`].
- `OpenError::InterfaceOpenFailed` containing `"Permission denied"` or `"exclusive access"` -> maps to `ExitCode::Permission` (`4`) [VERIFIED: `error.rs:75`].
- Safety gate denials -> maps to `ExitCode::Blocked` (`5`) [VERIFIED: `error.rs:65`].
Zero changes are required in `classify_error` to maintain 100% POSIX compliance!

### 2. Doctor Non-Fail-Fast Diagnostic Engine (`MonkaDevice::diagnose()`)
Consumer #6 (`monkey doctor`) requires stage-by-stage health assessment without early returns [CITED: D-03].

```rust
#[derive(Debug)]
pub struct DeviceDiagnostics {
    pub hid_init: Result<(), TransportError>,
    pub device_set: Option<MonkaDeviceSet>,
    pub interface_a_status: InterfaceCheckStatus,
    pub interface_b_status: InterfaceCheckStatus,
}

#[derive(Debug)]
pub enum InterfaceCheckStatus {
    NotPresent,
    OpenSuccess,
    OpenFailed(TransportError),
}

impl MonkaDevice {
    pub fn diagnose() -> DeviceDiagnostics {
        let api = match init_hidapi() {
            Ok(api) => api,
            Err(e) => {
                return DeviceDiagnostics {
                    hid_init: Err(e),
                    device_set: None,
                    interface_a_status: InterfaceCheckStatus::NotPresent,
                    interface_b_status: InterfaceCheckStatus::NotPresent,
                };
            }
        };

        let sets = find_monka_device_sets(&api);
        let device_set = sets.into_iter().next();

        let (interface_a_status, interface_b_status) = if let Some(ref set) = device_set {
            let status_a = match set.interface_a {
                Some(ref dev_a) => match open_device_path(&api, dev_a) {
                    Ok(_) => InterfaceCheckStatus::OpenSuccess,
                    Err(e) => InterfaceCheckStatus::OpenFailed(e),
                },
                None => InterfaceCheckStatus::NotPresent,
            };

            let status_b = match set.interface_b {
                Some(ref dev_b) => match open_device_path(&api, dev_b) {
                    Ok(_) => InterfaceCheckStatus::OpenSuccess,
                    Err(e) => InterfaceCheckStatus::OpenFailed(e),
                },
                None => InterfaceCheckStatus::NotPresent,
            };

            (status_a, status_b)
        } else {
            (InterfaceCheckStatus::NotPresent, InterfaceCheckStatus::NotPresent)
        };

        DeviceDiagnostics {
            hid_init: Ok(()),
            device_set,
            interface_a_status,
            interface_b_status,
        }
    }
}
```

In `crates/monkey-core/src/doctor.rs`:
`run_doctor_checks(mock: bool)` delegates real checks to `MonkaDevice::diagnose()`, decoupling `doctor.rs` completely from raw HID APIs while maintaining exact check output, remediations, and summary logic [VERIFIED: `doctor.rs:42-229`].

---

## Elimination of Ad-Hoc Test Seams & Env-Var Hacks

### 1. Removal of `MONKEY_SIMULATE_EMPTY`
The hack was used in exactly four places [VERIFIED: search across workspace]:
1. `crates/monkey-cli/src/commands/info.rs:164`:
   ```rust
   // REMOVE:
   let sets = if std::env::var("MONKEY_SIMULATE_EMPTY").is_ok() { Vec::new() } else { ... };
   ```
2. `crates/monkey-cli/src/commands/probe.rs:201`:
   ```rust
   // REMOVE:
   let sets = if std::env::var("MONKEY_SIMULATE_EMPTY").is_ok() { Vec::new() } else { ... };
   ```
3. `crates/monkey-cli/tests/cli_probe_test.rs:226,257`:
   Tests in `test_cli_no_device_found_error_exit()` used `.env("MONKEY_SIMULATE_EMPTY", "1")`.
   **Replacement:**
   In `cli_probe_test.rs`, replace the subprocess hack with deterministic test assertions:
   - Verify `classify_error(&anyhow!(OpenError::NoDevice))` outputs `ExitCode::NoDevice` (`3`).
   - For commands, test that when `MonkaDevice::open` returns `OpenError::NoDevice`, the error message and exit code match expected constants.

### 2. Elimination of 5 Heterogeneous Test Seams
[CITED: D-05]
1. `run_probe_with_transport` & `probe_device_with_transport` in `probe.rs`:
   Replaced by `MonkaDevice::from_transport(Box::new(mock)).probe()`.
2. `run_bench_with_transport` in `bench.rs`:
   Replaced by `device.run_bulk_benchmark(...)` and `device.run_transaction_benchmark(...)`.
3. `run_info_with_device_set` in `info.rs`:
   Retained as a clean descriptor renderer helper or refactored to `run_info_with_device(&device)`.
4. `resolve_transport(mock, allow_hardware_writes)` in `rgb.rs`:
   Replaced by:
   ```rust
   let mut device = if args.mock {
       MonkaDevice::from_transport(Box::new(MockTransport::new()))
           .with_hardware_writes_allowed(true)
   } else {
       if requires_write { require_write_consent(args.allow_hardware_writes)?; }
       MonkaDevice::open(InterfacePolicy::PreferB)?
           .with_hardware_writes_allowed(true)
   };
   ```
5. `stream_one` and `open_lcd_transport` in `lcd.rs`:
   Replaced by:
   ```rust
   let mut device = if args.mock {
       MonkaDevice::from_transport(Box::new(MockTransport::new()))
           .with_hardware_writes_allowed(true)
   } else {
       require_write_consent(args.allow_hardware_writes)?;
       MonkaDevice::open(InterfacePolicy::RequireA)?
           .with_hardware_writes_allowed(true)
   };
   let metrics = device.stream_frame_with_progress(&frame, config, on_progress)?;
   ```

---

## Blast Radius & Upstream Impact (GitNexus & Codebase)

### 1. GitNexus Impact Analysis Summary
GitNexus graph analysis was performed on core symbols targeted for modification [VERIFIED: GitNexus MCP tools]:

- **`open_device_path`**:
  - Upstream Impact: **CRITICAL** (18 impacted symbols, 12 processes affected, 5 depth-1 callers).
  - Depth-1 Callers: `doctor::run_doctor_checks`, `commands::lcd::open_lcd_transport`, `commands::probe::run_probe_with_writer`, `commands::rgb::resolve_transport`, `commands::bench::run_bench`.
- **`find_monka_device_sets`**:
  - Upstream Impact: **CRITICAL** (20 impacted symbols, 13 processes affected, 6 depth-1 callers).
  - Depth-1 Callers: `doctor::run_doctor_checks`, `commands::info::run_info_with_writer`, `commands::probe::run_probe_with_writer`, `commands::lcd::open_lcd_transport`, `commands::rgb::resolve_transport`, `commands::bench::run_bench`.

### 2. Blast Radius Matrix by Component

| Component | Files Affected | Invariant / Risk | Mitigation Strategy |
|---|---|---|---|
| **Transport Trait** | `transport/mod.rs`, `hid.rs`, `mock.rs`, `tests/transaction_safety_test.rs`, `tests/mock_transport_test.rs` | Breaking trait signatures: removing `&SafetyRails` breaks 3 implementations. | Introduce `SafeTransport` simultaneously before modifying callers. Update tests atomically. |
| **Protocol Safety** | `protocol/transaction.rs`, `protocol/channel.rs` | `TransactionManager` must not lose compile-time safety checks. | Adapt `TransactionManager` to wrap `SafeTransport`. |
| **LCD Pipeline** | `lcd/streamer.rs`, `commands/lcd.rs`, `tests/lcd_test.rs`, `tests/cli_lcd_test.rs` | LCD chunks must still enforce 8-chunk pacing and hardware write consent. | `LcdStreamer` consumes `SafeTransport`. `MonkaDevice::stream_frame` handles progress bar. |
| **RGB Engine** | `rgb/manager.rs`, `commands/rgb.rs`, `tests/rgb_manager_test.rs`, `tests/cli_rgb_test.rs` | Volatile RAM preview (30Hz) and flash debounce (500ms) must remain intact. | `RgbManager` consumes `SafeTransport`. `MonkaDevice::apply_rgb_*` exposes operations. |
| **Benchmarking** | `bench/mod.rs`, `commands/bench.rs`, `tests/cli_bench_test.rs` | Throughput and latency timing must not suffer allocation overhead. | Benchmark harness drives `MonkaDevice` or `SafeTransport`. |
| **Diagnostics** | `doctor.rs`, `commands/doctor.rs`, `tests/cli_doctor_test.rs` | Non-early-returning stage diagnostic reporting must preserve Pass/Warn/Fail output. | `MonkaDevice::diagnose()` provides complete diagnostic tree. |
| **Device Discovery** | `device.rs`, `commands/info.rs`, `commands/probe.rs`, `tests/device_discovery_test.rs`, `tests/cli_probe_test.rs` | Eliminating `MONKEY_SIMULATE_EMPTY` and standardizing on `InterfacePolicy`. | Update `info` and `probe` to consume `MonkaDevice`. |

---

## Runtime State Inventory

An audit of all persistent runtime state across the workspace was conducted [VERIFIED: filesystem search and git inspection]:

1. **Stored Data & Profiles:**
   - Default profile location: `./rgb_profile.json` (created only when user runs `monkey rgb save`).
   - Static embedded assets: `crates/monkey-core/data/layout_81keys.json` (compiled into binary via `include_str!`).
   - No hidden cache files, SQLite databases, or local application state files exist.
2. **Live Service Config & Daemons:**
   - Zero background services, daemons, LaunchAgents, or systemd units exist in v1.0. All executions are one-shot CLI commands.
3. **OS-Registered State:**
   - IOKit USB HID Registry Entry IDs (`DevSrvsID:<u64>`) allocated dynamically by macOS kernel.
   - Apple TCC Input Monitoring (`kTCCServiceListenEvent`) requested at OS level on first access to Interface B.
4. **Secrets & Environment Variables:**
   - `MONKEY_SIMULATE_EMPTY`: Used only in `probe.rs` and `info.rs`. **Targeted for complete deletion in Phase 6.**
   - `RUST_LOG`: Used for `tracing-subscriber` runtime log filtering (`warn`, `info`, `debug`, `trace`).
5. **Build Artifacts:**
   - Cargo virtual workspace: `target/` (containing target debug/release builds). Cleanable via `cargo clean`.

---

## Validation Architecture & Implementation Plan

### 1. Test Suite Invariants
The workspace has **122 automated unit, integration, and CLI tests** [VERIFIED: `cargo test` execution]:
- `monkey-core` unittests: 1 test
- `device_discovery_test.rs`: 13 tests
- `hid_transport_test.rs`: 13 tests
- `lcd_test.rs`: 12 tests
- `mock_transport_test.rs`: 7 tests
- `protocol_codecs_test.rs`: 8 tests
- `protocol_types_test.rs`: 6 tests
- `rgb_codecs_test.rs`: 4 tests
- `rgb_manager_test.rs`: 5 tests
- `transaction_safety_test.rs`: 8 tests
- `cli_bench_test.rs`: 10 tests
- `cli_completions_test.rs`: 5 tests
- `cli_doctor_test.rs`: 3 tests
- `cli_exit_codes_test.rs`: 5 tests
- `cli_lcd_test.rs`: 3 tests
- `cli_probe_test.rs`: 10 tests
- `cli_rgb_test.rs`: 8 tests

**Strict Nyquist Criterion:** At every step of Phase 6 execution:
- All 122 existing tests (or updated equivalents) must pass (`cargo test`).
- Zero clippy warnings allowed (`cargo clippy --all-targets -- -D warnings`).
- License and security audit clean (`cargo deny check`).

### 2. Execution Wave Plan

To mitigate the **CRITICAL** blast radius identified by GitNexus without intermediate build breakages, execution should follow 4 atomic waves:

```
┌────────────────────────────────────────────────────────────────────────┐
│ WAVE 1: Pure Transport & SafeTransport Façade                          │
│ - Strip &SafetyRails from Transport trait                              │
│ - Implement SafeTransport<'a> guard façade                             │
│ - Update HidTransport, MockTransport, RecordingTransport               │
│ - Adapt TransactionManager, LcdStreamer, RgbManager to SafeTransport   │
│ - Verify core unit tests: mock_transport_test, transaction_safety_test │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ WAVE 2: MonkaDevice Lifecycle & InterfacePolicy Core Engine            │
│ - Add InterfacePolicy enum to monkey-core::device                      │
│ - Add OpenError enum with granular stages                              │
│ - Implement MonkaDevice::open(policy) and from_transport(mock)         │
│ - Implement MonkaDevice deep operations: stream_frame, apply_rgb_*, ...│
│ - Implement MonkaDevice::diagnose() -> DeviceDiagnostics               │
│ - Add unit tests for MonkaDevice & InterfacePolicy in monkey-core      │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ WAVE 3: CLI Migration & Env-Var Hack Eradication                       │
│ - Migrate commands/info.rs to MonkaDevice::discover()                  │
│ - Migrate commands/probe.rs to MonkaDevice::open(RequireB)             │
│ - Migrate commands/lcd.rs to MonkaDevice::open(RequireA)               │
│ - Migrate commands/rgb.rs to MonkaDevice::open(PreferB)                │
│ - Migrate commands/bench.rs to MonkaDevice::open(BulkFirst/ControlFirst│
│ - Migrate commands/doctor.rs & core/doctor.rs to MonkaDevice::diagnose │
│ - Remove MONKEY_SIMULATE_EMPTY from info.rs, probe.rs, cli_probe_test  │
│ - Remove ad-hoc seams: stream_one, resolve_transport, ...              │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ WAVE 4: Full Verification, Documentation & GitNexus Re-indexing        │
│ - Run full workspace cargo test (122+ tests passing)                   │
│ - Run cargo clippy --all-targets -- -D warnings (0 warnings)           │
│ - Update docs/dev/architecture.md with SafeTransport & MonkaDevice     │
│ - Re-run GitNexus analyze to refresh symbol map and verify impact drop │
└────────────────────────────────────────────────────────────────────────┘
```

---

## Confidence Assessment

| Dimension | Rating | Rationale |
|---|---|---|
| **Domain Understanding** | HIGH | Complete protocol, dual-interface USB HID model, and hardware safety constraints analyzed and verified directly in codebase. |
| **Architectural Design** | HIGH | Decisions D-01 through D-07 address the exact seam leaks, borrow-checker constraints, and interface policies with clean Rust patterns. |
| **Blast Radius Control** | HIGH | GitNexus impact analysis pinpointed the exact 18-20 affected symbols; 4-wave plan guarantees no intermediate broken states. |
| **Test Verification** | HIGH | Baseline test suite executed (122 tests passing, 0 clippy warnings); test seam transitions mapped 1-to-1. |
