use clap::{Parser, Subcommand};
use monkey_cli::commands;
use monkey_cli::output::OutputFormat;

/// Monka 3075 Pro USB HID Tooling
#[derive(Parser, Debug)]
#[command(name = "monkey", author, version, about = "Monka 3075 Pro USB HID Tooling", long_about = None)]
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
}

fn init_tracing(verbose: u8) {
    let filter = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter_layer = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter_layer)
        .with_target(false)
        .try_init();
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    let format = if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::Human
    };

    match cli.command {
        Commands::Info => commands::info::run_info(format)?,
        Commands::Probe => commands::probe::run_probe(format)?,
        Commands::Bench(args) => commands::bench::run(args, format)?,
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
