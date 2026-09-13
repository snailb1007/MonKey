//! `monkey bench` — synthetic throughput and latency benchmarking (REQ-BENCH-03).

use std::io::Write;
use std::time::Duration;

use clap::{Args, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use monkey_core::{
    find_monka_device_sets, init_hidapi, open_device_path, run_bulk_streaming_bench_with_progress,
    run_transaction_latency_bench_with_progress, BenchmarkConfig, HidTransport, LatencyReport,
    MockTransport, ThroughputReport, Transport,
};
use serde::Serialize;

use crate::output::OutputFormat;

/// Which runners to execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum BenchType {
    /// Bulk LCD frame streaming throughput on Interface A.
    Bulk,
    /// Host feature-report send latency on Interface B (not device RTT).
    Transaction,
    /// Both runners.
    All,
}

impl BenchType {
    fn as_str(self) -> &'static str {
        match self {
            BenchType::Bulk => "bulk",
            BenchType::Transaction => "transaction",
            BenchType::All => "all",
        }
    }

    fn runs_bulk(self) -> bool {
        matches!(self, BenchType::Bulk | BenchType::All)
    }

    fn runs_transaction(self) -> bool {
        matches!(self, BenchType::Transaction | BenchType::All)
    }
}

#[derive(Debug, Args)]
pub struct BenchArgs {
    /// Which benchmark runners to execute
    #[arg(long = "type", value_enum, default_value_t = BenchType::All)]
    pub bench_type: BenchType,

    /// Wall-clock ceiling per runner (0 disables the time limit)
    #[arg(long, default_value_t = 5)]
    pub duration_secs: u64,

    /// Synthetic LCD frames to stream (0 runs until the duration budget expires)
    #[arg(long, default_value_t = 30)]
    pub frames: usize,

    /// Feature-report sends sampled by the latency runner
    #[arg(long, default_value_t = 100)]
    pub iterations: usize,

    /// Frame rate the throughput runner is graded against
    #[arg(long, default_value_t = monkey_core::TARGET_FPS_MIN)]
    pub target_fps: f64,

    /// Host pacing delay between bulk chunks, in milliseconds
    #[arg(long, default_value_t = 0)]
    pub inter_chunk_delay_ms: u64,

    /// Host settle delay after each feature report, in milliseconds
    #[arg(long, default_value_t = 0)]
    pub inter_packet_delay_ms: u64,

    /// Run against an in-memory mock transport instead of real hardware
    #[arg(long)]
    pub mock: bool,

    /// Permit the bulk runner to stream synthetic frames to a real device
    #[arg(long)]
    pub allow_hardware_writes: bool,
}

impl BenchArgs {
    fn to_config(&self) -> BenchmarkConfig {
        BenchmarkConfig {
            duration: Duration::from_secs(self.duration_secs),
            frame_count: self.frames,
            iterations: self.iterations,
            target_fps: self.target_fps,
            inter_packet_delay: Duration::from_millis(self.inter_packet_delay_ms),
            inter_chunk_delay: Duration::from_millis(self.inter_chunk_delay_ms),
            ..BenchmarkConfig::default()
        }
    }
}

/// Configuration echoed back alongside the measurements so a JSON run is self-describing.
#[derive(Debug, Clone, Serialize)]
pub struct BenchConfigSummary {
    pub duration_secs: u64,
    pub frames: usize,
    pub iterations: usize,
    pub chunk_size: usize,
    pub chunks_per_frame: usize,
    pub target_fps: f64,
    pub inter_chunk_delay_ms: u64,
    pub inter_packet_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkReport {
    pub target: String,
    pub bench_type: String,
    pub config: BenchConfigSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throughput: Option<ThroughputReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency: Option<LatencyReport>,
}

pub fn run(args: BenchArgs, format: OutputFormat) -> anyhow::Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    run_bench(args, format, &mut handle)
}

/// Resolves the transport from `args`, runs the selected benchmarks, and writes the report.
pub fn run_bench<W: Write>(
    args: BenchArgs,
    format: OutputFormat,
    writer: &mut W,
) -> anyhow::Result<()> {
    let config = args.to_config();

    let report = if args.mock {
        let mut transport = MockTransport::new();
        // A duration-bound mock run dispatches frames as fast as the host allows;
        // keeping the call history would grow unbounded.
        transport.set_recording(false);
        run_bench_with_transport(&mut transport, &args, &config, "mock", format)?
    } else {
        if args.bench_type.runs_bulk() && !args.allow_hardware_writes {
            anyhow::bail!(
                "Bulk streaming writes synthetic frames to the device LCD. \
                 Re-run with --allow-hardware-writes to confirm, use --type transaction \
                 for a state-readback send-latency run, or use --mock."
            );
        }

        let api = init_hidapi()?;
        let sets = find_monka_device_sets(&api);
        let set = sets
            .first()
            .ok_or_else(|| anyhow::anyhow!("No Monka keyboards detected on the USB bus"))?;

        // Interface A carries the bulk pipe; Interface B carries the control pipe.
        let dev_info = if args.bench_type.runs_bulk() {
            set.interface_a.as_ref().or(set.interface_b.as_ref())
        } else {
            set.interface_b.as_ref().or(set.interface_a.as_ref())
        }
        .ok_or_else(|| anyhow::anyhow!("No suitable Monka interface found on the device set"))?;

        let mut transport = HidTransport::new(open_device_path(&api, dev_info)?);
        run_bench_with_transport(&mut transport, &args, &config, "hardware", format)?
    };

    format.write_to(writer, &format_bench_human(&report), &report)
}

/// Benchmark driver shared by the mock and hardware paths.
pub fn run_bench_with_transport(
    transport: &mut dyn Transport,
    args: &BenchArgs,
    config: &BenchmarkConfig,
    target: &str,
    format: OutputFormat,
) -> anyhow::Result<BenchmarkReport> {
    config.validate()?;

    let throughput = if args.bench_type.runs_bulk() {
        let bar = progress_bar(format, config.frame_count as u64, "streaming frames");
        let report = run_bulk_streaming_bench_with_progress(transport, config, |done, _| {
            bar.set_position(done as u64)
        })?;
        bar.finish_and_clear();
        Some(report)
    } else {
        None
    };

    let latency = if args.bench_type.runs_transaction() {
        let bar = progress_bar(format, config.iterations as u64, "sampling sends");
        let report = run_transaction_latency_bench_with_progress(transport, config, |done, _| {
            bar.set_position(done as u64)
        })?;
        bar.finish_and_clear();
        Some(report)
    } else {
        None
    };

    Ok(BenchmarkReport {
        target: target.to_string(),
        bench_type: args.bench_type.as_str().to_string(),
        config: BenchConfigSummary {
            duration_secs: config.duration.as_secs(),
            frames: config.frame_count,
            iterations: config.iterations,
            chunk_size: config.chunk_size,
            chunks_per_frame: config.chunks_per_frame(),
            target_fps: config.target_fps,
            inter_chunk_delay_ms: config.inter_chunk_delay.as_millis() as u64,
            inter_packet_delay_ms: config.inter_packet_delay.as_millis() as u64,
        },
        throughput,
        latency,
    })
}

/// Draws to stderr so a `--json` stdout stream stays machine-parseable.
fn progress_bar(format: OutputFormat, len: u64, what: &str) -> ProgressBar {
    if format == OutputFormat::Json || len == 0 {
        return ProgressBar::hidden();
    }

    let bar = ProgressBar::new(len);
    if let Ok(style) = ProgressStyle::with_template("  {msg:<22} [{bar:32}] {pos}/{len}") {
        bar.set_style(style.progress_chars("=> "));
    }
    bar.set_message(what.to_string());
    bar
}

pub fn format_bench_human(report: &BenchmarkReport) -> String {
    let mut out = String::new();
    let rule = "=".repeat(60);

    out.push_str(&format!("{rule}\n"));
    out.push_str("                  MonKey Protocol Benchmark\n");
    out.push_str(&format!("{rule}\n"));
    out.push_str(&format!(
        "  Target:                     {}\n",
        report.target
    ));
    out.push_str(&format!(
        "  Benchmark:                  {}\n",
        report.bench_type
    ));

    if let Some(t) = &report.throughput {
        out.push_str(&format!("{rule}\n"));
        out.push_str("  Bulk Streaming (Interface A)\n");
        out.push_str(&format!("    Frames streamed:          {}\n", t.frames));
        out.push_str(&format!(
            "    Chunks per frame:         {}\n",
            t.chunks_per_frame
        ));
        out.push_str(&format!(
            "    Bytes on wire:            {}\n",
            t.total_bytes
        ));
        out.push_str(&format!(
            "    Elapsed:                  {:.3} s\n",
            t.elapsed_secs
        ));
        out.push_str(&format!(
            "    Throughput:               {:.1} KiB/s\n",
            t.kib_per_sec
        ));
        out.push_str(&format!(
            "    Frame rate:               {:.1} FPS\n",
            t.frames_per_sec
        ));
        out.push_str(&format!(
            "    Frame time (avg):         {:.3} ms\n",
            t.avg_frame_ms
        ));
        out.push_str(&format!(
            "    Frame time (p95):         {:.3} ms\n",
            t.p95_frame_ms
        ));
        out.push_str(&format!(
            "    Target {:.0} FPS:            {}\n",
            t.target_fps,
            if t.meets_target_fps {
                "MET [ok]"
            } else {
                "MISSED [!]"
            }
        ));
    }

    if let Some(l) = &report.latency {
        out.push_str(&format!("{rule}\n"));
        out.push_str("  Command Send Latency (Interface B; not device RTT)\n");
        out.push_str(&format!("    Samples:                  {}\n", l.samples));
        out.push_str(&format!("    Latency (min):            {} us\n", l.min_us));
        out.push_str(&format!("    Latency (p50):            {} us\n", l.p50_us));
        out.push_str(&format!("    Latency (p95):            {} us\n", l.p95_us));
        out.push_str(&format!("    Latency (p99):            {} us\n", l.p99_us));
        out.push_str(&format!("    Latency (max):            {} us\n", l.max_us));
        out.push_str(&format!(
            "    Latency (avg):            {:.1} us\n",
            l.avg_us
        ));
        out.push_str(&format!(
            "    Jitter (p95 - p50):       {} us\n",
            l.jitter_us()
        ));
    }

    out.push_str(&rule);
    out
}
