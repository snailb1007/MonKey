//! Coverage for the synthetic benchmark harness and the `monkey bench` command
//! (REQ-BENCH-01, REQ-BENCH-02, REQ-BENCH-03).

use std::process::Command;
use std::time::Duration;

use monkey_cli::commands::bench::{
    format_bench_human, run_bench_with_transport, BenchArgs, BenchType,
};
use monkey_cli::output::OutputFormat;
use monkey_core::bench::{
    encode_frame, run_bulk_streaming_bench, run_transaction_latency_bench, synthetic_lcd_frame,
    BenchmarkConfig, LatencyReport, LCD_FRAME_BYTES,
};
use monkey_core::transport::{MockTransport, TransportCall};
use monkey_core::BULK_CHUNK_SIZE;

fn test_config(frames: usize, iterations: usize) -> BenchmarkConfig {
    BenchmarkConfig {
        duration: Duration::ZERO,
        frame_count: frames,
        iterations,
        inter_packet_delay: Duration::ZERO,
        inter_chunk_delay: Duration::ZERO,
        ..BenchmarkConfig::default()
    }
}

fn test_args(bench_type: BenchType) -> BenchArgs {
    BenchArgs {
        bench_type,
        duration_secs: 0,
        frames: 3,
        iterations: 8,
        target_fps: 10.0,
        inter_chunk_delay_ms: 0,
        inter_packet_delay_ms: 0,
        mock: true,
        allow_hardware_writes: false,
    }
}

#[test]
fn test_synthetic_frame_encodes_into_valid_bulk_packets() {
    let frame = synthetic_lcd_frame();
    assert_eq!(frame.len(), LCD_FRAME_BYTES, "LCD frame must be 128x128x2");

    let packets = encode_frame(&frame, BULK_CHUNK_SIZE)
        .expect("default chunk size must produce encodable packets");

    assert_eq!(
        packets.len(),
        8,
        "32768 bytes at 4096 payload bytes spans 8 chunks"
    );
    for packet in &packets {
        assert_eq!(
            packet.data.len(),
            BULK_CHUNK_SIZE,
            "every chunk must occupy a full 4096-byte wire report"
        );
    }

    // Payload bytes must round-trip back to the original framebuffer.
    let rebuilt: Vec<u8> = packets.iter().flat_map(|p| p.data).collect();
    assert_eq!(rebuilt, frame);
}

#[test]
fn test_bulk_bench_accounting_matches_transport_calls() {
    let config = test_config(4, 0);
    let mut transport = MockTransport::new();

    let report = run_bulk_streaming_bench(&mut transport, &config)
        .expect("bulk benchmark must succeed against the mock transport");

    assert_eq!(report.frames, 4);
    assert_eq!(report.chunks_per_frame, config.chunks_per_frame());
    assert_eq!(report.payload_bytes, (4 * LCD_FRAME_BYTES) as u64);
    assert_eq!(
        report.total_bytes,
        (4 * report.chunks_per_frame * BULK_CHUNK_SIZE) as u64
    );
    assert!(report.bytes_per_sec > 0.0, "throughput must be positive");
    assert!(report.frames_per_sec > 0.0, "frame rate must be positive");

    let bulk_writes = transport
        .calls()
        .iter()
        .filter(|c| matches!(c, TransportCall::WriteBulk { .. }))
        .count();
    assert_eq!(
        bulk_writes,
        4 * report.chunks_per_frame,
        "one bulk write per chunk per frame"
    );
}

#[test]
fn test_latency_bench_samples_every_iteration() {
    let config = test_config(0, 25);
    let mut transport = MockTransport::new();

    let report = run_transaction_latency_bench(&mut transport, &config)
        .expect("latency benchmark must succeed against the mock transport");

    assert_eq!(report.samples, 25);
    assert!(report.min_us <= report.p50_us);
    assert!(report.p50_us <= report.p95_us);
    assert!(report.p95_us <= report.p99_us);
    assert!(report.p99_us <= report.max_us);

    let feature_writes = transport
        .calls()
        .iter()
        .filter(|c| matches!(c, TransportCall::SendFeature { .. }))
        .count();
    assert_eq!(feature_writes, 25);
    for call in transport.calls() {
        if let TransportCall::SendFeature { data } = call {
            assert_eq!(data[1], 0xF5);
            assert_eq!(&data[14..16], &[0xAA, 0x55]);
        }
    }
}

#[test]
fn test_latency_percentiles_use_nearest_rank() {
    let report = LatencyReport::from_samples((1..=100).collect());

    assert_eq!(report.samples, 100);
    assert_eq!(report.min_us, 1);
    assert_eq!(report.max_us, 100);
    assert_eq!(report.p50_us, 50);
    assert_eq!(report.p95_us, 95);
    assert_eq!(report.p99_us, 99);
    assert_eq!(report.avg_us, 50.5);
    assert_eq!(report.jitter_us(), 45);
}

#[test]
fn test_empty_latency_samples_do_not_panic() {
    let report = LatencyReport::from_samples(Vec::new());
    assert_eq!(report.samples, 0);
    assert_eq!(report.p95_us, 0);
    assert_eq!(report.avg_us, 0.0);
}

#[test]
fn test_config_validation_rejects_unencodable_chunk_size() {
    // Raw reports carry at most 4096 bytes.
    let bad = BenchmarkConfig {
        chunk_size: BULK_CHUNK_SIZE + 1,
        ..test_config(1, 1)
    };
    let err = bad
        .validate()
        .expect_err("oversized chunk size must be rejected");
    assert!(err.to_string().contains("chunk size"), "got: {err}");

    let no_budget = BenchmarkConfig {
        duration: Duration::ZERO,
        frame_count: 0,
        ..BenchmarkConfig::default()
    };
    assert!(
        no_budget.validate().is_err(),
        "a run with neither frame nor duration budget must be rejected"
    );

    assert!(test_config(1, 1).validate().is_ok());
}

#[test]
fn test_duration_budget_stops_the_bulk_runner() {
    let config = BenchmarkConfig {
        duration: Duration::from_millis(20),
        frame_count: 0,
        ..test_config(0, 0)
    };
    let mut transport = MockTransport::new();
    transport.set_recording(false);

    let report =
        run_bulk_streaming_bench(&mut transport, &config).expect("duration-bound run must succeed");

    assert!(
        report.frames > 0,
        "at least one frame should complete in 20ms"
    );
    assert!(
        report.elapsed_secs < 5.0,
        "runner must stop on the duration budget, took {}s",
        report.elapsed_secs
    );
}

#[test]
fn test_bench_type_selects_runners() {
    let config = test_config(2, 5);

    let mut transport = MockTransport::new();
    let bulk_only = run_bench_with_transport(
        &mut transport,
        &test_args(BenchType::Bulk),
        &config,
        "mock",
        OutputFormat::Json,
    )
    .expect("bulk run must succeed");
    assert!(bulk_only.throughput.is_some());
    assert!(
        bulk_only.latency.is_none(),
        "--type bulk must not sample latency"
    );

    let mut transport = MockTransport::new();
    let txn_only = run_bench_with_transport(
        &mut transport,
        &test_args(BenchType::Transaction),
        &config,
        "mock",
        OutputFormat::Json,
    )
    .expect("transaction run must succeed");
    assert!(txn_only.latency.is_some());
    assert!(
        txn_only.throughput.is_none(),
        "--type transaction must not stream frames"
    );

    let mut transport = MockTransport::new();
    let all = run_bench_with_transport(
        &mut transport,
        &test_args(BenchType::All),
        &config,
        "mock",
        OutputFormat::Json,
    )
    .expect("combined run must succeed");
    assert!(all.throughput.is_some() && all.latency.is_some());
    assert_eq!(all.bench_type, "all");
    assert_eq!(all.target, "mock");
}

#[test]
fn test_human_output_reports_both_sections() {
    let config = test_config(2, 5);
    let mut transport = MockTransport::new();
    let report = run_bench_with_transport(
        &mut transport,
        &test_args(BenchType::All),
        &config,
        "mock",
        OutputFormat::Human,
    )
    .expect("combined run must succeed");

    let text = format_bench_human(&report);
    for expected in [
        "MonKey Protocol Benchmark",
        "Bulk Streaming (Interface A)",
        "Command Send Latency (Interface B; not device RTT)",
        "Frame rate:",
        "Latency (p95):",
        "Jitter (p95 - p50):",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
    }
}

#[test]
fn test_cli_bench_mock_emits_valid_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_monkey"))
        .args([
            "bench",
            "--mock",
            "--json",
            "--frames",
            "3",
            "--iterations",
            "5",
        ])
        .output()
        .expect("failed to invoke the monkey binary");

    assert!(
        output.status.success(),
        "bench --mock exited with {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be valid JSON");

    assert_eq!(parsed["target"], "mock");
    assert_eq!(parsed["bench_type"], "all");
    assert_eq!(parsed["throughput"]["frames"], 3);
    assert_eq!(parsed["latency"]["samples"], 5);
    assert!(parsed["throughput"]["bytes_per_sec"].as_f64().unwrap() > 0.0);
    assert_eq!(parsed["config"]["chunks_per_frame"], 8);
}

#[test]
fn test_cli_bench_mock_human_output_renders() {
    let output = Command::new(env!("CARGO_BIN_EXE_monkey"))
        .args(["bench", "--mock", "--frames", "2", "--iterations", "4"])
        .output()
        .expect("failed to invoke the monkey binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("MonKey Protocol Benchmark"));
    assert!(stdout.contains("Frames streamed:"));
    assert!(!stdout.contains('{'), "human output must not leak JSON");
}

#[test]
fn test_hardware_bulk_run_requires_explicit_write_consent() {
    let output = Command::new(env!("CARGO_BIN_EXE_monkey"))
        .args(["bench", "--type", "bulk", "--frames", "1"])
        .output()
        .expect("failed to invoke the monkey binary");

    assert!(
        !output.status.success(),
        "hardware bulk run must not proceed unguarded"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--allow-hardware-writes"),
        "error must name the consent flag, got: {stderr}"
    );
}
