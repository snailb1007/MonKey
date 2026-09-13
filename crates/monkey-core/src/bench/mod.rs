//! Synthetic benchmark harness for bulk throughput and transaction latency.
//!
//! Every runner drives the [`Transport`] trait, so the same code path measures a
//! [`MockTransport`](crate::transport::MockTransport) (hardware-isolated host ceiling)
//! and real [`HidTransport`](crate::transport::HidTransport) hardware without branching.

use std::time::{Duration, Instant};

use serde::Serialize;

use crate::error::{MonkeyError, Result};
use crate::protocol::framing::ChunkIterator;
use crate::protocol::safety::{SafetyRails, WriteMode};
use crate::protocol::transaction::TransactionManager;
use crate::protocol::types::{BulkChunkPacket, CommandId, FeatureReportPacket, BULK_CHUNK_SIZE};
use crate::transport::Transport;

/// Raw LCD framebuffer size: 128 x 128 pixels at 16 bits per pixel.
pub const LCD_FRAME_BYTES: usize = 32_768;

/// Lower bound of the LCD animation target band (REQ-BENCH-01).
pub const TARGET_FPS_MIN: f64 = 10.0;

/// Upper bound of the LCD animation target band (REQ-BENCH-01).
pub const TARGET_FPS_MAX: f64 = 15.0;

/// Tuning knobs shared by both benchmark runners.
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkConfig {
    /// Wall-clock ceiling for a single runner. [`Duration::ZERO`] means "no time limit".
    pub duration: Duration,
    /// Synthetic LCD frames to stream in the throughput runner. Zero means "duration bound only".
    pub frame_count: usize,
    /// Payload bytes carried per raw bulk packet (unused trailing bytes are zero-padded).
    pub chunk_size: usize,
    /// Feature-report sends performed by the latency runner (not device RTT).
    pub iterations: usize,
    /// Frame rate the throughput runner is graded against.
    pub target_fps: f64,
    /// Host-side settle delay applied after each feature report.
    pub inter_packet_delay: Duration,
    /// Host-side pacing delay applied between bulk chunks.
    pub inter_chunk_delay: Duration,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(5),
            frame_count: 30,
            chunk_size: BULK_CHUNK_SIZE,
            iterations: 100,
            target_fps: TARGET_FPS_MIN,
            inter_packet_delay: Duration::ZERO,
            inter_chunk_delay: Duration::ZERO,
        }
    }
}

impl BenchmarkConfig {
    /// Number of bulk packets one synthetic LCD frame is sliced into.
    pub fn chunks_per_frame(&self) -> usize {
        if self.chunk_size == 0 {
            0
        } else {
            LCD_FRAME_BYTES.div_ceil(self.chunk_size)
        }
    }

    /// Rejects configurations that would fail mid-run inside the packet codec.
    pub fn validate(&self) -> Result<()> {
        if self.chunk_size == 0 || self.chunk_size > BULK_CHUNK_SIZE {
            return Err(MonkeyError::Protocol(format!(
                "Benchmark chunk size {} must be between 1 and {}",
                self.chunk_size, BULK_CHUNK_SIZE
            )));
        }

        let chunks = self.chunks_per_frame();
        if chunks > u8::MAX as usize {
            return Err(MonkeyError::Protocol(format!(
                "Chunk size {} slices an LCD frame into {} chunks, exceeding the {} chunk index space",
                self.chunk_size,
                chunks,
                u8::MAX
            )));
        }

        if self.frame_count == 0 && self.duration.is_zero() {
            return Err(MonkeyError::Protocol(
                "Benchmark requires a frame budget or a duration budget".to_string(),
            ));
        }

        Ok(())
    }

    /// True while the throughput runner still has frame and duration budget left.
    fn has_budget(&self, frames_done: usize, elapsed: Duration) -> bool {
        if self.frame_count > 0 && frames_done >= self.frame_count {
            return false;
        }
        if !self.duration.is_zero() && elapsed >= self.duration {
            return false;
        }
        true
    }
}

/// Bulk streaming throughput measurement (REQ-BENCH-01).
#[derive(Debug, Clone, Serialize)]
pub struct ThroughputReport {
    /// Frames actually streamed before the budget ran out.
    pub frames: usize,
    /// Bulk packets sent per frame.
    pub chunks_per_frame: usize,
    /// On-wire bytes pushed through the transport, including zero padding.
    pub total_bytes: u64,
    /// Framebuffer bytes delivered, excluding zero padding.
    pub payload_bytes: u64,
    pub elapsed_secs: f64,
    pub bytes_per_sec: f64,
    pub kib_per_sec: f64,
    pub frames_per_sec: f64,
    pub avg_frame_ms: f64,
    pub p95_frame_ms: f64,
    pub target_fps: f64,
    /// Whether the achieved frame rate cleared [`BenchmarkConfig::target_fps`].
    pub meets_target_fps: bool,
}

/// Host feature-report send latency, including configured settle delay.
/// No response is read, so these samples do not establish device roundtrip latency.
#[derive(Debug, Clone, Serialize)]
pub struct LatencyReport {
    pub samples: usize,
    pub min_us: u64,
    pub max_us: u64,
    pub avg_us: f64,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
}

impl LatencyReport {
    /// Builds a report from unsorted microsecond samples.
    pub fn from_samples(mut samples_us: Vec<u64>) -> Self {
        if samples_us.is_empty() {
            return Self {
                samples: 0,
                min_us: 0,
                max_us: 0,
                avg_us: 0.0,
                p50_us: 0,
                p95_us: 0,
                p99_us: 0,
            };
        }

        samples_us.sort_unstable();
        let total: u64 = samples_us.iter().sum();

        Self {
            samples: samples_us.len(),
            min_us: samples_us[0],
            max_us: samples_us[samples_us.len() - 1],
            avg_us: total as f64 / samples_us.len() as f64,
            p50_us: percentile(&samples_us, 50.0),
            p95_us: percentile(&samples_us, 95.0),
            p99_us: percentile(&samples_us, 99.0),
        }
    }

    /// Spread between the tail and the median, a proxy for pacing jitter.
    pub fn jitter_us(&self) -> u64 {
        self.p95_us.saturating_sub(self.p50_us)
    }
}

/// Nearest-rank percentile over an ascending slice.
fn percentile(sorted: &[u64], pct: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = ((pct / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

/// Deterministic synthetic framebuffer standing in for real LCD content.
pub fn synthetic_lcd_frame() -> Vec<u8> {
    (0..LCD_FRAME_BYTES).map(|i| (i % 251) as u8).collect()
}

/// Encodes bytes into raw bulk reports. Sequence metadata stays on the host.
pub fn encode_frame(frame: &[u8], chunk_size: usize) -> Result<Vec<BulkChunkPacket>> {
    Ok(ChunkIterator::new(frame, chunk_size)?
        .map(|chunk| chunk.packet)
        .collect())
}

/// Streams synthetic LCD frames and measures sustained bulk throughput.
pub fn run_bulk_streaming_bench(
    transport: &mut dyn Transport,
    config: &BenchmarkConfig,
) -> Result<ThroughputReport> {
    run_bulk_streaming_bench_with_progress(transport, config, |_, _| {})
}

/// [`run_bulk_streaming_bench`] with a `(frames_done, frame_budget)` progress callback.
///
/// `frame_budget` is zero for duration-bound runs, where no total is known up front.
pub fn run_bulk_streaming_bench_with_progress<F>(
    transport: &mut dyn Transport,
    config: &BenchmarkConfig,
    mut on_frame: F,
) -> Result<ThroughputReport>
where
    F: FnMut(usize, usize),
{
    config.validate()?;

    // Encoding happens once so the timed loop measures transport dispatch, not chunking.
    let frame = synthetic_lcd_frame();
    let packets = encode_frame(&frame, config.chunk_size)?;

    let rails = SafetyRails::default().with_hardware_writes_permitted(true);
    let mut manager = TransactionManager::new(transport, &rails)
        .with_delays(config.inter_packet_delay, config.inter_chunk_delay);

    let mut frame_times_us: Vec<u64> = Vec::new();
    let start = Instant::now();

    while config.has_budget(frame_times_us.len(), start.elapsed()) {
        let frame_start = Instant::now();
        manager.stream_bulk_chunks(&packets, |_, _| {})?;
        frame_times_us.push(frame_start.elapsed().as_micros() as u64);
        on_frame(frame_times_us.len(), config.frame_count);
    }

    let elapsed = start.elapsed();
    let frames = frame_times_us.len();
    let total_bytes = (frames * packets.len() * BULK_CHUNK_SIZE) as u64;
    let payload_bytes = (frames * frame.len()) as u64;
    let elapsed_secs = elapsed.as_secs_f64();

    let (bytes_per_sec, frames_per_sec) = if elapsed_secs > 0.0 {
        (
            total_bytes as f64 / elapsed_secs,
            frames as f64 / elapsed_secs,
        )
    } else {
        (0.0, 0.0)
    };

    let avg_frame_ms = if frames > 0 {
        frame_times_us.iter().sum::<u64>() as f64 / frames as f64 / 1000.0
    } else {
        0.0
    };

    let mut sorted = frame_times_us;
    sorted.sort_unstable();

    Ok(ThroughputReport {
        frames,
        chunks_per_frame: packets.len(),
        total_bytes,
        payload_bytes,
        elapsed_secs,
        bytes_per_sec,
        kib_per_sec: bytes_per_sec / 1024.0,
        frames_per_sec,
        avg_frame_ms,
        p95_frame_ms: percentile(&sorted, 95.0) as f64 / 1000.0,
        target_fps: config.target_fps,
        meets_target_fps: frames > 0 && frames_per_sec >= config.target_fps,
    })
}

/// Measures host feature-report send latency on the control pipe, not device RTT.
pub fn run_transaction_latency_bench(
    transport: &mut dyn Transport,
    config: &BenchmarkConfig,
) -> Result<LatencyReport> {
    run_transaction_latency_bench_with_progress(transport, config, |_, _| {})
}

/// [`run_transaction_latency_bench`] with a `(samples_done, iterations)` progress callback.
pub fn run_transaction_latency_bench_with_progress<F>(
    transport: &mut dyn Transport,
    config: &BenchmarkConfig,
    mut on_sample: F,
) -> Result<LatencyReport>
where
    F: FnMut(usize, usize),
{
    let iterations = config.iterations.max(1);

    // State readback opcode from Phase 2 CONTEXT.md. No response is awaited here;
    // hardware protocol/roundtrip verification remains a separate task.
    let probe = FeatureReportPacket::new(CommandId::StateReadback as u8);

    let rails = SafetyRails::default().with_hardware_writes_permitted(true);
    let mut manager = TransactionManager::new(transport, &rails)
        .with_delays(config.inter_packet_delay, config.inter_chunk_delay);

    let mut samples_us = Vec::with_capacity(iterations);
    let start = Instant::now();

    for _ in 0..iterations {
        if !config.duration.is_zero() && start.elapsed() >= config.duration {
            break;
        }

        let sample_start = Instant::now();
        manager.send_feature_command(&probe, WriteMode::RamPreview)?;
        samples_us.push(sample_start.elapsed().as_micros() as u64);
        on_sample(samples_us.len(), iterations);
    }

    Ok(LatencyReport::from_samples(samples_us))
}
