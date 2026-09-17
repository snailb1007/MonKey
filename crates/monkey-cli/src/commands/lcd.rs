//! `monkey lcd` image, animation, and diagnostic pattern commands.

use std::io::Write;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Context;
use clap::{Args, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use monkey_core::device::{InterfacePolicy, MonkaDevice};
use monkey_core::lcd::{
    decode_gif_frames, generate_test_pattern_with_format, load_image_frame, ColorFormat,
    FpsRegulator, LcdPacingConfig, TestPatternType,
};
use monkey_core::transport::MockTransport;
use serde::Serialize;

use crate::output::OutputFormat;

/// Default animation rate, mid-band of the 10-15 FPS hardware-safe range.
const DEFAULT_LCD_FPS: u32 = 12;

#[derive(Debug, Args)]
pub struct LcdArgs {
    #[command(subcommand)]
    pub command: LcdCommand,
}

#[derive(Debug, Subcommand)]
pub enum LcdCommand {
    /// Load, crop, resize, and display one image.
    Image(ImageArgs),
    /// Decode and play an animated GIF.
    Anim(AnimArgs),
    /// Display a built-in LCD diagnostic pattern.
    TestPattern(TestPatternArgs),
}

#[derive(Debug, Args)]
pub struct ImageArgs {
    pub path: PathBuf,
    #[arg(long)]
    pub dither: bool,
    #[arg(long)]
    pub big_endian: bool,
    #[arg(long)]
    pub mock: bool,
    #[arg(long)]
    pub allow_hardware_writes: bool,
    /// Delay between raw LCD chunks, in milliseconds (0–8).
    #[arg(long, default_value_t = 3)]
    pub inter_chunk_delay_ms: u64,
}

#[derive(Debug, Args)]
pub struct AnimArgs {
    pub path: PathBuf,
    /// Override GIF delays with a safe target frame rate.
    #[arg(long, value_parser = clap::value_parser!(u32)
        .range(monkey_core::lcd::MIN_SAFE_FPS as i64..=monkey_core::lcd::MAX_SAFE_FPS as i64))]
    pub fps: Option<u32>,
    #[arg(long)]
    pub dither: bool,
    #[arg(long)]
    pub big_endian: bool,
    /// Repeat until Ctrl-C.
    #[arg(long)]
    pub r#loop: bool,
    #[arg(long)]
    pub mock: bool,
    #[arg(long)]
    pub allow_hardware_writes: bool,
    #[arg(long, default_value_t = 3)]
    pub inter_chunk_delay_ms: u64,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PatternName {
    Red,
    Green,
    Blue,
    White,
    Black,
    Grayscale,
    RgbBars,
    Geometry,
}

#[derive(Debug, Args)]
pub struct TestPatternArgs {
    pub pattern: PatternName,
    #[arg(long)]
    pub big_endian: bool,
    #[arg(long)]
    pub mock: bool,
    #[arg(long)]
    pub allow_hardware_writes: bool,
    #[arg(long, default_value_t = 3)]
    pub inter_chunk_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LcdReport {
    pub operation: String,
    pub target: String,
    pub frames: usize,
    pub chunks_per_frame: usize,
    pub bytes_sent: usize,
    pub dither: bool,
    pub little_endian: bool,
    pub elapsed_ms: f64,
}

pub fn run(args: LcdArgs, format: OutputFormat) -> anyhow::Result<()> {
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    run_with_writer(args, format, &mut writer)
}

pub fn run_with_writer<W: Write>(
    args: LcdArgs,
    format: OutputFormat,
    writer: &mut W,
) -> anyhow::Result<()> {
    match args.command {
        LcdCommand::Image(args) => run_image(args, format, writer),
        LcdCommand::TestPattern(args) => run_test_pattern(args, format, writer),
        LcdCommand::Anim(args) => run_anim(args, format, writer),
    }
}

fn acquire_device(mock: bool, allow_hardware_writes: bool) -> anyhow::Result<MonkaDevice> {
    if mock {
        Ok(MonkaDevice::from_transport(Box::new(MockTransport::new()))
            .with_hardware_writes_allowed(true))
    } else {
        require_write_consent(allow_hardware_writes)?;
        let device =
            MonkaDevice::open(InterfacePolicy::RequireA)?.with_hardware_writes_allowed(true);
        Ok(device)
    }
}

fn run_image<W: Write>(
    args: ImageArgs,
    format: OutputFormat,
    writer: &mut W,
) -> anyhow::Result<()> {
    let frame = load_image_frame(&args.path, args.dither, !args.big_endian)
        .with_context(|| format!("failed to load image {}", args.path.display()))?;
    let config = pacing(args.inter_chunk_delay_ms, DEFAULT_LCD_FPS)?;
    let target = if args.mock { "mock" } else { "hardware" };
    let mut device = acquire_device(args.mock, args.allow_hardware_writes)?;

    let bar = if format == OutputFormat::Human {
        let bar = ProgressBar::new(monkey_core::lcd::LCD_CHUNK_COUNT as u64);
        if let Ok(style) = ProgressStyle::with_template("  LCD chunks [{bar:32}] {pos}/{len}") {
            bar.set_style(style.progress_chars("=> "));
        }
        Some(bar)
    } else {
        None
    };

    let started = Instant::now();
    let metrics = device.stream_frame_with_progress(&frame, config, |done, _| {
        if let Some(bar) = &bar {
            bar.set_position(done as u64);
        }
    })?;
    if let Some(bar) = bar {
        bar.finish_and_clear();
    }

    let report = LcdReport {
        operation: "image".to_string(),
        target: target.to_string(),
        frames: 1,
        chunks_per_frame: metrics.chunks_sent,
        bytes_sent: metrics.bytes_sent,
        dither: args.dither,
        little_endian: !args.big_endian,
        elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
    };
    write_report(writer, format, report)
}

fn run_test_pattern<W: Write>(
    args: TestPatternArgs,
    format: OutputFormat,
    writer: &mut W,
) -> anyhow::Result<()> {
    let frame = generate_test_pattern_with_format(
        args.pattern.into_core(),
        if args.big_endian {
            ColorFormat::BigEndian
        } else {
            ColorFormat::LittleEndian
        },
    );
    let config = pacing(args.inter_chunk_delay_ms, DEFAULT_LCD_FPS)?;
    let target = if args.mock { "mock" } else { "hardware" };
    let mut device = acquire_device(args.mock, args.allow_hardware_writes)?;

    let bar = if format == OutputFormat::Human {
        let bar = ProgressBar::new(monkey_core::lcd::LCD_CHUNK_COUNT as u64);
        if let Ok(style) = ProgressStyle::with_template("  LCD chunks [{bar:32}] {pos}/{len}") {
            bar.set_style(style.progress_chars("=> "));
        }
        Some(bar)
    } else {
        None
    };

    let started = Instant::now();
    let metrics = device.stream_frame_with_progress(&frame, config, |done, _| {
        if let Some(bar) = &bar {
            bar.set_position(done as u64);
        }
    })?;
    if let Some(bar) = bar {
        bar.finish_and_clear();
    }

    let report = LcdReport {
        operation: "test-pattern".to_string(),
        target: target.to_string(),
        frames: 1,
        chunks_per_frame: metrics.chunks_sent,
        bytes_sent: metrics.bytes_sent,
        dither: false,
        little_endian: !args.big_endian,
        elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
    };
    write_report(writer, format, report)
}

fn run_anim<W: Write>(args: AnimArgs, format: OutputFormat, writer: &mut W) -> anyhow::Result<()> {
    let frames = decode_gif_frames(&args.path, args.dither, !args.big_endian)
        .with_context(|| format!("failed to decode GIF {}", args.path.display()))?;
    if frames.is_empty() {
        anyhow::bail!("GIF contains no frames");
    }
    let config = pacing(
        args.inter_chunk_delay_ms,
        args.fps.unwrap_or(DEFAULT_LCD_FPS),
    )?;
    let target = if args.mock { "mock" } else { "hardware" };
    let mut device = acquire_device(args.mock, args.allow_hardware_writes)?;

    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancelled);
    ctrlc::set_handler(move || flag.store(true, Ordering::SeqCst))
        .context("could not install Ctrl-C handler")?;

    let started = Instant::now();
    let mut sent_frames = 0usize;
    let mut sent_bytes = 0usize;
    let mut sent_chunks = 0usize;
    let mut regulator = FpsRegulator::new(args.fps.unwrap_or(DEFAULT_LCD_FPS))?;

    'playback: loop {
        for (index, frame) in frames.iter().enumerate() {
            if cancelled.load(Ordering::SeqCst) {
                break 'playback;
            }
            if args.fps.is_some() && sent_frames > 0 {
                regulator.wait_for_next_frame();
            }
            let metrics = device.stream_frame(&frame.frame, config)?;
            sent_frames += 1;
            sent_bytes += metrics.bytes_sent;
            sent_chunks += metrics.chunks_sent;
            if args.fps.is_none() && (args.r#loop || index + 1 < frames.len()) {
                let safe_delay = monkey_core::lcd::clamp_safe_frame_delay(frame.delay);
                if sleep_interruptible(safe_delay, &cancelled) {
                    break 'playback;
                }
            }
        }
        if !args.r#loop {
            break;
        }
    }

    let report = LcdReport {
        operation: "anim".to_string(),
        target: target.to_string(),
        frames: sent_frames,
        chunks_per_frame: sent_chunks
            .checked_div(sent_frames)
            .unwrap_or(monkey_core::lcd::LCD_CHUNK_COUNT),
        bytes_sent: sent_bytes,
        dither: args.dither,
        little_endian: !args.big_endian,
        elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
    };
    write_report(writer, format, report)
}

fn require_write_consent(allowed: bool) -> anyhow::Result<()> {
    if !allowed {
        anyhow::bail!(
            "LCD commands write to hardware. Re-run with --allow-hardware-writes or use --mock."
        );
    }
    Ok(())
}

fn pacing(delay_ms: u64, fps: u32) -> anyhow::Result<LcdPacingConfig> {
    let config = LcdPacingConfig {
        inter_chunk_delay: Duration::from_millis(delay_ms),
        target_fps: fps,
    };
    config.validate().map_err(|e| anyhow::anyhow!(e))?;
    Ok(config)
}

fn sleep_interruptible(duration: Duration, cancelled: &AtomicBool) -> bool {
    let started = Instant::now();
    while started.elapsed() < duration {
        if cancelled.load(Ordering::SeqCst) {
            return true;
        }
        let remaining = duration.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            break;
        }
        thread::sleep(remaining.min(Duration::from_millis(10)));
    }
    cancelled.load(Ordering::SeqCst)
}

fn write_report<W: Write>(
    writer: &mut W,
    format: OutputFormat,
    report: LcdReport,
) -> anyhow::Result<()> {
    let human = format!(
        "LCD {} complete: {} frame(s), {} chunks/frame, {} bytes ({})",
        report.operation, report.frames, report.chunks_per_frame, report.bytes_sent, report.target
    );
    format.write_to(writer, &human, &report)
}

impl PatternName {
    fn into_core(self) -> TestPatternType {
        match self {
            Self::Red => TestPatternType::Red,
            Self::Green => TestPatternType::Green,
            Self::Blue => TestPatternType::Blue,
            Self::White => TestPatternType::White,
            Self::Black => TestPatternType::Black,
            Self::Grayscale => TestPatternType::Grayscale,
            Self::RgbBars => TestPatternType::RgbBars,
            Self::Geometry => TestPatternType::Geometry,
        }
    }
}
