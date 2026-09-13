---
status: testing
phase: 02-protocol-codecs-transaction-safety-rails-benchmark-harness
source: 02-01-SUMMARY.md, 02-02-SUMMARY.md, 02-03-SUMMARY.md
started: 2026-09-13T12:31:25Z
updated: 2026-09-13T12:31:25Z
---

## Current Test

number: 1
name: Workspace Builds and Test Suite Passes
expected: |
  `cargo test --workspace` compiles every crate and target, and all tests pass
  (69 at the last committed phase-02 state).
awaiting: user response

## Tests

### 1. Workspace Builds and Test Suite Passes
expected: `cargo test --workspace` compiles every crate and target and all tests pass (69 at committed HEAD).
result: [pending]

### 2. Benchmark CLI — Human Output
expected: `cargo run -p monkey-cli -- bench --mock` prints formatted throughput and latency sections (frames, KiB/s, FPS, avg/p95 frame time, target verdict, p50/p95/p99 latency, jitter).
result: [pending]

### 3. Benchmark CLI — JSON Output
expected: `cargo run -p monkey-cli -- bench --mock --json` emits a single valid JSON document on stdout; indicatif progress bars render on stderr so the stdout stream stays machine-parseable.
result: [pending]

### 4. Benchmark Runner Selection
expected: `--type bulk` runs only the throughput bench, `--type transaction` only the latency bench, `--type all` (default) runs both. Flags `--duration-secs`, `--frames`, `--iterations`, `--target-fps` change the reported config accordingly.
result: [pending]

### 5. Hardware Write Consent Gate
expected: A bulk bench against real hardware refuses to run without `--allow-hardware-writes`, since streaming synthetic frames mutates the device LCD. Mock runs are unaffected.
result: [pending]

### 6. LCD Frame Chunking
expected: A 32,768-byte synthetic LCD framebuffer slices into CRC-tagged 4096-byte wire reports and reassembles byte-identically. Invalid chunk counts and short writes fail explicitly rather than continuing.
result: [pending]

### 7. Dangerous Command Whitelist
expected: Only whitelisted opcodes reach the transport; unverified/dangerous opcodes (e.g. `CommandId::Reboot`, DFU/ISP bricking opcodes) are rejected before any write, and the blocked-call counter increments.
result: [pending]

### 8. Flash Commit Debouncing
expected: `WriteMode::RamPreview` writes are unthrottled; `WriteMode::FlashCommit` writes are debounced at the configured interval (default 500ms), rejecting writes inside the window and recovering after it.
result: [pending]

## Summary

total: 8
passed: 0
issues: 0
pending: 8
skipped: 0
blocked: 0

## Gaps

[none yet]
