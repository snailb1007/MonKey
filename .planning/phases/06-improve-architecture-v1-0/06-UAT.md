---
status: complete
phase: 06-improve-architecture-v1-0
source:
  - .planning/phases/06-improve-architecture-v1-0/06-01-SUMMARY.md
  - .planning/phases/06-improve-architecture-v1-0/06-02-SUMMARY.md
  - .planning/phases/06-improve-architecture-v1-0/06-03-SUMMARY.md
  - .planning/phases/06-improve-architecture-v1-0/06-04-SUMMARY.md
started: 2026-09-17T04:15:00Z
updated: 2026-09-17T04:40:00Z
---

## Current Test
<!-- OVERWRITE each test - shows where we are -->

number: 9
name: Workspace regression suite and static analysis
expected: Workspace regression suite and static analysis
awaiting: none

## Tests

### 1. Architecture Documentation and GitNexus Code Graph Refresh
expected: Architecture documentation in `docs/dev/architecture.md` comprehensively documents the MonkaDevice deep coordinator, InterfacePolicy declarative enum, SafeTransport RAII guard façade, and POSIX exit code mappings. GitNexus code graph is refreshed and reflects the updated architecture.
result: pass

### 2. Monka 3075 Pro CLI Probe and Doctor Execution
expected: Running `cargo run -p monkey-cli -- probe` and `cargo run -p monkey-cli -- doctor` executes through MonkaDevice, probes hardware safely (0 flash writes), inspects system environment, and returns POSIX exit code 0.
result: pass

### 3. MonkaDevice lifecycle and InterfacePolicy resolution
expected: MonkaDevice lifecycle and InterfacePolicy resolution
result: pass
source: automated
coverage_id: D1

### 4. MonkaDevice deep operations and probe read-only safety invariant
expected: MonkaDevice deep operations and probe read-only safety invariant
result: pass
source: automated
coverage_id: D2

### 5. monkey-core::doctor consumes MonkaDevice::diagnose()
expected: monkey-core::doctor consumes MonkaDevice::diagnose()
result: pass
source: automated
coverage_id: D3

### 6. info and probe commands migrated to MonkaDevice and MONKEY_SIMULATE_EMPTY eradicated
expected: info and probe commands migrated to MonkaDevice and MONKEY_SIMULATE_EMPTY eradicated
result: pass
source: automated
coverage_id: CLI1

### 7. lcd, rgb, and bench commands migrated to MonkaDevice and ad-hoc test seams eradicated
expected: lcd, rgb, and bench commands migrated to MonkaDevice and ad-hoc test seams eradicated
result: pass
source: automated
coverage_id: CLI2

### 8. POSIX exit codes 0..=5 preserved with OpenError classification
expected: POSIX exit codes 0..=5 preserved with OpenError classification
result: pass
source: automated
coverage_id: CLI3

### 9. Workspace regression suite and static analysis
expected: Workspace regression suite and static analysis
result: pass
source: automated
coverage_id: V1

## Summary

total: 9
passed: 9
issues: 0
pending: 0
skipped: 0

## Gaps

[none yet]
