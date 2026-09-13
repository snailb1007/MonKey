# Phase 02 Plan 03: Synthetic Benchmark Harness & CLI bench Command Summary

## Completed Work
1. **Benchmark Harness (`crates/monkey-core/src/bench/mod.rs`)**:
   - `BenchmarkConfig` carrying duration, frame, chunk, iteration, target-FPS and pacing budgets, with `validate()` rejecting configurations that would fail mid-run inside the packet codec.
   - `ThroughputReport` (frames, wire/payload bytes, KiB/s, FPS, avg and p95 frame time, target verdict) and `LatencyReport` (samples, min/max/avg, p50/p95/p99, jitter) with nearest-rank percentiles.
   - `run_bulk_streaming_bench` and `run_transaction_latency_bench` driving the `Transport` trait, so mock and HID hardware share one code path; `*_with_progress` variants expose per-frame / per-sample callbacks for CLI rendering.
   - `synthetic_lcd_frame()` and `encode_frame()` producing a deterministic 32,768-byte framebuffer sliced into 9 CRC-tagged 4096-byte wire reports.
2. **CLI Command (`crates/monkey-cli/src/commands/bench.rs`)**:
   - `monkey bench` with `--type <bulk|transaction|all>`, `--duration-secs`, `--frames`, `--iterations`, `--target-fps`, pacing overrides, and `--mock`; output honours the global `--json` flag.
   - `indicatif` progress bars drawn to stderr so a `--json` stdout stream stays machine-parseable.
   - Hardware bulk runs are gated behind `--allow-hardware-writes`, since streaming synthetic frames mutates the device LCD.
3. **Mock Transport (`crates/monkey-core/src/transport/mock.rs`)**:
   - Added `set_recording()` / `is_recording()` so duration-bound benchmarks do not accumulate unbounded call history; `assert_no_writes()` now refuses to vouch for a transport whose history was suppressed.
4. **Integration Testing (`crates/monkey-cli/tests/cli_bench_test.rs`)**:
   - 12 tests covering frame encoding and round-trip, byte accounting against recorded transport calls, per-iteration latency sampling, nearest-rank percentile math, empty-sample handling, config validation, duration-budget cutoff, runner selection per `--type`, human output sections, CLI JSON schema, CLI human rendering, and the hardware write-consent gate.

## Deviations
- The plan sketched bench logic split across core and CLI; the measurement runners live entirely in `monkey-core` over the `Transport` trait, leaving the CLI a thin argument/format wrapper. This removed a duplicated mock-vs-hardware code path.
- Chunk payload size defaults to `BulkPacket::MAX_PAYLOAD_LEN` (4088) rather than 4096. A 4096-byte payload overflows the 8-byte bulk header budget and made `BulkPacket::new` fail at runtime; on-wire report size is still 4096 bytes, so a frame spans 9 chunks rather than 8.
- Added `--allow-hardware-writes` (not in the plan) to keep the read-only-by-default posture of D-12.

## Verification
- `cargo test --workspace` — 69 tests pass (12 new in `cli_bench_test`).
- `cargo run -p monkey-cli -- bench --mock --json` emits valid JSON throughput and latency metrics.
- `cargo run -p monkey-cli -- bench --mock` renders formatted metrics for both runners.
