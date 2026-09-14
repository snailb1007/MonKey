use monkey_cli::commands::completions::{generate_completions, ShellChoice};
use monkey_cli::Cli;

#[test]
fn test_completions_bash() {
    let mut buf = Vec::new();
    generate_completions::<Cli, _>(ShellChoice::Bash, &mut buf);
    let output = String::from_utf8(buf).expect("Invalid UTF-8 in bash completions");

    assert!(output.contains("monkey"));
    assert!(output.contains("info"));
    assert!(output.contains("probe"));
    assert!(output.contains("bench"));
    assert!(output.contains("lcd"));
    assert!(output.contains("rgb"));
    assert!(output.contains("doctor"));
    assert!(output.contains("completions"));
}

#[test]
fn test_completions_zsh() {
    let mut buf = Vec::new();
    generate_completions::<Cli, _>(ShellChoice::Zsh, &mut buf);
    let output = String::from_utf8(buf).expect("Invalid UTF-8 in zsh completions");

    assert!(output.contains("#compdef monkey"));
    assert!(output.contains("info"));
    assert!(output.contains("probe"));
    assert!(output.contains("bench"));
    assert!(output.contains("lcd"));
    assert!(output.contains("rgb"));
    assert!(output.contains("doctor"));
    assert!(output.contains("completions"));
}

#[test]
fn test_completions_fish() {
    let mut buf = Vec::new();
    generate_completions::<Cli, _>(ShellChoice::Fish, &mut buf);
    let output = String::from_utf8(buf).expect("Invalid UTF-8 in fish completions");

    assert!(output.contains("complete -c monkey"));
    assert!(output.contains("info"));
    assert!(output.contains("probe"));
    assert!(output.contains("bench"));
    assert!(output.contains("lcd"));
    assert!(output.contains("rgb"));
    assert!(output.contains("doctor"));
    assert!(output.contains("completions"));
}

#[test]
fn test_completions_elvish_and_powershell() {
    let mut elvish_buf = Vec::new();
    generate_completions::<Cli, _>(ShellChoice::Elvish, &mut elvish_buf);
    let elvish_str = String::from_utf8(elvish_buf).expect("Invalid UTF-8 in elvish completions");
    assert!(elvish_str.contains("monkey"));

    let mut ps_buf = Vec::new();
    generate_completions::<Cli, _>(ShellChoice::Powershell, &mut ps_buf);
    let ps_str = String::from_utf8(ps_buf).expect("Invalid UTF-8 in powershell completions");
    assert!(ps_str.contains("monkey"));
}
