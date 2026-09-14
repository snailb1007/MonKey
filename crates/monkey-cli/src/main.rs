use clap::Parser;
use monkey_cli::commands;
use monkey_cli::output::OutputFormat;
use monkey_cli::{Cli, Commands};

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
        Commands::Lcd(args) => commands::lcd::run(args, format)?,
        Commands::Rgb(args) => commands::rgb::run_rgb(args, format)?,
        Commands::Doctor(args) => commands::doctor::run(args, format)?,
        Commands::Completions(args) => commands::completions::run::<Cli>(args)?,
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        let exit_code = monkey_cli::classify_error(&err);
        eprintln!("Error [{}]: {:#}", exit_code.category_name(), err);
        std::process::exit(exit_code.as_i32());
    }
}
