use crate::dto::*;
use crate::error::DesktopError;
use crate::profile::{export_profile_json, import_profile_json};
use crate::worker::{HardwareWorkerHandle, MockHardwareBackend};
use monkey_core::device::{
    group_monka_devices, DiscoveredDevice, InterfacePolicy, InterfaceRole, MonkaDeviceSet,
    OpenError, MONKA_PID, MONKA_VID,
};
use monkey_core::protocol::types::CommandId;
use monkey_core::rgb::encode_rgb_control_packet;
use monkey_core::transport::MockTransport;
use std::ffi::CString;

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

// --- 08-01 M: Windows RGB preview routing assertions ---

fn windows_instance_path(mi: &str, instance: &str) -> String {
    format!("\\\\?\\hid#vid_05ac&pid_024f&{mi}#{instance}#{{4d1e55b2-f16f-11cf-88cb-001111000030}}")
}

fn windows_test_device(
    usage_page: u16,
    usage: u16,
    interface_number: i32,
    path: &str,
) -> DiscoveredDevice {
    DiscoveredDevice {
        vid: MONKA_VID,
        pid: MONKA_PID,
        manufacturer: None,
        product: None,
        serial_number: None,
        path: CString::new(path).expect("Valid CString"),
        usage_page,
        usage,
        interface_number,
    }
}

#[test]
fn test_rgb_preview_payload_matches_codec_and_normalizes_to_hid_frame() {
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
    worker
        .rgb_preview(dev_id, config.clone())
        .expect("rgb_preview should succeed");

    let transport = transport_ref.lock().unwrap();
    let sent: Vec<Vec<u8>> = transport
        .calls()
        .iter()
        .filter_map(|c| match c {
            monkey_core::transport::TransportCall::SendFeature { data } => Some(data.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(sent.len(), 1, "preview sends exactly one feature report");
    let data = &sent[0];

    // Direct inspection against the codec: exact bytes plus raw64 command
    // bytes (04 13), color, markers, and no bulk/flash records.
    let core_config = config
        .to_core_config()
        .expect("dto converts to core config");
    let expected = encode_rgb_control_packet(&core_config).expect("codec encodes config");
    assert_eq!(
        data.as_slice(),
        expected.as_bytes(),
        "preview bytes must equal the encoded RGB control packet"
    );
    assert_eq!(
        data.len(),
        64,
        "preview frame must be the raw 64-byte shape"
    );
    assert_eq!(data[0], 0x04, "magic04");
    assert_eq!(
        data[1],
        CommandId::RgbControl as u8,
        "RGB control command byte"
    );
    assert_eq!(&data[3..6], &[255, 0, 128], "color bytes");
    assert_eq!(&data[14..16], &[0xAA, 0x55], "frame markers");
    assert!(
        !transport
            .calls()
            .iter()
            .any(|c| matches!(c, monkey_core::transport::TransportCall::WriteBulk { .. })),
        "preview must not emit bulk writes or flash saves"
    );

    worker.shutdown();
}

#[test]
fn test_constants_only_init_requires_interface_b() {
    // A constants-only set carries no interfaces: RequireB must fail with
    // InterfaceUnavailable(InterfaceB), proving preview routing needs B.
    let set = MonkaDeviceSet::new(MONKA_VID, MONKA_PID);
    let err = InterfacePolicy::RequireB
        .resolve(&set)
        .expect_err("RequireB on an empty set must fail");
    assert_eq!(
        err,
        OpenError::InterfaceUnavailable(InterfaceRole::InterfaceB)
    );
}

#[test]
fn test_production_group_pairs_windows_interfaces() {
    let a_path = windows_instance_path("mi_00", "8&2d0c3b4c&0&0000");
    let b_path = windows_instance_path("mi_01", "8&2d0c3b4c&0&0000");
    let a = windows_test_device(0xFF68, 0x0061, 0, &a_path);
    let b = windows_test_device(0xFFFF, 0x0001, 1, &b_path);

    let sets = group_monka_devices(&[a, b]);
    assert_eq!(sets.len(), 1);
    let set = &sets[0];
    assert!(set.interface_a.is_some());
    assert!(set.interface_b.is_some());
}
