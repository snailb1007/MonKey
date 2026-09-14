use clap::{Parser, Subcommand};

use crate::commands;

/// Monka 3075 Pro USB HID Tooling
#[derive(Parser, Debug)]
#[command(
    name = "monkey",
    author,
    version,
    about = "Monka 3075 Pro USB HID Tooling",
    long_about = None
)]
pub struct Cli {
    /// Format output as structured JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Increase logging verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Inspect keyboard hardware identity and composite HID interfaces
    Info,
    /// Probe device capabilities and transport state (read-only)
    Probe,
    /// Measure bulk streaming throughput and command roundtrip latency
    Bench(commands::bench::BenchArgs),
    /// Render and stream LCD images, animations, and diagnostics
    Lcd(commands::lcd::LcdArgs),
    /// Ambient RGB controls, two-tier state persistence, and profile management
    Rgb(commands::rgb::RgbArgs),
    /// Diagnose system platform, USB permissions, and hardware health
    Doctor(commands::doctor::DoctorArgs),
    /// Generate shell completion scripts (bash, zsh, fish, elvish, powershell)
    Completions(commands::completions::CompletionsArgs),
}
