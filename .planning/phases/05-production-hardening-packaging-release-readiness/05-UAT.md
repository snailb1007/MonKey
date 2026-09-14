---
status: testing
phase: 05-production-hardening-packaging-release-readiness
source: [05-01-SUMMARY.md, 05-02-SUMMARY.md, 05-03-SUMMARY.md]
started: 2026-09-14T03:20:00Z
updated: 2026-09-14T03:20:00Z
---

## Current Test

number: 1
name: Mock Pre-Flight Diagnostics Display
expected: |
  Running `cargo run -p monkey-cli -- doctor --mock` displays formatted system and hardware diagnostics covering Host OS, USB HID subsystem, Monka 3075 Pro detection, Interface A, Interface B, and Hardware Safety Rails, concluding with Status: HEALTHY (6 passed, 0 failed).
awaiting: user response

## Tests

### 1. Mock Pre-Flight Diagnostics Display
expected: Running `cargo run -p monkey-cli -- doctor --mock` displays formatted system and hardware diagnostics covering Host OS, USB HID subsystem, Monka 3075 Pro detection, Interface A, Interface B, and Hardware Safety Rails, concluding with Status: HEALTHY (6 passed, 0 failed).
result: [pending]

### 2. Machine-Readable Diagnostics JSON
expected: Running `cargo run -p monkey-cli -- doctor --mock --json` emits a valid JSON document containing `platform`, `checks` array with individual test statuses, and `summary` with `all_healthy: true`.
result: [pending]

### 3. Live Hardware Diagnostics & Remediation Guidance
expected: Running `cargo run -p monkey-cli -- doctor` against the host detects current keyboard connection state (e.g. 2.4GHz wireless dongle), reports Interface A and Interface B accessibility, and displays actionable remediation guidance for any warnings.
result: [pending]

### 4. Shell Completion Generation
expected: Running `cargo run -p monkey-cli -- completions zsh` (or bash, fish, elvish, powershell) outputs a valid shell completion script on stdout including all CLI subcommands (`info`, `probe`, `bench`, `lcd`, `rgb`, `doctor`, `completions`).
result: [pending]

### 5. Standardized POSIX Exit Codes
expected: CLI commands return standardized exit codes on errors (e.g. `cargo run -p monkey-cli -- invalid_cmd` exits with code 2 for CLI usage error). Domain failures map to 0 (success), 1 (general), 2 (usage), 3 (no device), 4 (permission), 5 (safety blocked).
result: [pending]

### 6. Dependency & License Compliance Auditing
expected: Running `cargo deny check bans licenses sources` passes with `bans ok, licenses ok, sources ok`, verifying zero banned dependencies, wildcard versions, or non-compliant licenses.
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0
blocked: 0

## Gaps

[none yet]
