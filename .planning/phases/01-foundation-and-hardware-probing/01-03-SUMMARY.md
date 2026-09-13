---
phase: 01-foundation-and-hardware-probing
plan: 03
subsystem: cli
tags: [rust, clap, cli, json-formatting, headless-tests, mock-transport, capability-probe, read-only-safety]

requires:
  - phase: 01-foundation-and-hardware-probing
    plan: 02
provides:
  - Standalone binary crate crates/monkey-cli with monkey executable
  - User-facing monkey info command displaying hardware identity and composite HID interface breakdown
  - User-facing monkey probe command inspecting capability tuple (model, hardware revision, firmware version, transport state, capabilities)
  - Formatted human-readable terminal output and structured machine-readable JSON output via --json flag (D-11)
  - Strict read-only probing enforcement guaranteeing zero flash writes and zero DFU/ISP mode triggers (D-12)
  - Automated headless integration test suite verifying CLI commands and JSON schemas against MockTransport without physical hardware (D-06)
affects:
  - 02-protocol-codecs
  - 03-lcd-rendering
  - 04-rgb-matrix
  - 05-production-hardening

actuals:
  tokens: 24000
  tasks: 3
  commits: 3

tech-stack:
  added:
    - "clap 4.6.6 (with derive, env)"
    - "serde_json 1.0.151"
    - "tracing-subscriber 0.3.23 (with env-filter, fmt)"
    - "anyhow 1.0.104"
  patterns:
    - "Unified output formatting via OutputFormat enum with Human and Json serializers (D-11)"
    - "Zero-write read-only probing invariant verified via MockTransport::assert_no_writes() (D-12)"
    - "Headless CLI integration testing with MockTransport and synthetic device sets without hardware (D-06)"
    - "Deterministic error handling with clean stderr messages and standard non-zero exit codes on missing hardware"

key-files:
  created:
    - crates/monkey-cli/src/lib.rs
    - crates/monkey-cli/src/commands/mod.rs
    - crates/monkey-cli/src/commands/info.rs
    - crates/monkey-cli/src/commands/probe.rs
    - crates/monkey-cli/src/output.rs
    - crates/monkey-cli/tests/cli_probe_test.rs
  modified:
    - Cargo.lock
    - crates/monkey-cli/Cargo.toml
    - crates/monkey-cli/src/main.rs

key-decisions:
  - "D-01 & DISC-02: Formatted hardware identifiers (VID 0x05AC, PID 0x024F, Product Name RKGK890/BT5.1-KB, and interfaces) in human sections and machine JSON"
  - "D-11: Supported --json global flag across all inspection commands for clean JSON output"
  - "D-12 & DISC-03: Strictly enforced read-only safety invariant in monkey probe with zero flash commits, confirmed via MockTransport::assert_no_writes()"
  - "D-06: Validated end-to-end CLI command execution, schema serialization, and error codes in headless automated integration tests"

patterns-established:
  - "Writer abstraction for CLI commands: Functions accept generic W: Write allowing in-memory headless testing without mocking process stdout"
  - "Deterministic simulation hook: MONKEY_SIMULATE_EMPTY environment variable enables complete test coverage of CLI error exit codes without unplugging devices"
  - "Dual presentation: Human-readable sectioned terminal tables for interactive users and structured JSON for scripts and CI automation"

requirements-completed:
  - DISC-02
  - DISC-03

coverage:
  - id: D1
    description: "monkey info displays VID 0x05AC, PID 0x024F, Product Name, serial, and composite interfaces in human and JSON modes"
    requirement: "DISC-02"
    verification:
      - kind: unit
        ref: "crates/monkey-cli/tests/cli_probe_test.rs#test_info_json_schema_and_identifiers"
        status: pass
      - kind: unit
        ref: "crates/monkey-cli/tests/cli_probe_test.rs#test_info_human_output_formatting"
        status: pass
    human_judgment: false
  - id: D2
    description: "monkey probe inspects device capability tuple (model, revision, firmware, transport state, capabilities) with zero writes"
    requirement: "DISC-03"
    verification:
      - kind: unit
        ref: "crates/monkey-cli/tests/cli_probe_test.rs#test_probe_json_schema_and_capability_tuple"
        status: pass
      - kind: unit
        ref: "crates/monkey-cli/tests/cli_probe_test.rs#test_probe_enforces_read_only_safety_invariant"
        status: pass
      - kind: unit
        ref: "crates/monkey-cli/tests/cli_probe_test.rs#test_probe_human_output_formatting"
        status: pass
    human_judgment: false
  - id: D3
    description: "Headless integration test suite verifies MockTransport safety invariants and non-zero exit on missing device"
    requirement: "DISC-02, DISC-03, DISC-04"
    verification:
      - kind: unit
        ref: "crates/monkey-cli/tests/cli_probe_test.rs#test_cli_no_device_found_error_exit"
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-09-13
status: complete
---

# Phase 1 Plan 03: monkey-cli Integration (info and probe), Formatted/JSON Output, and Headless Integration Tests Summary

**Stand-alone binary crate `crates/monkey-cli` implementing `monkey info` and `monkey probe` with human terminal tables and structured `--json` output, strictly enforced zero-write safety invariants, and 7 automated headless integration tests.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-09-13T07:12:00Z
- **Completed:** 2026-09-13T07:27:00Z
- **Tasks:** 3
- **Files modified:** 9
- **Commits:** 3

## Accomplishments

- Implemented `monkey-cli` binary crate with `clap` command parser supporting global flags (`--json`, `-v/--verbose`) and subcommands `info` and `probe` per D-11.
- Implemented `monkey info` command querying `find_monka_device_sets` to report hardware identifiers (VID `0x05ac`, PID `0x024f`, Product Name `RKGK890`, Manufacturer, Serial Number, Release Number) and dual composite HID interfaces (`0xFF68:0x0061` bulk pipe and `0x000C:0x0001`/`0xFFFF` control pipe) in terminal tables or structured JSON (`DeviceInfoOutput`).
- Implemented `monkey probe` command inspecting the device capability tuple (`model`, `hardware_revision`, `firmware_version`, `transport_state`, `capabilities`), validating read-only safety with zero flash writes, zero DFU/ISP triggers, and capability matrix indicators.
- Verified on live connected Monka keyboard hardware: `monkey info` and `monkey probe` both executed successfully against the host's connected Monka keyboard, outputting accurate descriptor metadata and transport state.
- Implemented 7 headless automated integration tests in `cli_probe_test.rs` covering JSON schema validation, capability tuples, `mock.assert_no_writes()` safety rail verification, human formatting, and missing device error exit codes (exit code 1 with clean stderr, zero panics). Total workspace tests now number 39 passing with zero failures and zero warnings.

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement tracer CLI binary with clap command tree and info subcommand** - `63d0ff1` (feat)
2. **Task 2: Complete monkey info and monkey probe commands with capability tuple inspection** - `c418f59` (feat)
3. **Task 3: Automated headless integration test suite for CLI commands using MockTransport** - `3080572` (test)

## Files Created/Modified

- `Cargo.lock` - Pinned updated dependencies (`clap`, `serde_json`, `tracing-subscriber`, etc.)
- `crates/monkey-cli/Cargo.toml` - Declared workspace dependencies, `monkey_cli` library target, and `monkey` binary target
- `crates/monkey-cli/src/lib.rs` - Library root exposing `commands` and `output` modules for testability
- `crates/monkey-cli/src/main.rs` - CLI entrypoint with `clap` parser, tracing initialization, command dispatch, and clean exit code handling
- `crates/monkey-cli/src/output.rs` - `OutputFormat` enum (`Human`, `Json`) and generic `Write` stream helpers
- `crates/monkey-cli/src/commands/mod.rs` - Module root exporting `info` and `probe`
- `crates/monkey-cli/src/commands/info.rs` - `monkey info` hardware inspection and `DeviceInfoOutput` formatting
- `crates/monkey-cli/src/commands/probe.rs` - `monkey probe` capability tuple inspection and read-only safety gate enforcement
- `crates/monkey-cli/tests/cli_probe_test.rs` - 7 automated headless integration tests for CLI commands and safety rails

## Decisions Made

- Added `[lib]` and `[[bin]]` configurations to `crates/monkey-cli` so integration tests in `tests/` can directly import `monkey_cli::commands` and `monkey_cli::output` while keeping binary execution accessible via `cargo run -p monkey-cli`.
- In `run_info_with_writer` and `run_probe_with_writer`, introduced `MONKEY_SIMULATE_EMPTY` test hook enabling headless end-to-end testing of the compiled binary's error exit path (code 1, user-friendly stderr message, no panics) on any host without requiring hardware disconnects.
- Tested against physical Monka keyboard attached to host: validated that `monkey info` and `monkey probe` execute seamlessly against real hardware (`OnMicro` OEM, VID `0x05ac`, PID `0x024f`).

## Deviations from Plan

None. All deliverables and behaviors specified in `01-03-PLAN.md` were implemented and verified with automated checks.

## Issues Encountered

None. All 39 automated unit and integration tests across the workspace pass cleanly.

## User Setup Required

None.

## Next Phase Readiness

- Phase 1 (Foundation and Hardware Probing) is now fully complete across all 3 plans:
  1. Plan 01: Cargo virtual workspace, synchronous `Transport` trait, zero-copy packet layouts, and `MockTransport`.
  2. Plan 02: Dual-interface HID discovery (`0xFF68:0x0061` vs `0x000C:0x0001`/`0xFFFF`), `macos-shared-device` non-exclusive opening, `HidTransport` with Report ID 0 prefixing, and connection heartbeat state detection.
  3. Plan 03: `crates/monkey-cli` with `monkey info` and `monkey probe`, human/JSON output formats, zero-write safety rails, and headless integration test suite.
- Ready for Phase 2: Protocol Codecs, Transaction Safety Rails & Benchmark Harness.

---
*Phase: 01-foundation-and-hardware-probing*
*Completed: 2026-09-13*
