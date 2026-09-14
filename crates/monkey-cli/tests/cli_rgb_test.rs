use monkey_cli::commands::rgb::{
    run_rgb, RgbArgs, RgbCommand, SaveArgs, SetArgs, StatusArgs, RestoreArgs,
};
use monkey_cli::output::OutputFormat;

#[test]
fn test_cli_rgb_set_ram_preview_mock() {
    let args = RgbArgs {
        command: RgbCommand::Set(SetArgs {
            mode: "static".into(),
            color: Some("#FF0000".into()),
            brightness: 100,
            speed: 50,
            direction: "left-to-right".into(),
            commit: false,
            force: false,
            mock: true,
            allow_hardware_writes: false,
        }),
    };

    let res = run_rgb(args, OutputFormat::Human);
    assert!(res.is_ok(), "Setting static red on mock should succeed");
}

#[test]
fn test_cli_rgb_set_flash_commit_mock() {
    let args = RgbArgs {
        command: RgbCommand::Set(SetArgs {
            mode: "wave".into(),
            color: None,
            brightness: 80,
            speed: 70,
            direction: "left-to-right".into(),
            commit: true,
            force: false,
            mock: true,
            allow_hardware_writes: false,
        }),
    };

    let res = run_rgb(args, OutputFormat::Human);
    assert!(res.is_ok(), "Committing wave on mock should succeed");
}

#[test]
fn test_cli_rgb_set_json_output() {
    let args = RgbArgs {
        command: RgbCommand::Set(SetArgs {
            mode: "breathing".into(),
            color: Some("blue".into()),
            brightness: 90,
            speed: 40,
            direction: "right-to-left".into(),
            commit: false,
            force: false,
            mock: true,
            allow_hardware_writes: false,
        }),
    };

    let res = run_rgb(args, OutputFormat::Json);
    assert!(res.is_ok(), "JSON format on mock should succeed");
}

#[test]
fn test_cli_rgb_status_mock() {
    let args = RgbArgs {
        command: RgbCommand::Status(StatusArgs { mock: true }),
    };

    assert!(run_rgb(args, OutputFormat::Human).is_ok());
}

#[test]
fn test_cli_rgb_save_and_restore_mock() {
    let tmp_file = std::env::temp_dir().join("monkey_cli_test_profile.json");

    // Save
    let save_args = RgbArgs {
        command: RgbCommand::Save(SaveArgs {
            file: Some(tmp_file.clone()),
            stdout: false,
            mock: true,
        }),
    };
    assert!(run_rgb(save_args, OutputFormat::Human).is_ok());
    assert!(tmp_file.exists());

    // Restore
    let restore_args = RgbArgs {
        command: RgbCommand::Restore(RestoreArgs {
            file: Some(tmp_file.clone()),
            commit: false,
            force: false,
            mock: true,
            allow_hardware_writes: false,
        }),
    };
    assert!(run_rgb(restore_args, OutputFormat::Human).is_ok());

    let _ = std::fs::remove_file(tmp_file);
}

#[test]
fn test_cli_rgb_invalid_mode() {
    let args = RgbArgs {
        command: RgbCommand::Set(SetArgs {
            mode: "nonexistent_mode".into(),
            color: None,
            brightness: 100,
            speed: 50,
            direction: "left-to-right".into(),
            commit: false,
            force: false,
            mock: true,
            allow_hardware_writes: false,
        }),
    };

    let res = run_rgb(args, OutputFormat::Human);
    assert!(res.is_err());
    let msg = res.unwrap_err().to_string();
    assert!(msg.contains("Invalid lighting mode"));
}

#[test]
fn test_cli_rgb_invalid_color() {
    let args = RgbArgs {
        command: RgbCommand::Set(SetArgs {
            mode: "static".into(),
            color: Some("#invalid_hex".into()),
            brightness: 100,
            speed: 50,
            direction: "left-to-right".into(),
            commit: false,
            force: false,
            mock: true,
            allow_hardware_writes: false,
        }),
    };

    let res = run_rgb(args, OutputFormat::Human);
    assert!(res.is_err());
    let msg = res.unwrap_err().to_string();
    assert!(msg.contains("Invalid color hex"));
}

#[test]
fn test_cli_rgb_hardware_write_consent_gate() {
    // Non-mock without allow_hardware_writes must fail with consent message
    let args = RgbArgs {
        command: RgbCommand::Set(SetArgs {
            mode: "static".into(),
            color: Some("red".into()),
            brightness: 100,
            speed: 50,
            direction: "left-to-right".into(),
            commit: false,
            force: false,
            mock: false,
            allow_hardware_writes: false,
        }),
    };

    let res = run_rgb(args, OutputFormat::Human);
    assert!(res.is_err());
    let msg = res.unwrap_err().to_string();
    assert!(
        msg.contains("Hardware writes require explicit consent"),
        "Expected consent gate message, got: {}",
        msg
    );
}
