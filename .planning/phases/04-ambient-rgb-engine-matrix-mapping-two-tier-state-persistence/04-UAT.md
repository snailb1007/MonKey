---
status: testing
phase: 04-ambient-rgb-engine-matrix-mapping-two-tier-state-persistence
source: [04-01-SUMMARY.md, 04-02-SUMMARY.md, 04-03-SUMMARY.md]
started: 2026-09-14T10:19:30+07:00
updated: 2026-09-14T10:19:30+07:00
---

## Current Test

number: 1
name: RGB Test Suite Passes
expected: |
  `cargo test -p monkey-core --test rgb_codecs_test --test rgb_manager_test` and `cargo test -p monkey-cli --test cli_rgb_test` pass cleanly (17/17 tests passing across codec, manager, and CLI suites).
awaiting: user response

## Tests

### 1. RGB Test Suite Passes
expected: `cargo test -p monkey-core --test rgb_codecs_test --test rgb_manager_test` and `cargo test -p monkey-cli --test cli_rgb_test` pass cleanly (17/17 tests passing across codec, manager, and CLI suites).
result: [pending]

### 2. Ambient Lighting RAM Preview
expected: Running `cargo run -p monkey-cli -- rgb set static --color ff0000 --brightness 80 --speed 50 --mock` succeeds with `target=mock`, sending 64-byte RGB packet (opcode 0x13, marker [0xAA, 0x55]) to volatile RAM at up to 30Hz with zero SPI NOR flash writes.
result: [pending]

### 3. Flash Commit Transaction & Debouncing
expected: Running `cargo run -p monkey-cli -- rgb set breathing --color green --commit --mock` initiates a 4-packet transaction (`04 18` -> `04 13` -> `04 02` -> `04 F0`) to commit settings to flash, and enforces a 500ms debounce guard between consecutive flash writes.
result: [pending]

### 4. Hardware Write-Consent Protection
expected: Running `cargo run -p monkey-cli -- rgb set static --color red` against real hardware without `--mock` and without `--allow-hardware-writes` fails with an explicit safety error requiring consent before modifying hardware state.
result: [pending]

### 5. Low-Battery Safety Gate
expected: When on wireless connection with battery below 20%, flash commit operations are blocked with a safety warning preventing corrupted flash writes during brownout, unless explicitly overridden with `--force`.
result: [pending]

### 6. Profile Backup & Restore
expected: Running `cargo run -p monkey-cli -- rgb save /tmp/rgb_test_profile.json --mock` saves valid JSON profile with schema_version=1 and lighting parameters. Running `cargo run -p monkey-cli -- rgb restore /tmp/rgb_test_profile.json --mock` loads and applies the configuration successfully.
result: [pending]

### 7. RGB Status Readback and JSON Format
expected: Running `cargo run -p monkey-cli -- rgb status --mock` reads back active lighting state via `04 F5`. Adding `--json` produces clean, machine-parseable JSON on stdout without log pollution.
result: [pending]

## Summary

total: 7
passed: 0
issues: 0
pending: 7
skipped: 0
blocked: 0

## Gaps

[none yet]
