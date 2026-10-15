use crate::dto::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcV1Fixtures {
    pub version: u32,
    pub title: String,
    pub description: String,
    pub supported_modes: Vec<String>,
    pub supported_patterns: Vec<String>,
    pub actions: Vec<ActionFixture>,
    pub errors: Vec<ErrorFixture>,
    pub malformed_cases: Vec<MalformedCaseFixture>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionFixture {
    pub action: DesktopAction,
    pub request: DesktopRequestEnvelope,
    pub success_response: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorFixture {
    pub code: ErrorCode,
    pub response: DesktopErrorResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MalformedCaseFixture {
    pub name: String,
    pub raw_json: String,
    pub expected_error_code: ErrorCode,
}

pub fn generate_default_fixtures() -> IpcV1Fixtures {
    let mock_device = DeviceSnapshot {
        id: "dev-mock-0".to_string(),
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
        battery_percent: Some(95),
        firmware: Some("v1.0.0".to_string()),
    };

    let sample_config = LightingConfigDto {
        mode: "wave".to_string(),
        color: ColorDto {
            r: 255,
            g: 255,
            b: 255,
        },
        brightness: 100,
        speed: 50,
        direction: Some("left_to_right".to_string()),
    };

    let sample_asset = AssetPreview {
        id: "asset-mock-1".to_string(),
        name: "test_pattern.png".to_string(),
        width: 128,
        height: 128,
        frame_count: 1,
        preview_data_url: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==".to_string(),
    };

    let sample_job = JobSnapshot {
        id: "job-mock-1".to_string(),
        device_id: "dev-mock-0".to_string(),
        state: JobState::Running,
        frames_sent: 42,
        error: None,
    };

    let actions = vec![
        ActionFixture {
            action: DesktopAction::ListDevices,
            request: DesktopRequestEnvelope {
                version: 1,
                request_id: "req-list-devices".to_string(),
                device_id: None,
                action: DesktopAction::ListDevices,
                payload: serde_json::json!({}),
            },
            success_response: serde_json::to_value(DesktopSuccessResponse::new(
                "req-list-devices".to_string(),
                ListDevicesResult {
                    devices: vec![mock_device.clone()],
                },
            ))
            .unwrap(),
        },
        ActionFixture {
            action: DesktopAction::Probe,
            request: DesktopRequestEnvelope {
                version: 1,
                request_id: "req-probe".to_string(),
                device_id: Some("dev-mock-0".to_string()),
                action: DesktopAction::Probe,
                payload: serde_json::json!({}),
            },
            success_response: serde_json::to_value(DesktopSuccessResponse::new(
                "req-probe".to_string(),
                ProbeResult {
                    device: mock_device.clone(),
                },
            ))
            .unwrap(),
        },
        ActionFixture {
            action: DesktopAction::Diagnostics,
            request: DesktopRequestEnvelope {
                version: 1,
                request_id: "req-diagnostics".to_string(),
                device_id: None,
                action: DesktopAction::Diagnostics,
                payload: serde_json::json!({}),
            },
            success_response: serde_json::to_value(DesktopSuccessResponse::new(
                "req-diagnostics".to_string(),
                DiagnosticsResult {
                    checks: vec![
                        DiagnosticCheckDto {
                            id: "platform".to_string(),
                            label: "Host Operating System".to_string(),
                            status: DiagnosticStatusDto::Pass,
                            detail: "Supported macOS environment".to_string(),
                            remediation: None,
                        },
                        DiagnosticCheckDto {
                            id: "hid_subsystem".to_string(),
                            label: "USB HID Subsystem".to_string(),
                            status: DiagnosticStatusDto::Pass,
                            detail: "HID subsystem available".to_string(),
                            remediation: None,
                        },
                    ],
                },
            ))
            .unwrap(),
        },
        ActionFixture {
            action: DesktopAction::PickAsset,
            request: DesktopRequestEnvelope {
                version: 1,
                request_id: "req-pick-asset".to_string(),
                device_id: None,
                action: DesktopAction::PickAsset,
                payload: serde_json::to_value(PickAssetPayload {
                    kind: AssetKind::Image,
                })
                .unwrap(),
            },
            success_response: serde_json::to_value(DesktopSuccessResponse::new(
                "req-pick-asset".to_string(),
                PickAssetResult {
                    asset: Some(sample_asset.clone()),
                },
            ))
            .unwrap(),
        },
        ActionFixture {
            action: DesktopAction::StartLcd,
            request: DesktopRequestEnvelope {
                version: 1,
                request_id: "req-start-lcd".to_string(),
                device_id: Some("dev-mock-0".to_string()),
                action: DesktopAction::StartLcd,
                payload: serde_json::to_value(StartLcdPayload {
                    asset_id: None,
                    pattern: Some(DiagnosticPattern::ColorBars),
                })
                .unwrap(),
            },
            success_response: serde_json::to_value(DesktopSuccessResponse::new(
                "req-start-lcd".to_string(),
                StartLcdResult {
                    job_id: "job-mock-1".to_string(),
                },
            ))
            .unwrap(),
        },
        ActionFixture {
            action: DesktopAction::StopLcd,
            request: DesktopRequestEnvelope {
                version: 1,
                request_id: "req-stop-lcd".to_string(),
                device_id: Some("dev-mock-0".to_string()),
                action: DesktopAction::StopLcd,
                payload: serde_json::to_value(StopLcdPayload {
                    job_id: "job-mock-1".to_string(),
                })
                .unwrap(),
            },
            success_response: serde_json::to_value(DesktopSuccessResponse::new(
                "req-stop-lcd".to_string(),
                StopLcdResult { stopped: true },
            ))
            .unwrap(),
        },
        ActionFixture {
            action: DesktopAction::JobStatus,
            request: DesktopRequestEnvelope {
                version: 1,
                request_id: "req-job-status".to_string(),
                device_id: Some("dev-mock-0".to_string()),
                action: DesktopAction::JobStatus,
                payload: serde_json::to_value(JobStatusPayload {
                    job_id: "job-mock-1".to_string(),
                })
                .unwrap(),
            },
            success_response: serde_json::to_value(DesktopSuccessResponse::new(
                "req-job-status".to_string(),
                JobStatusResult { job: sample_job },
            ))
            .unwrap(),
        },
        ActionFixture {
            action: DesktopAction::RgbStatus,
            request: DesktopRequestEnvelope {
                version: 1,
                request_id: "req-rgb-status".to_string(),
                device_id: Some("dev-mock-0".to_string()),
                action: DesktopAction::RgbStatus,
                payload: serde_json::json!({}),
            },
            success_response: serde_json::to_value(DesktopSuccessResponse::new(
                "req-rgb-status".to_string(),
                RgbStatusResult {
                    config: Some(sample_config.clone()),
                    persistence: PersistenceState::Verified,
                    supported_modes: LightingConfigDto::SUPPORTED_MODES
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                },
            ))
            .unwrap(),
        },
        ActionFixture {
            action: DesktopAction::RgbPreview,
            request: DesktopRequestEnvelope {
                version: 1,
                request_id: "req-rgb-preview".to_string(),
                device_id: Some("dev-mock-0".to_string()),
                action: DesktopAction::RgbPreview,
                payload: serde_json::to_value(RgbPreviewPayload {
                    config: sample_config.clone(),
                })
                .unwrap(),
            },
            success_response: serde_json::to_value(DesktopSuccessResponse::new(
                "req-rgb-preview".to_string(),
                RgbPreviewResult {
                    config: sample_config.clone(),
                    persistence: PersistenceState::Ram,
                },
            ))
            .unwrap(),
        },
        ActionFixture {
            action: DesktopAction::RgbCommit,
            request: DesktopRequestEnvelope {
                version: 1,
                request_id: "req-rgb-commit".to_string(),
                device_id: Some("dev-mock-0".to_string()),
                action: DesktopAction::RgbCommit,
                payload: serde_json::json!({}),
            },
            success_response: serde_json::to_value(DesktopSuccessResponse::new(
                "req-rgb-commit".to_string(),
                RgbCommitResult {
                    persistence: PersistenceState::Verified,
                },
            ))
            .unwrap(),
        },
        ActionFixture {
            action: DesktopAction::ExportProfile,
            request: DesktopRequestEnvelope {
                version: 1,
                request_id: "req-export-profile".to_string(),
                device_id: Some("dev-mock-0".to_string()),
                action: DesktopAction::ExportProfile,
                payload: serde_json::to_value(ExportProfilePayload {
                    config: sample_config.clone(),
                    description: Some("Custom ripple profile".to_string()),
                })
                .unwrap(),
            },
            success_response: serde_json::to_value(DesktopSuccessResponse::new(
                "req-export-profile".to_string(),
                ExportProfileResult {
                    profile_json: "{\"schema_version\":1,\"model\":\"Monka 3075 Pro / RKGK890\",\"lighting\":{\"mode\":\"ripple\"}}".to_string(),
                },
            ))
            .unwrap(),
        },
        ActionFixture {
            action: DesktopAction::ImportProfile,
            request: DesktopRequestEnvelope {
                version: 1,
                request_id: "req-import-profile".to_string(),
                device_id: Some("dev-mock-0".to_string()),
                action: DesktopAction::ImportProfile,
                payload: serde_json::to_value(ImportProfilePayload {
                    profile_json: "{\"schema_version\":1,\"model\":\"Monka 3075 Pro / RKGK890\",\"lighting\":{\"mode\":\"ripple\"}}".to_string(),
                })
                .unwrap(),
            },
            success_response: serde_json::to_value(DesktopSuccessResponse::new(
                "req-import-profile".to_string(),
                ImportProfileResult {
                    config: sample_config,
                },
            ))
            .unwrap(),
        },
    ];

    let error_codes = vec![
        (ErrorCode::NotConnected, "No keyboard connected", false),
        (
            ErrorCode::PermissionDenied,
            "Access to USB HID interface was denied by the OS",
            false,
        ),
        (
            ErrorCode::Unsupported,
            "Key remapping and macro programming are not supported",
            false,
        ),
        (
            ErrorCode::Busy,
            "Hardware worker is currently streaming an LCD frame",
            true,
        ),
        (
            ErrorCode::UnsafeOperation,
            "Speculative flash writes or unverified opcodes are rejected",
            false,
        ),
        (
            ErrorCode::LowBattery,
            "Persistence is blocked below 20% battery on wireless",
            false,
        ),
        (
            ErrorCode::InvalidInput,
            "Malformed request envelope or parameter out of range",
            false,
        ),
        (
            ErrorCode::IoError,
            "USB HID communication failure during transfer",
            true,
        ),
        (
            ErrorCode::Cancelled,
            "Active LCD streaming job was cancelled by user",
            false,
        ),
        (
            ErrorCode::Internal,
            "An unexpected internal error occurred",
            false,
        ),
    ];

    let errors = error_codes
        .into_iter()
        .map(|(code, msg, retryable)| ErrorFixture {
            code,
            response: DesktopErrorResponse::new(
                format!("req-error-{:?}", code).to_lowercase(),
                DesktopErrorPayload {
                    code,
                    message: msg.to_string(),
                    retryable,
                },
            ),
        })
        .collect();

    let malformed_cases = vec![
        MalformedCaseFixture {
            name: "invalid_version".to_string(),
            raw_json: r#"{"version": 2, "requestId": "req-1", "action": "list_devices", "payload": {}}"#.to_string(),
            expected_error_code: ErrorCode::InvalidInput,
        },
        MalformedCaseFixture {
            name: "empty_request_id".to_string(),
            raw_json: r#"{"version": 1, "requestId": "   ", "action": "list_devices", "payload": {}}"#.to_string(),
            expected_error_code: ErrorCode::InvalidInput,
        },
        MalformedCaseFixture {
            name: "unknown_action".to_string(),
            raw_json: r#"{"version": 1, "requestId": "req-1", "action": "erase_flash", "payload": {}}"#.to_string(),
            expected_error_code: ErrorCode::InvalidInput,
        },
        MalformedCaseFixture {
            name: "unknown_field".to_string(),
            raw_json: r#"{"version": 1, "requestId": "req-1", "action": "list_devices", "payload": {}, "extraField": true}"#.to_string(),
            expected_error_code: ErrorCode::InvalidInput,
        },
    ];

    IpcV1Fixtures {
        version: 1,
        title: "MonKey Desktop IPC Contract v1".to_string(),
        description: "Locked cross-language contract fixtures for Tauri desktop IPC".to_string(),
        supported_modes: LightingConfigDto::SUPPORTED_MODES
            .iter()
            .map(|s| s.to_string())
            .collect(),
        supported_patterns: vec![
            "red".to_string(),
            "green".to_string(),
            "blue".to_string(),
            "white".to_string(),
            "black".to_string(),
            "grayscale".to_string(),
            "color_bars".to_string(),
            "geometry".to_string(),
        ],
        actions,
        errors,
        malformed_cases,
    }
}
