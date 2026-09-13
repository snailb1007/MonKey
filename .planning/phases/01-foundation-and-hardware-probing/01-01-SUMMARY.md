---
phase: 01-foundation-and-hardware-probing
plan: 01
subsystem: transport
tags: [rust, hidapi, zerocopy, thiserror, transport-seam, mock-transport]

requires: []
provides:
  - Virtual Cargo workspace establishing monkey-core library and monkey-cli stub
  - Synchronous Send Transport trait abstraction and typed TransportError hierarchy
  - Deterministic in-memory MockTransport with call recording, FIFO input queues, canned responses, and assert_no_writes
  - Zero-copy packet layouts (FeatureReportPacket, BulkChunkPacket) with compile-time 64B/4096B size assertions and little-endian primitives
affects:
  - 01-02-PLAN
  - 01-03-PLAN
  - 02-protocol-codecs
  - 03-lcd-rendering

actuals:
  tokens: 5200
  tasks: 3
  commits: 3

tech-stack:
  added:
    - "hidapi 2.6.7 (with macos-shared-device)"
    - "zerocopy 0.8.57 (with derive)"
    - "thiserror 2.0.20"
    - "tracing 0.1.44"
    - "serde 1.0.229 (with derive)"
  patterns:
    - "Synchronous Send Transport trait abstraction for zero-Tokio hardware I/O (D-04)"
    - "In-memory test double MockTransport with call recording and assert_no_writes guard (D-06, D-12)"
    - "Zero-copy #[repr(C, packed)] packet structs with compile-time size assertions (D-07, D-08, D-10)"

key-files:
  created:
    - Cargo.toml
    - crates/monkey-core/Cargo.toml
    - crates/monkey-core/src/lib.rs
    - crates/monkey-core/src/error.rs
    - crates/monkey-core/src/transport/mod.rs
    - crates/monkey-core/src/transport/mock.rs
    - crates/monkey-core/src/protocol/mod.rs
    - crates/monkey-core/src/protocol/types.rs
    - crates/monkey-core/tests/mock_transport_test.rs
    - crates/monkey-core/tests/protocol_types_test.rs
  modified:
    - .gitignore

key-decisions:
  - "D-03: Centralized hidapi 2.6.7 dependency with macos-shared-device feature in workspace root"
  - "D-04: Implemented synchronous Transport trait with write_bulk, send_feature_report, get_feature_report, and read_input_report"
  - "D-06 & D-12: Implemented MockTransport with assert_no_writes to verify read-only probing safety without hardware"
  - "D-07, D-08 & D-10: Used zerocopy derives and little-endian primitives with compile-time 64B/4096B size assertions"

patterns-established:
  - "Deterministic mock recording: Store calls as TransportCall enum in FIFO Vec to allow exact assertion of hardware interactions"
  - "Fail-fast header validation: Check magic byte 0x04 and markers [0xAA, 0x55] prior to packet payload processing"
  - "Compile-time size guards: const _: () = assert!(size_of::<T>() == N) to enforce packet layouts at build time"

requirements-completed:
  - DISC-04

coverage:
  - id: D1
    description: "Cargo virtual workspace compiles crates/monkey-core and resolves dependencies cleanly"
    requirement: "DISC-04"
    verification:
      - kind: unit
        ref: "cargo check -p monkey-core"
        status: pass
    human_judgment: false
  - id: D2
    description: "Transport trait seam and structured TransportError hierarchy"
    requirement: "DISC-04"
    verification:
      - kind: unit
        ref: "crates/monkey-core/tests/mock_transport_test.rs#test_call_recording_fifo_sequence"
        status: pass
    human_judgment: false
  - id: D3
    description: "Deterministic MockTransport with call recording, FIFO queues, and assert_no_writes safety guard"
    requirement: "DISC-04"
    verification:
      - kind: unit
        ref: "crates/monkey-core/tests/mock_transport_test.rs"
        status: pass
    human_judgment: false
  - id: D4
    description: "Zero-copy packet layouts with little-endian primitives and compile-time size guards (64B and 4096B)"
    requirement: "DISC-04"
    verification:
      - kind: unit
        ref: "crates/monkey-core/tests/protocol_types_test.rs"
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-09-13
status: complete
---

# Phase 1 Plan 01: Cargo Virtual Workspace, Transport Trait, and MockTransport Summary

**Virtual Cargo workspace scaffolded with synchronous Transport trait seam, deterministic MockTransport test double, and zero-copy packet layouts with compile-time 64B/4096B size assertions.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-09-13T07:00:20Z
- **Completed:** 2026-09-13T07:15:00Z
- **Tasks:** 3
- **Files modified:** 11

## Accomplishments

- Established virtual Cargo workspace with `crates/monkey-core` and stub `crates/monkey-cli`, resolving vetted dependencies (`hidapi` 2.6.7 with `macos-shared-device`, `zerocopy` 0.8.57, `thiserror` 2.0.20, `tracing` 0.1.44, `serde` 1.0.229).
- Implemented synchronous `Transport: Send` trait and structured `TransportError` enum covering all hardware failure modes.
- Implemented deterministic `MockTransport` supporting FIFO call recording, canned feature reports, FIFO input queue draining, error injection, and `assert_no_writes` read-only verification guard.
- Defined `FeatureReportPacket` (64 bytes) and `BulkChunkPacket` (4096 bytes) using `zerocopy` derives, little-endian types, header validation (`0x04` magic, `0xAA 0x55` marker), and compile-time size assertions.
- Authored 11 comprehensive automated unit tests across `mock_transport_test.rs` and `protocol_types_test.rs` with 100% pass rate.

## Task Commits

Each task was committed atomically:

1. **Task 1: Initialize Cargo virtual workspace, Transport trait seam, and TransportError** - `6ce5cb8` (feat)
2. **Task 2: Implement deterministic MockTransport with call recording and assertion helpers** - `bcb054e` (feat)
3. **Task 3: Define zero-copy packet layouts with little-endian primitives and compile-time size guards** - `0ff84df` (feat)

## Files Created/Modified

- `Cargo.toml` - Virtual workspace root manifest declaring members and centralized dependencies
- `Cargo.lock` - Pinned dependency lockfile
- `.gitignore` - Added `/target/` to ignore build artifacts
- `crates/monkey-core/Cargo.toml` - Core driver library manifest inheriting workspace dependencies
- `crates/monkey-core/src/lib.rs` - Library entrypoint re-exporting errors, transport, and protocol types
- `crates/monkey-core/src/error.rs` - Structured `TransportError` enum using `thiserror`
- `crates/monkey-core/src/transport/mod.rs` - Synchronous `Transport` trait seam definition
- `crates/monkey-core/src/transport/mock.rs` - Deterministic in-memory `MockTransport` implementation
- `crates/monkey-core/src/protocol/mod.rs` - Protocol module root exporting types
- `crates/monkey-core/src/protocol/types.rs` - Zero-copy packet layouts and compile-time size guards
- `crates/monkey-core/tests/mock_transport_test.rs` - Unit tests for mock call recording, canned responses, and safety guards
- `crates/monkey-core/tests/protocol_types_test.rs` - Unit tests for zero-copy transmute and compile-time invariants
- `crates/monkey-cli/Cargo.toml` - Minimal CLI manifest stub for workspace member resolution
- `crates/monkey-cli/src/main.rs` - CLI entrypoint placeholder

## Decisions Made

- Added minimal placeholder for `crates/monkey-cli` so the virtual workspace resolves and compiles cleanly without waiting for Plan 03.
- Installed Rust stable toolchain via standard `rustup` installer on host and added `~/.cargo/bin` to PATH.
- Added `/target/` to `.gitignore` to prevent committing build artifacts.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Missing Rust toolchain on environment**
- **Found during:** Task 1 (checking compiler toolchain)
- **Issue:** `cargo` and `rustc` were not installed on the system (`command not found`).
- **Fix:** Installed Rust stable 1.98.1 via `rustup-init` and added `export PATH="$HOME/.cargo/bin:$PATH"` to `~/.zshrc`.
- **Files modified:** `~/.zshrc`
- **Verification:** `cargo --version` and `rustc --version` succeed.
- **Committed in:** Not in repo git (host environment configuration per 01-CONTEXT Claude's Discretion).

**2. [Rule 3 - Blocking] Workspace member monkey-cli required for workspace resolution**
- **Found during:** Task 1
- **Issue:** Cargo fails to check or build a virtual workspace if a member declared in root `Cargo.toml` (`crates/monkey-cli`) lacks a manifest.
- **Fix:** Created a minimal `crates/monkey-cli/Cargo.toml` stub and placeholder `src/main.rs` so `cargo check -p monkey-core` runs smoothly. Plan 03 will flesh this out.
- **Files modified:** `crates/monkey-cli/Cargo.toml`, `crates/monkey-cli/src/main.rs`
- **Verification:** `cargo check -p monkey-core` runs and passes.
- **Committed in:** `6ce5cb8` (Task 1 commit).

---

**Total deviations:** 2 auto-fixed (both environment/workspace blocking fixes).
**Impact on plan:** None. All required deliverables and tests completed cleanly as planned.

## Issues Encountered

None. All 11 automated unit tests and `clippy` checks passed with zero errors or warnings.

## User Setup Required

None - toolchain installed and workspace self-contained.

## Next Phase Readiness

- Ready for Plan 02 (`01-02-PLAN.md`): Implement userspace `HidTransport` wrapping `hidapi::HidDevice`, dual-interface discovery (`0xFF68:0x0061` bulk OUT vs `0x000C:0x0001`/`0xFFFF` control), and non-destructive active connection state detection.

---
*Phase: 01-foundation-and-hardware-probing*
*Completed: 2026-09-13*
