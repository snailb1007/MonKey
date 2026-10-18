use crate::dto::*;
use crate::error::DesktopError;
use crate::fixtures::{generate_default_fixtures, IpcV1Fixtures};

#[test]
fn test_request_envelope_valid() {
    let json = r#"{
        "version": 1,
        "requestId": "req-123",
        "deviceId": "dev-0",
        "action": "list_devices",
        "payload": {}
    }"#;
    let req: DesktopRequestEnvelope =
        serde_json::from_str(json).expect("Valid request should deserialize");
    assert_eq!(req.version, 1);
    assert_eq!(req.request_id, "req-123");
    assert_eq!(req.device_id.as_deref(), Some("dev-0"));
    assert_eq!(req.action, DesktopAction::ListDevices);
    assert!(req.validate_envelope().is_ok());
}

#[test]
fn test_request_envelope_rejects_unknown_fields() {
    let json = r#"{
        "version": 1,
        "requestId": "req-123",
        "action": "list_devices",
        "payload": {},
        "maliciousField": "evil"
    }"#;
    let res: Result<DesktopRequestEnvelope, _> = serde_json::from_str(json);
    assert!(
        res.is_err(),
        "Deny unknown fields must reject extraneous fields"
    );
}

#[test]
fn test_request_envelope_version_mismatch() {
    let req = DesktopRequestEnvelope {
        version: 2,
        request_id: "req-1".to_string(),
        device_id: None,
        action: DesktopAction::ListDevices,
        payload: serde_json::json!({}),
    };
    match req.validate_envelope() {
        Err(DesktopError::InvalidInput(msg)) => {
            assert!(msg.contains("Unsupported request version"));
        }
        other => panic!("Expected InvalidInput error, got: {other:?}"),
    }
}

#[test]
fn test_request_envelope_empty_request_id() {
    let req = DesktopRequestEnvelope {
        version: 1,
        request_id: "   ".to_string(),
        device_id: None,
        action: DesktopAction::ListDevices,
        payload: serde_json::json!({}),
    };
    match req.validate_envelope() {
        Err(DesktopError::InvalidInput(msg)) => {
            assert!(msg.contains("Request ID cannot be empty"));
        }
        other => panic!("Expected InvalidInput error, got: {other:?}"),
    }
}

#[test]
fn test_request_envelope_too_long_request_id() {
    let req = DesktopRequestEnvelope {
        version: 1,
        request_id: "a".repeat(129),
        device_id: None,
        action: DesktopAction::ListDevices,
        payload: serde_json::json!({}),
    };
    match req.validate_envelope() {
        Err(DesktopError::InvalidInput(msg)) => {
            assert!(msg.contains("maximum length of 128"));
        }
        other => panic!("Expected InvalidInput error, got: {other:?}"),
    }
}

#[test]
fn test_all_action_discriminants_deserialize() {
    let actions = [
        ("list_devices", DesktopAction::ListDevices),
        ("probe", DesktopAction::Probe),
        ("diagnostics", DesktopAction::Diagnostics),
        ("pick_asset", DesktopAction::PickAsset),
        ("start_lcd", DesktopAction::StartLcd),
        ("stop_lcd", DesktopAction::StopLcd),
        ("job_status", DesktopAction::JobStatus),
        ("rgb_status", DesktopAction::RgbStatus),
        ("rgb_preview", DesktopAction::RgbPreview),
        ("rgb_commit", DesktopAction::RgbCommit),
        ("export_profile", DesktopAction::ExportProfile),
        ("import_profile", DesktopAction::ImportProfile),
    ];

    for (action_str, expected_action) in actions {
        let json = format!(
            r#"{{"version": 1, "requestId": "req-1", "action": "{action_str}", "payload": {{}}}}"#
        );
        let req: DesktopRequestEnvelope = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("Failed to deserialize action '{action_str}': {e}"));
        assert_eq!(req.action, expected_action);
    }
}

#[test]
fn test_unsupported_actions_rejected() {
    let forbidden_actions = [
        "remap_key",
        "set_macro",
        "update_firmware",
        "flash_erase",
        "reset_device",
        "audio_visualize",
        "mouse_dpi",
    ];

    for action_str in forbidden_actions {
        let json = format!(
            r#"{{"version": 1, "requestId": "req-1", "action": "{action_str}", "payload": {{}}}}"#
        );
        let res: Result<DesktopRequestEnvelope, _> = serde_json::from_str(&json);
        assert!(
            res.is_err(),
            "Action '{action_str}' should be rejected as unknown action"
        );
    }
}

#[test]
fn test_all_error_codes_roundtrip() {
    let codes = [
        ErrorCode::NotConnected,
        ErrorCode::PermissionDenied,
        ErrorCode::Unsupported,
        ErrorCode::Busy,
        ErrorCode::UnsafeOperation,
        ErrorCode::LowBattery,
        ErrorCode::InvalidInput,
        ErrorCode::IoError,
        ErrorCode::Cancelled,
        ErrorCode::Internal,
    ];

    for code in codes {
        let payload = DesktopErrorPayload {
            code,
            message: format!("Test message for {code:?}"),
            retryable: matches!(code, ErrorCode::Busy | ErrorCode::IoError),
        };
        let resp = DesktopErrorResponse::new("req-err".to_string(), payload);
        let serialized = serde_json::to_string(&resp).unwrap();
        let deserialized: DesktopErrorResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.error.code, code);
        assert_eq!(
            deserialized.error.retryable,
            matches!(code, ErrorCode::Busy | ErrorCode::IoError)
        );
        assert!(!deserialized.ok);
    }
}

#[test]
fn test_lighting_config_validation() {
    let valid = LightingConfigDto {
        mode: "wave".to_string(),
        color: ColorDto {
            r: 10,
            g: 20,
            b: 30,
        },
        brightness: 80,
        speed: 50,
        direction: Some("left_to_right".to_string()),
    };
    assert!(valid.validate_for_preview().is_ok());

    // Null direction in preview must be rejected for animated modes
    let null_dir = LightingConfigDto {
        direction: None,
        ..valid.clone()
    };
    assert!(null_dir.validate_for_preview().is_err());

    // Null direction in preview is allowed for non-directional modes (off, static)
    for non_dir_mode in ["off", "static"] {
        let non_dir = LightingConfigDto {
            mode: non_dir_mode.to_string(),
            direction: None,
            ..valid.clone()
        };
        assert!(
            non_dir.validate_for_preview().is_ok(),
            "Mode '{non_dir_mode}' must allow null direction in preview"
        );
    }

    // Brightness > 100 rejected
    let bad_brightness = LightingConfigDto {
        brightness: 101,
        ..valid.clone()
    };
    assert!(bad_brightness.validate_for_preview().is_err());

    // Speed > 100 rejected
    let bad_speed = LightingConfigDto {
        speed: 105,
        ..valid.clone()
    };
    assert!(bad_speed.validate_for_preview().is_err());

    // Invalid mode rejected
    let bad_mode = LightingConfigDto {
        mode: "disco_fever".to_string(),
        ..valid.clone()
    };
    assert!(bad_mode.validate_for_preview().is_err());
}

#[test]
fn test_start_lcd_payload_validation() {
    // Valid with pattern only
    let pattern_only = StartLcdPayload {
        asset_id: None,
        pattern: Some(DiagnosticPattern::ColorBars),
    };
    assert!(pattern_only.validate().is_ok());

    // Valid with asset_id only
    let asset_only = StartLcdPayload {
        asset_id: Some("asset-1".to_string()),
        pattern: None,
    };
    assert!(asset_only.validate().is_ok());

    // Invalid with both
    let both = StartLcdPayload {
        asset_id: Some("asset-1".to_string()),
        pattern: Some(DiagnosticPattern::ColorBars),
    };
    assert!(both.validate().is_err());

    // Invalid with neither
    let neither = StartLcdPayload {
        asset_id: None,
        pattern: None,
    };
    assert!(neither.validate().is_err());
}

#[test]
fn test_device_snapshot_no_pii() {
    let snapshot = DeviceSnapshot {
        id: "dev-0".to_string(),
        label: "Monka 3075 Pro / RKGK890".to_string(),
        connection: ConnectionType::Wired,
        interfaces: InterfacesDto {
            lcd: true,
            config: true,
        },
        capabilities: CapabilitiesDto {
            lcd: true,
            rgb: true,
            persist: true,
            remap: false,
            macro_: false,
            firmware: false,
        },
        battery_percent: Some(100),
        firmware: Some("v1.0.0".to_string()),
    };

    let json = serde_json::to_string(&snapshot).unwrap();
    // Verify no serial number or OS HID file path leaked
    assert!(!json.contains("serial"));
    assert!(!json.contains("path"));
    assert!(!json.contains("IOUSB"));
    assert!(!json.contains("DevSrvsID"));
}

#[test]
fn test_fixtures_file_equality() {
    let fixtures = generate_default_fixtures();
    let fixtures_json = serde_json::to_string_pretty(&fixtures).unwrap();

    // Verify fixtures serialize cleanly and can deserialize back
    let parsed: IpcV1Fixtures = serde_json::from_str(&fixtures_json)
        .expect("Generated fixtures JSON must deserialize cleanly");
    assert_eq!(parsed.version, 1);
    assert_eq!(parsed.actions.len(), 12);
    assert_eq!(parsed.errors.len(), 10);
    assert_eq!(parsed.malformed_cases.len(), 4);

    // Verify or emit to contracts/ipc-v1.json
    let contract_path = std::path::Path::new("../contracts/ipc-v1.json");
    if let Some(parent) = contract_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(contract_path, &fixtures_json).unwrap();

    let disk_content = std::fs::read_to_string(contract_path)
        .expect("contracts/ipc-v1.json must exist and be readable");
    let disk_fixtures: IpcV1Fixtures = serde_json::from_str(&disk_content)
        .expect("contracts/ipc-v1.json must parse as valid IpcV1Fixtures");
    assert_eq!(
        disk_fixtures, fixtures,
        "Disk fixtures must equal Rust-generated fixtures"
    );
}
