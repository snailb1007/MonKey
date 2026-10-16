use crate::dto::*;
use crate::error::DesktopError;
use crate::profile::{export_profile_json, import_profile_json};
use crate::worker::{HardwareWorkerHandle, MockHardwareBackend};
use monkey_core::transport::MockTransport;

#[test]
fn test_rgb_preview_sends_ram_feature_report_and_sets_ram_persistence() {
    let mock = MockTransport::new();
    let (backend, transport_ref) = MockHardwareBackend::with_mock_transport(mock);
    let worker = HardwareWorkerHandle::spawn_mock(backend);

    let devices = worker.list_devices().unwrap();
    let dev_id = &devices[0].id;

    let config = LightingConfigDto {
        mode: "breathing".to_string(),
        color: ColorDto {
            r: 255,
            g: 0,
            b: 128,
        },
        brightness: 75,
        speed: 60,
        direction: Some("left_to_right".to_string()),
    };

    let preview_res = worker
        .rgb_preview(dev_id, config.clone())
        .expect("rgb_preview should succeed");

    assert_eq!(preview_res.persistence, PersistenceState::Ram);
    assert_eq!(preview_res.config.mode, "breathing");

    // Check status reflects RAM persistence
    let status = worker
        .rgb_status(dev_id)
        .expect("rgb_status should succeed");
    assert_eq!(status.persistence, PersistenceState::Ram);
    assert_eq!(status.config.as_ref().unwrap().mode, "breathing");

    // Verify feature report was sent
    let transport = transport_ref.lock().unwrap();
    let calls = transport.calls();
    let feature_reports: Vec<_> = calls
        .iter()
        .filter(|c| matches!(c, monkey_core::transport::TransportCall::SendFeature { .. }))
        .collect();
    assert_eq!(
        feature_reports.len(),
        1,
        "Exactly one feature report sent for preview"
    );

    worker.shutdown();
}

#[test]
fn test_rgb_commit_without_prior_preview_fails_with_unsafe_operation() {
    let mock = MockTransport::new();
    let (backend, transport_ref) = MockHardwareBackend::with_mock_transport(mock);
    let worker = HardwareWorkerHandle::spawn_mock(backend);

    let devices = worker.list_devices().unwrap();
    let dev_id = &devices[0].id;

    // Direct commit without preview
    let res = worker.rgb_commit(dev_id);
    match res {
        Err(DesktopError::UnsafeOperation(msg)) => {
            assert!(msg.contains("No pending RGB preview"));
        }
        other => panic!("Expected UnsafeOperation when committing without preview, got: {other:?}"),
    }

    // Prove ZERO writes occurred
    let transport = transport_ref.lock().unwrap();
    transport
        .assert_no_writes()
        .expect("Zero writes must occur when commit is rejected");

    worker.shutdown();
}

#[test]
fn test_rgb_commit_low_wireless_battery_blocks_persistence_with_zero_commit_writes() {
    let mock = MockTransport::new();
    let (mut backend, transport_ref) = MockHardwareBackend::with_mock_transport(mock);
    backend.is_wireless = true;
    backend.battery_percent = Some(15); // 15% < 20%
    let worker = HardwareWorkerHandle::spawn_mock(backend);

    let devices = worker.list_devices().unwrap();
    let dev_id = &devices[0].id;

    let config = LightingConfigDto {
        mode: "static".to_string(),
        color: ColorDto { r: 0, g: 255, b: 0 },
        brightness: 50,
        speed: 50,
        direction: Some("left_to_right".to_string()),
    };

    // Apply preview first (which succeeds in RAM)
    worker.rgb_preview(dev_id, config).unwrap();

    // Clear recorded calls to verify commit step
    {
        let mut transport = transport_ref.lock().unwrap();
        transport.clear_calls();
    }

    // Commit must be rejected due to low battery (<20%)
    let res = worker.rgb_commit(dev_id);
    match res {
        Err(DesktopError::LowBattery(pct)) => {
            assert_eq!(pct, 15);
        }
        other => panic!("Expected LowBattery error, got: {other:?}"),
    }

    // Verify commit write was BLOCKED
    let transport = transport_ref.lock().unwrap();
    transport
        .assert_no_writes()
        .expect("Zero writes must occur when low battery blocks commit");

    worker.shutdown();
}

#[test]
fn test_rgb_commit_unknown_wireless_battery_blocks_persistence() {
    let mock = MockTransport::new();
    let (mut backend, transport_ref) = MockHardwareBackend::with_mock_transport(mock);
    backend.is_wireless = true;
    backend.battery_percent = None; // Unknown battery on wireless
    let worker = HardwareWorkerHandle::spawn_mock(backend);

    let devices = worker.list_devices().unwrap();
    let dev_id = &devices[0].id;

    let config = LightingConfigDto {
        mode: "wave".to_string(),
        color: ColorDto {
            r: 100,
            g: 100,
            b: 100,
        },
        brightness: 50,
        speed: 50,
        direction: Some("left_to_right".to_string()),
    };

    worker.rgb_preview(dev_id, config).unwrap();

    {
        let mut transport = transport_ref.lock().unwrap();
        transport.clear_calls();
    }

    // Commit must be blocked because unknown wireless battery could deplete during write
    let res = worker.rgb_commit(dev_id);
    match res {
        Err(DesktopError::LowBattery(_)) => {}
        other => panic!("Expected LowBattery error for unknown wireless battery, got: {other:?}"),
    }

    let transport = transport_ref.lock().unwrap();
    transport
        .assert_no_writes()
        .expect("Zero writes must occur when unknown wireless battery blocks commit");

    worker.shutdown();
}

#[test]
fn test_rgb_commit_valid_flow_succeeds_and_reaches_verified() {
    let mock = MockTransport::new();
    let (backend, _) = MockHardwareBackend::with_mock_transport(mock);
    let worker = HardwareWorkerHandle::spawn_mock(backend);

    let devices = worker.list_devices().unwrap();
    let dev_id = &devices[0].id;

    let config = LightingConfigDto {
        mode: "rainbow".to_string(),
        color: ColorDto {
            r: 255,
            g: 255,
            b: 255,
        },
        brightness: 100,
        speed: 80,
        direction: Some("left_to_right".to_string()),
    };

    // 1. Preview in RAM
    let preview = worker.rgb_preview(dev_id, config).unwrap();
    assert_eq!(preview.persistence, PersistenceState::Ram);

    // 2. Commit to persistent storage
    let commit = worker.rgb_commit(dev_id).expect("rgb_commit must succeed");
    assert_eq!(commit.persistence, PersistenceState::Verified);

    // 3. Status is now verified
    let status = worker.rgb_status(dev_id).unwrap();
    assert_eq!(status.persistence, PersistenceState::Verified);

    // 4. Repeated commit without new preview fails safely
    let second_commit = worker.rgb_commit(dev_id);
    assert!(
        second_commit.is_err(),
        "Second commit without new preview must fail"
    );

    worker.shutdown();
}

#[test]
fn test_profile_export_and_import_roundtrip_with_validation() {
    let config = LightingConfigDto {
        mode: "ripple".to_string(),
        color: ColorDto {
            r: 42,
            g: 99,
            b: 210,
        },
        brightness: 90,
        speed: 45,
        direction: Some("right_to_left".to_string()),
    };

    let exported_json = export_profile_json(
        "Monka 3075 Pro / RKGK890",
        &config,
        Some("Custom ripple profile"),
    )
    .expect("Profile export should succeed");

    assert!(exported_json.contains("\"schema_version\": 1"));
    assert!(exported_json.contains("\"ripple\""));

    // Import profile
    let imported_config =
        import_profile_json(&exported_json).expect("Profile import should succeed");

    assert_eq!(imported_config.mode, "ripple");
    assert_eq!(imported_config.color.r, 42);
    assert_eq!(imported_config.color.g, 99);
    assert_eq!(imported_config.color.b, 210);
    assert_eq!(imported_config.brightness, 90);
    assert_eq!(imported_config.speed, 45);
    assert_eq!(imported_config.direction.as_deref(), Some("right_to_left"));
}

#[test]
fn test_profile_import_rejects_unsupported_schema_version() {
    let future_json = r#"{
        "schema_version": 99,
        "model": "Monka 3075 Pro",
        "created_at": "12345678",
        "lighting": {
            "mode": "static",
            "color": {"r": 255, "g": 255, "b": 255},
            "brightness": 100,
            "speed": 50,
            "direction": "left_to_right"
        }
    }"#;

    let res = import_profile_json(future_json);
    match res {
        Err(DesktopError::InvalidInput(msg)) => {
            assert!(msg.contains("Unsupported profile schema version"));
        }
        other => panic!("Expected InvalidInput for future schema version, got: {other:?}"),
    }
}
