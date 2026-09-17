---
phase: 06-improve-architecture-v1-0
plan: 03
subsystem: cli-commands
tags: [monka-device, cli-migration, env-var-eradication, test-seams, exit-codes]

requires:
  - phase: 06-improve-architecture-v1-0
    provides: MonkaDevice coordinator and InterfacePolicy
provides:
  - Unified CLI command acquisition via MonkaDevice::open and MonkaDevice::from_transport
  - Eradication of MONKEY_SIMULATE_EMPTY from codebase
  - Elimination of 5 ad-hoc test seams (run_probe_with_transport, run_bench_with_transport, probe_device_with_transport, resolve_transport, stream_one)
  - Full preservation of standardized POSIX exit codes 0 through 5
  - Thread-safe SharedMockTransport for post-acquisition mock inspection
affects: [monkey-cli, monkey-core]

actuals:
  tasks: 3
  commits: 1

tech-stack:
  added: []
  patterns: [device-coordinator-consumption, seam-eradication, shared-mock-wrapper]

key-files:
  created: []
  modified:
    - crates/monkey-cli/src/commands/bench.rs
    - crates/monkey-cli/src/commands/info.rs
    - crates/monkey-cli/src/commands/lcd.rs
    - crates/monkey-cli/src/commands/probe.rs
    - crates/monkey-cli/src/commands/rgb.rs
    - crates/monkey-cli/src/error.rs
    - crates/monkey-cli/tests/cli_bench_test.rs
    - crates/monkey-cli/tests/cli_exit_codes_test.rs
    - crates/monkey-cli/tests/cli_probe_test.rs
    - crates/monkey-core/src/lib.rs
    - crates/monkey-core/src/transport/mock.rs
    - crates/monkey-core/src/transport/mod.rs

key-decisions:
  - "D-02: All CLI commands acquire hardware via MonkaDevice::open(policy) using InterfacePolicy."
  - "D-03: OpenError variants map directly to standardized POSIX exit codes (0 through 5)."
  - "D-04: Deep operations executed through MonkaDevice methods rather than raw sub-managers."
  - "D-05: MONKEY_SIMULATE_EMPTY and fragmented test seams eradicated across all CLI commands."

requirements-completed: [ARCH-01]

coverage:
  - id: CLI1
    description: "info and probe commands migrated to MonkaDevice and MONKEY_SIMULATE_EMPTY eradicated"
    requirement: "ARCH-01"
    verification:
      - kind: unit
        ref: "crates/monkey-cli/tests/cli_probe_test.rs#test_classify_no_device_open_error"
        status: pass
    human_judgment: false
  - id: CLI2
    description: "lcd, rgb, and bench commands migrated to MonkaDevice and ad-hoc test seams eradicated"
    requirement: "ARCH-01"
    verification:
      - kind: integration
        ref: "crates/monkey-cli/tests/cli_bench_test.rs#test_bench_type_selects_runners"
        status: pass
    human_judgment: false
  - id: CLI3
    description: "POSIX exit codes 0..=5 preserved with OpenError classification"
    requirement: "ARCH-01"
    verification:
      - kind: unit
        ref: "crates/monkey-cli/tests/cli_exit_codes_test.rs#test_classify_no_device_error"
        status: pass
    human_judgment: false

duration: 12 min
completed: 2026-09-17
status: complete
---

# 06-03 Summary: CLI Migration, Env-Var Hack Eradication, and Test Seam Unification

**Migrated all CLI commands (`info`, `probe`, `lcd`, `rgb`, `bench`, `doctor`) to the unified `MonkaDevice` abstraction, eradicated `MONKEY_SIMULATE_EMPTY`, eliminated 5 ad-hoc test seams, and verified 100% POSIX exit code fidelity.**

## Accomplishments
- Migrated `info` and `probe` to `MonkaDevice::discover()` and `MonkaDevice::open(InterfacePolicy::RequireB)`.
- Eradicated `MONKEY_SIMULATE_EMPTY` entirely from `info.rs` and `probe.rs` (0 references remaining in repository), eliminating covert test branching in production binaries (T-06-05 mitigation).
- Eradicated 5 fragmented ad-hoc test seams per D-05:
  - `run_probe_with_transport` and `probe_device_with_transport` in `probe.rs`
  - `open_lcd_transport` and `stream_one` in `lcd.rs`
  - `resolve_transport` and `TransportResolution` in `rgb.rs`
  - `run_bench_with_transport` in `bench.rs`
- Migrated `lcd`, `rgb`, and `bench` commands to acquire hardware handles through `MonkaDevice::open(policy)` or `MonkaDevice::from_transport(mock)` with required `--allow-hardware-writes` consent gates (T-06-06 mitigation).
- Introduced `SharedMockTransport` in `monkey-core` to enable inspection of transport calls and assertion of zero writes (`assert_no_writes()`) after moving into `MonkaDevice`.
- Verified exit code classification fidelity in `cli_exit_codes_test.rs` covering all 6 exit codes (0 through 5) including typed `OpenError` variants.

## Verification
- `cargo test -p monkey-cli`: all 42 tests passed, 0 failures.
- `cargo clippy --all-targets -- -D warnings`: 0 warnings across workspace.

## Self-Check: PASSED
