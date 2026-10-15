use std::sync::Arc;

use crate::adapter::DesktopAdapter;
use crate::dto::*;
use crate::worker::MockHardwareBackend;
use monkey_core::transport::MockTransport;

fn make_adapter() -> (DesktopAdapter, Arc<std::sync::Mutex<MockTransport>>) {
    let mock = MockTransport::new();
    let (backend, transport_ref) = MockHardwareBackend::with_mock_transport(mock);
    let adapter = DesktopAdapter::new_with_backend(backend);
    (adapter, transport_ref)
}

#[test]
fn test_adapter_list_devices_envelope_roundtrip() {
    let (adapter, transport_ref) = make_adapter();

    let req = DesktopRequestEnvelope {
        version: 1,
        request_id: "req-1".to_string(),
        device_id: None,
        action: DesktopAction::ListDevices,
        payload: serde_json::json!({}),
    };

    let resp = adapter.handle_request(req);
    assert_eq!(resp.version, 1);
    assert_eq!(resp.request_id, "req-1");
    assert!(resp.ok);
    assert!(resp.error.is_none());

    let res: ListDevicesResult =
        serde_json::from_value(resp.data.expect("data")).expect("ListDevicesResult");
    assert_eq!(res.devices.len(), 1);
    assert_eq!(res.devices[0].label, "Monka 3075 Pro / RKGK890");

    let transport = transport_ref.lock().unwrap();
    transport
        .assert_no_writes()
        .expect("Zero writes on list_devices");
}

#[test]
fn test_adapter_probe_envelope() {
    let (adapter, _) = make_adapter();

    // First list devices to get ID
    let list_req = DesktopRequestEnvelope {
        version: 1,
        request_id: "req-list".to_string(),
        device_id: None,
        action: DesktopAction::ListDevices,
        payload: serde_json::json!({}),
    };
    let list_resp = adapter.handle_request(list_req);
    let res: ListDevicesResult = serde_json::from_value(list_resp.data.unwrap()).unwrap();
    let dev_id = res.devices[0].id.clone();

    // Probe
    let probe_req = DesktopRequestEnvelope {
        version: 1,
        request_id: "req-probe".to_string(),
        device_id: Some(dev_id.clone()),
        action: DesktopAction::Probe,
        payload: serde_json::json!({}),
    };
    let probe_resp = adapter.handle_request(probe_req);
    assert!(probe_resp.ok);
    let probed: ProbeResult = serde_json::from_value(probe_resp.data.unwrap()).unwrap();
    assert_eq!(probed.device.id, dev_id);
    assert_eq!(probed.device.firmware.as_deref(), Some("v1.0.0"));
}

#[test]
fn test_adapter_diagnostics_envelope() {
    let (adapter, _) = make_adapter();

    let req = DesktopRequestEnvelope {
        version: 1,
        request_id: "req-diag".to_string(),
        device_id: None,
        action: DesktopAction::Diagnostics,
        payload: serde_json::json!({}),
    };
    let resp = adapter.handle_request(req);
    assert!(resp.ok);
    let diag: DiagnosticsResult = serde_json::from_value(resp.data.unwrap()).unwrap();
    assert!(!diag.checks.is_empty());
}

#[test]
fn test_adapter_start_lcd_pattern_and_job_status() {
    let (adapter, transport_ref) = make_adapter();

    // List devices
    let list_req = DesktopRequestEnvelope {
        version: 1,
        request_id: "req-l".to_string(),
        device_id: None,
        action: DesktopAction::ListDevices,
        payload: serde_json::json!({}),
    };
    let list_resp = adapter.handle_request(list_req);
    let res: ListDevicesResult = serde_json::from_value(list_resp.data.unwrap()).unwrap();
    let dev_id = res.devices[0].id.clone();

    // Start LCD pattern: color_bars
    let start_req = DesktopRequestEnvelope {
        version: 1,
        request_id: "req-start".to_string(),
        device_id: Some(dev_id.clone()),
        action: DesktopAction::StartLcd,
        payload: serde_json::json!({
            "assetId": null,
            "pattern": "color_bars"
        }),
    };
    let start_resp = adapter.handle_request(start_req);
    assert!(start_resp.ok);
    let start_res: StartLcdResult = serde_json::from_value(start_resp.data.unwrap()).unwrap();
    let job_id = start_res.job_id;

    // Query job status
    let status_req = DesktopRequestEnvelope {
        version: 1,
        request_id: "req-status".to_string(),
        device_id: Some(dev_id),
        action: DesktopAction::JobStatus,
        payload: serde_json::json!({
            "jobId": job_id
        }),
    };
    let status_resp = adapter.handle_request(status_req);
    assert!(status_resp.ok);
    let status_res: JobStatusResult = serde_json::from_value(status_resp.data.unwrap()).unwrap();
    assert_eq!(status_res.job.id, job_id);
    assert_eq!(status_res.job.frames_sent, 1);
    assert_eq!(status_res.job.state, JobState::Completed);

    let transport = transport_ref.lock().unwrap();
    let bulk_writes: Vec<_> = transport
        .calls()
        .iter()
        .filter(|c| matches!(c, monkey_core::transport::TransportCall::WriteBulk { .. }))
        .collect();
    assert_eq!(bulk_writes.len(), 8);
}

#[test]
fn test_adapter_rgb_lifecycle_preview_and_commit() {
    let (adapter, transport_ref) = make_adapter();

    let list_req = DesktopRequestEnvelope {
        version: 1,
        request_id: "req-1".to_string(),
        device_id: None,
        action: DesktopAction::ListDevices,
        payload: serde_json::json!({}),
    };
    let list_resp = adapter.handle_request(list_req);
    let list_res: ListDevicesResult = serde_json::from_value(list_resp.data.unwrap()).unwrap();
    let dev_id = list_res.devices[0].id.clone();

    // 1. Initial RGB status -> verified or unknown
    let status_req = DesktopRequestEnvelope {
        version: 1,
        request_id: "req-rgb-st".to_string(),
        device_id: Some(dev_id.clone()),
        action: DesktopAction::RgbStatus,
        payload: serde_json::json!({}),
    };
    let status_resp = adapter.handle_request(status_req);
    assert!(status_resp.ok);
    let status_data: RgbStatusResult = serde_json::from_value(status_resp.data.unwrap()).unwrap();
    assert_eq!(status_data.persistence, PersistenceState::Unknown);

    // 2. Commit before preview -> UnsafeOperation error
    let commit_req = DesktopRequestEnvelope {
        version: 1,
        request_id: "req-commit-fail".to_string(),
        device_id: Some(dev_id.clone()),
        action: DesktopAction::RgbCommit,
        payload: serde_json::json!({}),
    };
    let commit_resp = adapter.handle_request(commit_req);
    assert!(!commit_resp.ok);
    assert_eq!(commit_resp.error.unwrap().code, ErrorCode::UnsafeOperation);

    // 3. Preview valid config
    let preview_req = DesktopRequestEnvelope {
        version: 1,
        request_id: "req-preview".to_string(),
        device_id: Some(dev_id.clone()),
        action: DesktopAction::RgbPreview,
        payload: serde_json::json!({
            "config": {
                "mode": "breathing",
                "color": { "r": 255, "g": 128, "b": 0 },
                "brightness": 80,
                "speed": 50,
                "direction": "left_to_right"
            }
        }),
    };
    let preview_resp = adapter.handle_request(preview_req);
    assert!(preview_resp.ok);
    let preview_data: RgbPreviewResult =
        serde_json::from_value(preview_resp.data.unwrap()).unwrap();
    assert_eq!(preview_data.persistence, PersistenceState::Ram);

    // 4. Commit -> succeeds
    let commit_req2 = DesktopRequestEnvelope {
        version: 1,
        request_id: "req-commit-ok".to_string(),
        device_id: Some(dev_id.clone()),
        action: DesktopAction::RgbCommit,
        payload: serde_json::json!({}),
    };
    let commit_resp2 = adapter.handle_request(commit_req2);
    assert!(commit_resp2.ok);
    let commit_data: RgbCommitResult = serde_json::from_value(commit_resp2.data.unwrap()).unwrap();
    assert_eq!(commit_data.persistence, PersistenceState::Verified);

    let transport = transport_ref.lock().unwrap();
    let feat_calls: Vec<_> = transport
        .calls()
        .iter()
        .filter(|c| matches!(c, monkey_core::transport::TransportCall::SendFeature { .. }))
        .collect();
    assert_eq!(feat_calls.len(), 2, "Preview + Commit sent feature reports");
}

#[test]
fn test_adapter_profile_export_and_import() {
    let (adapter, _) = make_adapter();

    let export_req = DesktopRequestEnvelope {
        version: 1,
        request_id: "req-exp".to_string(),
        device_id: None,
        action: DesktopAction::ExportProfile,
        payload: serde_json::json!({
            "config": {
                "mode": "wave",
                "color": { "r": 0, "g": 200, "b": 255 },
                "brightness": 100,
                "speed": 75,
                "direction": "right_to_left"
            },
            "description": "Cyan wave profile"
        }),
    };
    let export_resp = adapter.handle_request(export_req);
    assert!(export_resp.ok);
    let export_data: ExportProfileResult =
        serde_json::from_value(export_resp.data.unwrap()).unwrap();
    assert!(export_data.profile_json.contains("\"wave\""));

    // Import the exported JSON
    let import_req = DesktopRequestEnvelope {
        version: 1,
        request_id: "req-imp".to_string(),
        device_id: None,
        action: DesktopAction::ImportProfile,
        payload: serde_json::json!({
            "profileJson": export_data.profile_json
        }),
    };
    let import_resp = adapter.handle_request(import_req);
    assert!(import_resp.ok);
    let import_data: ImportProfileResult =
        serde_json::from_value(import_resp.data.unwrap()).unwrap();
    assert_eq!(import_data.config.mode, "wave");
    assert_eq!(import_data.config.color.b, 255);
}

#[test]
fn test_adapter_handles_malformed_json_gracefully() {
    let (adapter, _) = make_adapter();

    // Unknown field in envelope
    let malformed_str = r#"{
        "version": 1,
        "requestId": "bad-1",
        "action": "list_devices",
        "payload": {},
        "unexpectedField": "boom"
    }"#;

    let res: Result<DesktopRequestEnvelope, _> = serde_json::from_str(malformed_str);
    assert!(res.is_err(), "Must reject unknown fields");

    // Invalid version
    let bad_ver_req = DesktopRequestEnvelope {
        version: 99,
        request_id: "bad-ver".to_string(),
        device_id: None,
        action: DesktopAction::ListDevices,
        payload: serde_json::json!({}),
    };
    let bad_ver_resp = adapter.handle_request(bad_ver_req);
    assert!(!bad_ver_resp.ok);
    assert_eq!(bad_ver_resp.error.unwrap().code, ErrorCode::InvalidInput);
}
