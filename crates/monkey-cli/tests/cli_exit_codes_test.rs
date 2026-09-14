use anyhow::anyhow;
use monkey_cli::error::{
    classify_error, ExitCode, EXIT_BLOCKED, EXIT_GENERAL, EXIT_NO_DEVICE, EXIT_PERMISSION,
    EXIT_USAGE,
};

#[test]
fn test_classify_no_device_error() {
    let err = anyhow!("No Monka 3075 Pro / RKGK890 keyboard detected");
    let code = classify_error(&err);
    assert_eq!(code, ExitCode::NoDevice);
    assert_eq!(code.as_i32(), EXIT_NO_DEVICE);
}

#[test]
fn test_classify_blocked_safety_error() {
    let err =
        anyhow!("Hardware writes require explicit consent. Re-run with `--allow-hardware-writes`");
    let code = classify_error(&err);
    assert_eq!(code, ExitCode::Blocked);
    assert_eq!(code.as_i32(), EXIT_BLOCKED);

    let err2 = anyhow!("Safety violation: Flash write throttled. Wait 500ms between commits.");
    assert_eq!(classify_error(&err2), ExitCode::Blocked);

    let err3 = anyhow!("Battery level is below 20% on wireless connection. Commit blocked.");
    assert_eq!(classify_error(&err3), ExitCode::Blocked);
}

#[test]
fn test_classify_permission_error() {
    let err = anyhow!(
        "Failed to open HID device: Permission denied (IOKit error kIOReturnExclusiveAccess)"
    );
    let code = classify_error(&err);
    assert_eq!(code, ExitCode::Permission);
    assert_eq!(code.as_i32(), EXIT_PERMISSION);
}

#[test]
fn test_classify_usage_error() {
    let err = anyhow!("Invalid lighting mode 'turbo'. Available modes: static, breathing, wave...");
    let code = classify_error(&err);
    assert_eq!(code, ExitCode::Usage);
    assert_eq!(code.as_i32(), EXIT_USAGE);

    let err2 = anyhow!("Invalid RGB hex color '#xyz123'. Expected format #RRGGBB");
    assert_eq!(classify_error(&err2), ExitCode::Usage);
}

#[test]
fn test_classify_general_error() {
    let err = anyhow!("Unknown internal error occurred during processing");
    let code = classify_error(&err);
    assert_eq!(code, ExitCode::General);
    assert_eq!(code.as_i32(), EXIT_GENERAL);
}
