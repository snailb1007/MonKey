use anyhow::Result;
use clap::Args;
use monkey_core::doctor::{run_doctor_checks, CheckStatus, DoctorReport};

use crate::output::OutputFormat;

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Run diagnostics against an in-memory mock environment
    #[arg(long)]
    pub mock: bool,
}

pub fn run(args: DoctorArgs, format: OutputFormat) -> Result<()> {
    let report = run_doctor_checks(args.mock);

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Human => {
            print_human_report(&report);
        }
    }

    Ok(())
}

fn print_human_report(report: &DoctorReport) {
    println!("============================================================");
    println!("              MonKey System & Hardware Doctor               ");
    println!("============================================================");
    println!("Platform: {}", report.platform);
    println!();
    println!("Checks:");

    for check in &report.checks {
        let tag = match check.status {
            CheckStatus::Pass => "[✓ PASS]",
            CheckStatus::Warn => "[! WARN]",
            CheckStatus::Fail => "[✗ FAIL]",
        };

        println!("  {:8} {}", tag, check.name);
        println!("           {}", check.message);
    }

    println!();
    println!("Summary:");
    println!(
        "  Total: {} | Passed: {} | Warned: {} | Failed: {}",
        report.summary.total,
        report.summary.passed,
        report.summary.warned,
        report.summary.failed
    );
    println!("============================================================");

    if report.summary.all_healthy {
        println!("Status: HEALTHY - Device and environment are ready for use.");
    } else {
        println!("Status: ATTENTION REQUIRED - Some checks failed or require action.");
    }

    let remediations: Vec<_> = report
        .checks
        .iter()
        .filter_map(|c| c.remediation.as_ref().map(|r| (&c.name, r)))
        .collect();

    if !remediations.is_empty() {
        println!("============================================================");
        println!("Actionable Remediation Guidance:");
        for (name, rem) in remediations {
            println!("  • {}: {}", name, rem);
        }
    }
    println!("============================================================");
}
