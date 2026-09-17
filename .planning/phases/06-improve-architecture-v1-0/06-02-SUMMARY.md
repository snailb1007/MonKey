---
phase: 06-improve-architecture-v1-0
plan: 02
subsystem: core-device
tags: [monka-device, interface-policy, open-error, safe-transport, doctor]

requires:
  - phase: 06-improve-architecture-v1-0
    provides: Pure Transport trait and SafeTransport RAII guard facade
provides:
  - MonkaDevice concrete coordinator struct
  - InterfacePolicy declarative enum (RequireA, RequireB, PreferB, BulkFirst, ControlFirst, Any)
  - OpenError staged initialization errors
  - DeviceDiagnostics non-fail-fast inspection
  - Borrow-safe deep module operations on MonkaDevice (LCD, RGB, probe, benchmarks)
affects: [monkey-cli, doctor, cli-commands]

actuals:
  tasks: 3
  commits: 1

tech-stack:
  added: []
  patterns: [concrete-coordinator, interface-policy, stage-categorized-error, non-fail-fast-diagnostics]

key-files:
  created:
    - crates/monkey-core/tests/monka_device_test.rs
  modified:
    - crates/monkey-core/src/device.rs
    - crates/monkey-core/src/bench/mod.rs
    - crates/monkey-core/src/doctor.rs
    - crates/monkey-core/src/lib.rs
    - crates/monkey-core/src/transport/safe.rs

key-decisions:
  - "D-01: Transport remains the single underlying I/O test seam; MonkaDevice is a concrete coordinator struct."
  - "D-02: Interface selection variations are encoded into the InterfacePolicy enum."
  - "D-03: OpenError categorizes hardware initialization into granular stage-specific variants; MonkaDevice::diagnose() provides non-fail-fast inspection."
  - "D-04: SafetyRails is owned internally by MonkaDevice; deep module operations manufacture SafeTransport internally to satisfy Rust borrow rules."

requirements-completed: [ARCH-01]

coverage:
  - id: D1
    description: "MonkaDevice lifecycle and InterfacePolicy resolution"
    requirement: "ARCH-01"
    verification:
      - kind: unit
        ref: "crates/monkey-core/tests/monka_device_test.rs#test_interface_policy_resolution_dual_set"
        status: pass
    human_judgment: false
  - id: D2
    description: "MonkaDevice deep operations and probe read-only safety invariant"
    requirement: "ARCH-01"
    verification:
      - kind: unit
        ref: "crates/monkey-core/tests/monka_device_test.rs#test_monka_device_probe_read_only_guarantee"
        status: pass
    human_judgment: false
  - id: D3
    description: "monkey-core::doctor consumes MonkaDevice::diagnose()"
    requirement: "ARCH-01"
    verification:
      - kind: unit
        ref: "crates/monkey-core/tests/monka_device_test.rs#test_monka_device_diagnose_structure"
        status: pass
    human_judgment: false

duration: 10 min
completed: 2026-09-17
status: complete
---

# 06-02 Summary: MonkaDevice Lifecycle & InterfacePolicy Core Engine

**Concrete `MonkaDevice` coordinator, declarative `InterfacePolicy` resolution, granular `OpenError` stages, borrow-safe deep module operations, and non-fail-fast `diagnose()` implemented in `monkey-core`.**

## Accomplishments
- Implemented `InterfacePolicy` enum per D-02 (`PreferB`, `RequireA`, `RequireB`, `BulkFirst`, `ControlFirst`, `Any`) with full unit test coverage across dual, single, and empty device sets.
- Implemented `OpenError` enum per D-03 (`HidInit`, `NoDevice`, `InterfaceUnavailable`, `InterfaceOpenFailed`) categorizing initialization failures into distinct stages.
- Implemented concrete `MonkaDevice` coordinator per D-01 and D-04, encapsulating `Box<dyn Transport>` and `SafetyRails`, with headless constructor `from_transport()` preserving `Transport` as the single underlying test seam.
- Implemented borrow-safe deep module operations on `MonkaDevice` (`stream_frame_with_progress`, `stream_frame`, `apply_rgb_preview`, `apply_rgb_commit`, `readback_rgb_status`, `probe`, `run_bulk_benchmark`, `run_transaction_benchmark`).
- Implemented non-fail-fast `MonkaDevice::diagnose()` and refactored `monkey-core::doctor::run_doctor_checks` to consume it, eliminating duplicated raw HID calls.
- Verified zero writes emitted during probe via `MockTransport::assert_no_writes()` (T-06-03 mitigation).

## Verification
- `cargo test -p monkey-core --test monka_device_test`: 10 passed, 0 failed.
- `cargo test -p monkey-core`: all 89 unit and integration tests passed.
- `cargo clippy -p monkey-core --all-targets -- -D warnings`: 0 warnings.

## Self-Check: PASSED
