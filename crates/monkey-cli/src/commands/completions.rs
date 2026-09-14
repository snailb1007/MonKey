use std::io::Write;

use anyhow::Result;
use clap::{Args, CommandFactory, ValueEnum};
use clap_complete::Shell;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ShellChoice {
    Bash,
    Elvish,
    Fish,
    Powershell,
    Zsh,
}

impl From<ShellChoice> for Shell {
    fn from(choice: ShellChoice) -> Self {
        match choice {
            ShellChoice::Bash => Shell::Bash,
            ShellChoice::Elvish => Shell::Elvish,
            ShellChoice::Fish => Shell::Fish,
            ShellChoice::Powershell => Shell::PowerShell,
            ShellChoice::Zsh => Shell::Zsh,
        }
    }
}

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    /// Target shell to generate completions for (bash, elvish, fish, powershell, zsh)
    #[arg(value_enum)]
    pub shell: ShellChoice,
}

/// Generates completion scripts for the specified shell writing into any `std::io::Write` destination.
pub fn generate_completions<C: CommandFactory, W: Write>(shell: ShellChoice, writer: &mut W) {
    let mut cmd = C::command();
    let generator: Shell = shell.into();
    clap_complete::generate(generator, &mut cmd, "monkey", writer);
}

/// Executes the CLI completions command printing to stdout.
pub fn run<C: CommandFactory>(args: CompletionsArgs) -> Result<()> {
    generate_completions::<C, _>(args.shell, &mut std::io::stdout());
    Ok(())
}
