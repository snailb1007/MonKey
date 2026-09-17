//! `monkey rgb` lighting controls, profiles, and state management.

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

use crate::output::OutputFormat;
use monkey_core::device::{InterfacePolicy, MonkaDevice};
use monkey_core::rgb::{FlowDirection, LightingConfig, LightingMode, RgbColor, RgbProfile};
use monkey_core::transport::MockTransport;

#[derive(Debug, Args)]
pub struct RgbArgs {
    #[command(subcommand)]
    pub command: RgbCommand,
}

#[derive(Debug, Subcommand)]
pub enum RgbCommand {
    /// Configure ambient lighting mode, color, brightness, and speed
    Set(SetArgs),
    /// Display current active lighting status
    Status(StatusArgs),
    /// Export active lighting configuration to a JSON profile
    Save(SaveArgs),
    /// Import and apply an RGB configuration from a JSON profile
    Restore(RestoreArgs),
}

#[derive(Debug, Args)]
pub struct SetArgs {
    /// Lighting mode (static, breathing, wave, rainbow, ripple, reactive, off)
    pub mode: String,

    /// Color as hex (#RRGGBB or RRGGBB) or named alias (red, green, blue, etc.)
    #[arg(short, long)]
    pub color: Option<String>,

    /// Brightness level (0..=100)
    #[arg(short, long, default_value_t = 100)]
    pub brightness: u8,

    /// Speed level (0..=100)
    #[arg(short, long, default_value_t = 50)]
    pub speed: u8,

    /// Flow direction (left-to-right, right-to-left)
    #[arg(short, long, default_value = "left-to-right")]
    pub direction: String,

    /// Commit permanently to onboard SPI NOR flash (default: volatile RAM preview)
    #[arg(long)]
    pub commit: bool,

    /// Override low-battery wireless safety gate
    #[arg(long)]
    pub force: bool,

    /// Run against in-memory mock transport instead of physical USB device
    #[arg(long)]
    pub mock: bool,

    /// Explicit consent flag required to perform hardware writes on physical device
    #[arg(long)]
    pub allow_hardware_writes: bool,
}

#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Run against in-memory mock transport
    #[arg(long)]
    pub mock: bool,
}

#[derive(Debug, Args)]
pub struct SaveArgs {
    /// Destination path for JSON profile (defaults to ./rgb_profile.json)
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Output profile JSON directly to stdout
    #[arg(long)]
    pub stdout: bool,

    /// Run against in-memory mock transport
    #[arg(long)]
    pub mock: bool,
}

#[derive(Debug, Args)]
pub struct RestoreArgs {
    /// Path to JSON profile file (defaults to ./rgb_profile.json)
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Commit permanently to onboard SPI NOR flash (default: volatile RAM preview)
    #[arg(long)]
    pub commit: bool,

    /// Override low-battery wireless safety gate
    #[arg(long)]
    pub force: bool,

    /// Run against in-memory mock transport
    #[arg(long)]
    pub mock: bool,

    /// Explicit consent flag required to perform hardware writes on physical device
    #[arg(long)]
    pub allow_hardware_writes: bool,
}

#[derive(Debug, Serialize)]
pub struct RgbStatusOutput {
    pub mode: String,
    pub color: String,
    pub brightness: u8,
    pub speed: u8,
    pub direction: String,
    pub write_mode: String,
}

pub fn run_rgb(args: RgbArgs, format: OutputFormat) -> Result<()> {
    match args.command {
        RgbCommand::Set(set_args) => run_set(set_args, format),
        RgbCommand::Status(status_args) => run_status(status_args, format),
        RgbCommand::Save(save_args) => run_save(save_args, format),
        RgbCommand::Restore(restore_args) => run_restore(restore_args, format),
    }
}

fn resolve_device(
    mock: bool,
    allow_hardware_writes: bool,
    requires_write: bool,
) -> Result<MonkaDevice> {
    if mock {
        return Ok(MonkaDevice::from_transport(Box::new(MockTransport::new()))
            .with_hardware_writes_allowed(true));
    }

    if requires_write && !allow_hardware_writes {
        bail!(
            "Hardware writes require explicit consent. Re-run with `--allow-hardware-writes` to modify keyboard RGB settings, or use `--mock` for headless testing."
        );
    }

    let device = MonkaDevice::open(InterfacePolicy::PreferB)?
        .with_hardware_writes_allowed(allow_hardware_writes || !requires_write);
    Ok(device)
}

fn run_set(args: SetArgs, format: OutputFormat) -> Result<()> {
    let mode: LightingMode = args.mode.parse()?;
    let color = if let Some(c) = args.color {
        RgbColor::parse(&c)?
    } else {
        match mode {
            LightingMode::Off => RgbColor::BLACK,
            LightingMode::Rainbow | LightingMode::Wave => RgbColor::WHITE,
            _ => RgbColor::RED,
        }
    };

    let direction = match args.direction.trim().to_lowercase().as_str() {
        "right-to-left" | "rtl" | "right" => FlowDirection::RightToLeft,
        _ => FlowDirection::LeftToRight,
    };

    let config = LightingConfig {
        mode,
        color,
        brightness: args.brightness,
        speed: args.speed,
        direction,
    };
    config.validate()?;

    let mut device = resolve_device(args.mock, args.allow_hardware_writes, true)?;

    let write_mode_str = if args.commit {
        device
            .apply_rgb_commit(&config, device.is_wireless(), None, args.force)
            .context("Failed to commit RGB configuration to flash")?;
        "FlashCommit (Permanent)"
    } else {
        device
            .apply_rgb_preview(&config)
            .context("Failed to apply RGB preview to RAM")?;
        "RamPreview (Volatile)"
    };

    let output = RgbStatusOutput {
        mode: format!("{:?}", config.mode),
        color: config.color.to_hex(),
        brightness: config.brightness,
        speed: config.speed,
        direction: format!("{:?}", config.direction),
        write_mode: write_mode_str.to_string(),
    };

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Human => {
            println!("============================================================");
            println!("                MonKey RGB Configuration Applied             ");
            println!("============================================================");
            println!("  Mode:         {:?}", config.mode);
            println!(
                "  Color:        {} ({:?})",
                config.color.to_hex(),
                config.color
            );
            println!("  Brightness:   {}/100", config.brightness);
            println!("  Speed:        {}/100", config.speed);
            println!("  Direction:    {:?}", config.direction);
            println!("  Target:       {}", write_mode_str);
            println!("============================================================");
        }
    }

    Ok(())
}

fn run_status(args: StatusArgs, format: OutputFormat) -> Result<()> {
    let mut device = resolve_device(args.mock, true, false)?;

    let active_config = device.readback_rgb_status()?.unwrap_or_default();

    let output = RgbStatusOutput {
        mode: format!("{:?}", active_config.mode),
        color: active_config.color.to_hex(),
        brightness: active_config.brightness,
        speed: active_config.speed,
        direction: format!("{:?}", active_config.direction),
        write_mode: "Active".to_string(),
    };

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Human => {
            println!("============================================================");
            println!("                   MonKey Active RGB Status                  ");
            println!("============================================================");
            println!("  Mode:         {:?}", active_config.mode);
            println!("  Color:        {}", active_config.color.to_hex());
            println!("  Brightness:   {}/100", active_config.brightness);
            println!("  Speed:        {}/100", active_config.speed);
            println!("  Direction:    {:?}", active_config.direction);
            println!("============================================================");
        }
    }

    Ok(())
}

fn run_save(args: SaveArgs, format: OutputFormat) -> Result<()> {
    let mut device = resolve_device(args.mock, true, false)?;

    let active_config = device.readback_rgb_status()?.unwrap_or_default();
    let profile = RgbProfile::new(
        "Monka 3075 Pro",
        active_config,
        Some("Saved profile".into()),
    );

    if args.stdout {
        println!("{}", profile.to_json()?);
        return Ok(());
    }

    let dest = args
        .file
        .unwrap_or_else(|| PathBuf::from("./rgb_profile.json"));
    profile.save_to_file(&dest)?;

    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "saved": true,
                "file": dest.display().to_string(),
                "profile": profile
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Human => {
            println!("RGB profile successfully saved to: {}", dest.display());
        }
    }

    Ok(())
}

fn run_restore(args: RestoreArgs, format: OutputFormat) -> Result<()> {
    let source = args
        .file
        .unwrap_or_else(|| PathBuf::from("./rgb_profile.json"));

    let profile = RgbProfile::load_from_file(&source)
        .with_context(|| format!("Failed to load RGB profile from {:?}", source))?;

    let mut device = resolve_device(args.mock, args.allow_hardware_writes, true)?;

    let write_mode_str = if args.commit {
        device
            .apply_rgb_commit(&profile.lighting, device.is_wireless(), None, args.force)
            .context("Failed to commit restored RGB profile to flash")?;
        "FlashCommit (Permanent)"
    } else {
        device
            .apply_rgb_preview(&profile.lighting)
            .context("Failed to apply restored RGB profile to RAM")?;
        "RamPreview (Volatile)"
    };

    let output = RgbStatusOutput {
        mode: format!("{:?}", profile.lighting.mode),
        color: profile.lighting.color.to_hex(),
        brightness: profile.lighting.brightness,
        speed: profile.lighting.speed,
        direction: format!("{:?}", profile.lighting.direction),
        write_mode: write_mode_str.to_string(),
    };

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Human => {
            println!("============================================================");
            println!("              MonKey RGB Profile Restored                   ");
            println!("============================================================");
            println!("  File:         {}", source.display());
            println!("  Model:        {}", profile.model);
            println!("  Mode:         {:?}", profile.lighting.mode);
            println!("  Color:        {}", profile.lighting.color.to_hex());
            println!("  Brightness:   {}/100", profile.lighting.brightness);
            println!("  Speed:        {}/100", profile.lighting.speed);
            println!("  Target:       {}", write_mode_str);
            println!("============================================================");
        }
    }

    Ok(())
}
