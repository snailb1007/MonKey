use monkey_core::protocol::safety::SafetyRails;
use monkey_core::protocol::types::{CommandId, FEATURE_REPORT_MAGIC, FEATURE_REPORT_MARKER};
use monkey_core::rgb::{
    LightingConfig, LightingMode, RgbColor, RgbManager, RgbProfile, CURRENT_PROFILE_SCHEMA_VERSION,
};
use monkey_core::transport::{MockTransport, TransportCall};
use std::time::Duration;

#[test]
fn test_rgb_manager_ram_preview() {
    let mut transport = MockTransport::new();
    let safety = SafetyRails::new().with_hardware_writes_permitted(true);

    let mut manager = RgbManager::new(&mut transport, &safety);

    let config = LightingConfig::default_static(RgbColor::RED);
    manager.apply_preview(&config).expect("preview succeeds");

    assert_eq!(manager.session_flash_commit_count(), 0);

    let calls = transport.calls();
    assert_eq!(calls.len(), 1);
    match &calls[0] {
        TransportCall::SendFeature { data } => {
            assert_eq!(data[0], FEATURE_REPORT_MAGIC);
            assert_eq!(data[1], CommandId::RgbControl as u8);
            assert_eq!(data[2], LightingMode::Static.to_byte());
            assert_eq!(data[3], 255); // R
            assert_eq!(data[4], 0);   // G
            assert_eq!(data[5], 0);   // B
            assert_eq!(data[14], FEATURE_REPORT_MARKER[0]);
            assert_eq!(data[15], FEATURE_REPORT_MARKER[1]);
        }
        other => panic!("Unexpected transport call: {:?}", other),
    }
}

#[test]
fn test_rgb_manager_flash_commit_transaction_sequence() {
    let mut transport = MockTransport::new();
    let safety = SafetyRails::new().with_hardware_writes_permitted(true);

    let mut manager = RgbManager::new(&mut transport, &safety);

    let config = LightingConfig::default_static(RgbColor::BLUE);
    manager
        .apply_commit(&config, false, None, false)
        .expect("commit succeeds");

    assert_eq!(manager.session_flash_commit_count(), 1);

    let calls = transport.calls();
    // Expected transaction sequence:
    // 1. StartTransaction (0x18)
    // 2. RgbControl (0x13)
    // 3. SaveSettings (0x02)
    // 4. EndTransaction (0xF0)
    assert_eq!(calls.len(), 4);

    let expected_commands = [
        CommandId::StartTransaction as u8,
        CommandId::RgbControl as u8,
        CommandId::SaveSettings as u8,
        CommandId::EndTransaction as u8,
    ];

    for (i, call) in calls.iter().enumerate() {
        match call {
            TransportCall::SendFeature { data } => {
                assert_eq!(data[0], FEATURE_REPORT_MAGIC);
                assert_eq!(data[1], expected_commands[i]);
            }
            other => panic!("Unexpected call at {}: {:?}", i, other),
        }
    }
}

#[test]
fn test_rgb_manager_flash_debouncing() {
    let mut transport = MockTransport::new();
    // 500ms debounce
    let safety = SafetyRails::with_flash_debounce(Duration::from_millis(500))
        .with_hardware_writes_permitted(true);

    let mut manager = RgbManager::new(&mut transport, &safety);
    let config = LightingConfig::default_static(RgbColor::GREEN);

    // First commit succeeds
    manager
        .apply_commit(&config, false, None, false)
        .expect("first commit succeeds");
    assert_eq!(manager.session_flash_commit_count(), 1);

    // Immediate second commit within 500ms fails with throttling error
    let second_err = manager.apply_commit(&config, false, None, false);
    assert!(second_err.is_err());
    let err_str = second_err.unwrap_err().to_string();
    assert!(
        err_str.contains("Flash write throttled"),
        "Expected throttle error, got: {}",
        err_str
    );
    assert_eq!(manager.session_flash_commit_count(), 1);
}

#[test]
fn test_rgb_manager_low_battery_safety_gate() {
    let config = LightingConfig::default_static(RgbColor::RED);

    // Case 1: Wireless with 15% battery (< 20%) without force must be blocked
    {
        let mut transport = MockTransport::new();
        let safety = SafetyRails::new().with_hardware_writes_permitted(true);
        let mut manager = RgbManager::new(&mut transport, &safety);
        let err = manager.apply_commit(&config, true, Some(15), false);
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(
            msg.contains("Battery is at 15% (< 20%)"),
            "Expected battery warning, got: {}",
            msg
        );
    }

    // Case 2: Wireless with 15% battery WITH force must succeed
    {
        let mut transport = MockTransport::new();
        let safety = SafetyRails::new().with_hardware_writes_permitted(true);
        let mut manager = RgbManager::new(&mut transport, &safety);
        let res = manager.apply_commit(&config, true, Some(15), true);
        assert!(res.is_ok(), "Force override should allow commit");
    }

    // Case 3: Wired connection with 15% battery should not be blocked
    {
        let mut transport = MockTransport::new();
        let safety = SafetyRails::new().with_hardware_writes_permitted(true);
        let mut manager = RgbManager::new(&mut transport, &safety);
        let res_wired = manager.apply_commit(&config, false, Some(15), false);
        assert!(res_wired.is_ok(), "Wired connection should not block on low battery");
    }
}

#[test]
fn test_rgb_profile_serialization_and_roundtrip() {
    let config = LightingConfig::new(LightingMode::Wave, RgbColor::CYAN, 90, 70).unwrap();
    let profile = RgbProfile::new("Monka 3075 Pro", config.clone(), Some("Custom Profile".into()));

    assert_eq!(profile.schema_version, CURRENT_PROFILE_SCHEMA_VERSION);
    let json = profile.to_json().expect("serializes to json");

    let parsed = RgbProfile::from_json(&json).expect("deserializes from json");
    assert_eq!(parsed.model, "Monka 3075 Pro");
    assert_eq!(parsed.lighting.mode, LightingMode::Wave);
    assert_eq!(parsed.lighting.color, RgbColor::CYAN);
    assert_eq!(parsed.lighting.brightness, 90);
    assert_eq!(parsed.lighting.speed, 70);

    // File roundtrip test using tempdir
    let tmp_dir = std::env::temp_dir().join("monkey_rgb_test");
    let file_path = tmp_dir.join("test_profile.json");

    profile.save_to_file(&file_path).expect("saves to file");
    let loaded = RgbProfile::load_from_file(&file_path).expect("loads from file");
    assert_eq!(loaded.lighting, config);

    let _ = std::fs::remove_file(file_path);
}
