use std::ffi::CString;
use std::process::Command;

use monkey_cli::commands::info::{
    build_info_output, format_info_human, run_info_with_device_set, DeviceInfoOutput,
};
use monkey_cli::commands::probe::{
    format_probe_human, probe_device_with_transport, run_probe_with_transport, ProbeOutput,
};
use monkey_cli::output::OutputFormat;
use monkey_core::device::{
    DiscoveredDevice, MonkaDeviceSet, MONKA_PID, MONKA_VID, PRODUCT_IDENTIFIER,
};
use monkey_core::protocol::FeatureReportPacket;
use monkey_core::transport::{MockTransport, TransportCall};

/// Helper creating a simulated dual-interface Monka 3075 Pro device set.
fn create_mock_monka_set() -> MonkaDeviceSet {
    let mut set = MonkaDeviceSet::new(MONKA_VID, MONKA_PID);
    set.product = Some(PRODUCT_IDENTIFIER.to_string());
    set.manufacturer = Some("Shenzhen HFD Technology Co., Ltd.".to_string());
    set.serial_number = Some("MONKA3075PRO-001".to_string());

    let dev_a = DiscoveredDevice {
        vid: MONKA_VID,
        pid: MONKA_PID,
        manufacturer: set.manufacturer.clone(),
        product: set.product.clone(),
        serial_number: set.serial_number.clone(),
        path: CString::new("/dev/hidraw_monka_bulk_pipe").unwrap(),
        usage_page: 0xFF68,
        usage: 0x0061,
        interface_number: 0,
    };

    let dev_b = DiscoveredDevice {
        vid: MONKA_VID,
        pid: MONKA_PID,
        manufacturer: set.manufacturer.clone(),
        product: set.product.clone(),
        serial_number: set.serial_number.clone(),
        path: CString::new("/dev/hidraw_monka_ctrl_pipe").unwrap(),
        usage_page: 0x000C,
        usage: 0x0001,
        interface_number: 1,
    };

    set.interface_a = Some(dev_a);
    set.interface_b = Some(dev_b);
    set.observed_usages = vec![(0xFF68, 0x0061), (0x000C, 0x0001), (0xFFFF, 0x0001)];
    set
}

#[test]
fn test_info_json_schema_and_identifiers() {
    let set = create_mock_monka_set();
    let mut buf = Vec::new();

    run_info_with_device_set(&set, OutputFormat::Json, &mut buf)
        .expect("run_info_with_device_set failed");

    let json_str = String::from_utf8(buf).expect("Invalid UTF-8 in JSON output");
    let v: serde_json::Value = serde_json::from_str(&json_str).expect("Failed to parse JSON");

    // Test 1 Behaviors: parses into valid JSON containing VID "0x05ac", PID "0x024f", and product name "RKGK890"
    assert_eq!(v["vid"], "0x05ac");
    assert_eq!(v["pid"], "0x024f");
    assert_eq!(v["vid_hex"], "0x05ac");
    assert_eq!(v["pid_hex"], "0x024f");
    assert_eq!(v["vid_dec"], 1452);
    assert_eq!(v["pid_dec"], 591);
    assert_eq!(v["product_name"], "RKGK890");
    assert_eq!(v["serial_number"], "MONKA3075PRO-001");
    assert_eq!(v["release_number"], "1.00");

    let interfaces = v["interfaces"]
        .as_array()
        .expect("interfaces must be array");
    assert_eq!(interfaces.len(), 2);

    let iface_a = &interfaces[0];
    assert_eq!(iface_a["role"], "InterfaceA");
    assert_eq!(iface_a["usage_page"], "0xff68");
    assert_eq!(iface_a["usage"], "0x0061");
    assert_eq!(iface_a["max_output_bytes"], 4096);
    assert_eq!(iface_a["max_input_bytes"], 64);

    let iface_b = &interfaces[1];
    assert_eq!(iface_b["role"], "InterfaceB");
    assert_eq!(iface_b["usage_page"], "0x000c");
    assert_eq!(iface_b["usage"], "0x0001");
    assert_eq!(iface_b["max_feature_bytes"], 64);

    // Also verify typed deserialization
    let typed: DeviceInfoOutput =
        serde_json::from_str(&json_str).expect("Failed typed deserialization");
    assert_eq!(typed.vid, "0x05ac");
    assert_eq!(typed.pid, "0x024f");
    assert_eq!(typed.product_name, "RKGK890");
}

#[test]
fn test_probe_json_schema_and_capability_tuple() {
    let set = create_mock_monka_set();
    let mut mock = MockTransport::new();
    let mut valid_packet = FeatureReportPacket::new(1);
    valid_packet.args[0] = 0; // rev1.0
    valid_packet.args[1] = 1; // v1.0.0
    valid_packet.args[2] = 0;
    valid_packet.args[3] = 0;
    mock.set_feature_response(0, valid_packet.as_bytes().to_vec());

    let mut buf = Vec::new();
    run_probe_with_transport(&mut mock, Some(&set), false, OutputFormat::Json, &mut buf)
        .expect("run_probe_with_transport failed");

    let json_str = String::from_utf8(buf).expect("Invalid UTF-8 in JSON output");
    let v: serde_json::Value = serde_json::from_str(&json_str).expect("Failed to parse probe JSON");

    // Test 2 Behaviors: parses into valid JSON containing model, hardware_revision, firmware_version, transport_state, and capabilities array
    assert_eq!(v["model"], "Monka 3075 Pro / RKGK890");
    assert_eq!(v["hardware_revision"], "rev1.0");
    assert_eq!(v["firmware_version"], "v1.0.0");
    assert_eq!(v["transport_state"], "WiredUsb");
    assert_eq!(v["read_only_verified"], true);
    assert_eq!(v["interface_a_detected"], true);
    assert_eq!(v["interface_b_detected"], true);

    let caps = v["capabilities"]
        .as_array()
        .expect("capabilities must be array");
    assert_eq!(caps.len(), 3);
    let cap_strings: Vec<&str> = caps.iter().map(|c| c.as_str().unwrap()).collect();
    assert!(cap_strings.contains(&"LCD display 128x128 RGB565"));
    assert!(cap_strings.contains(&"81-key RGB matrix"));
    assert!(cap_strings.contains(&"dual composite interface"));

    // Also verify typed deserialization
    let typed: ProbeOutput = serde_json::from_str(&json_str).expect("Failed typed deserialization");
    assert_eq!(typed.model, "Monka 3075 Pro / RKGK890");
    assert_eq!(typed.transport_state, "WiredUsb");
}

#[test]
fn test_probe_enforces_read_only_safety_invariant() {
    let set = create_mock_monka_set();
    let mut mock = MockTransport::new();
    mock.set_feature_response(0, vec![0u8; 64]);

    let probe_res = probe_device_with_transport(&mut mock, Some(&set), false);
    assert!(probe_res.read_only_verified);

    // Test 3 Behavior: monkey probe executed against MockTransport invokes mock.assert_no_writes() successfully,
    // proving zero writes were emitted during probing per D-12.
    mock.assert_no_writes()
        .expect("assert_no_writes failed during probing!");

    // Verify exactly what calls occurred: only read queries, zero writes
    for call in mock.calls() {
        match call {
            TransportCall::WriteBulk { .. } => {
                panic!("Disallowed write_bulk was invoked during probing!")
            }
            TransportCall::SendFeature { .. } => {
                panic!("Disallowed send_feature_report was invoked during probing!")
            }
            TransportCall::GetFeature { report_id } => {
                assert_eq!(*report_id, 0, "Probing should only read Report ID 0");
            }
            TransportCall::ReadInput { .. } => {}
        }
    }
}

#[test]
fn test_probe_wireless_sleeping_transport_state() {
    let set = create_mock_monka_set();
    let mut mock = MockTransport::new();
    // In wireless mode, when queries time out, evaluate_state_query returns WirelessSleeping
    mock.inject_error(monkey_core::TransportError::Timeout);

    let probe_res = probe_device_with_transport(&mut mock, Some(&set), true);
    assert_eq!(probe_res.transport_state, "WirelessSleeping");
    assert!(probe_res.read_only_verified);
    mock.assert_no_writes().expect("assert_no_writes failed");
}

#[test]
fn test_info_human_output_formatting() {
    let set = create_mock_monka_set();
    let info = build_info_output(&set);
    let human = format_info_human(&info);

    assert!(human.contains("MonKey Hardware Information"));
    assert!(human.contains("0x05ac"));
    assert!(human.contains("0x024f"));
    assert!(human.contains("RKGK890"));
    assert!(human.contains("Interface A (Vendor Bulk Pipe)"));
    assert!(human.contains("Interface B (Standard/Vendor Control Pipe)"));
}

#[test]
fn test_probe_human_output_formatting() {
    let set = create_mock_monka_set();
    let mut mock = MockTransport::new();
    mock.set_feature_response(0, vec![0u8; 64]);

    let output = probe_device_with_transport(&mut mock, Some(&set), false);
    let human = format_probe_human(&output);

    assert!(human.contains("MonKey Device Capability Probe"));
    assert!(human.contains("Monka 3075 Pro / RKGK890"));
    assert!(human.contains("[✓] LCD display 128x128 RGB565"));
    assert!(human.contains("[✓] 81-key RGB matrix"));
    assert!(human.contains("VERIFIED (0 flash writes)"));
}

#[test]
fn test_cli_no_device_found_error_exit() {
    // Test 4 Behavior: When no matching device is found, CLI outputs a clear,
    // user-friendly error message without panicking, and exits with a standard non-zero error code.
    let bin_path = env!("CARGO_BIN_EXE_monkey");

    let output = Command::new(bin_path)
        .arg("info")
        .env("MONKEY_SIMULATE_EMPTY", "1")
        .output()
        .expect("Failed to execute monkey binary");

    // Standard non-zero error exit code (1)
    assert!(
        !output.status.success(),
        "Command should exit with error code when no device is found"
    );
    assert_eq!(output.status.code(), Some(1), "Expected exit code 1");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No Monka 3075 Pro / RKGK890 keyboard detected"),
        "Expected user-friendly error in stderr, got: {stderr}"
    );
    assert!(stderr.contains("0x05ac"), "Expected VID in error message");
    assert!(stderr.contains("0x024f"), "Expected PID in error message");
    // Verify it did not panic
    assert!(
        !stderr.contains("panicked at"),
        "CLI must not panic on missing device"
    );

    // Also verify probe subcommand exits with code 1 and friendly error
    let probe_output = Command::new(bin_path)
        .arg("probe")
        .env("MONKEY_SIMULATE_EMPTY", "1")
        .output()
        .expect("Failed to execute monkey binary");

    assert!(!probe_output.status.success());
    assert_eq!(probe_output.status.code(), Some(1));
    let probe_stderr = String::from_utf8_lossy(&probe_output.stderr);
    assert!(probe_stderr.contains("No Monka 3075 Pro / RKGK890 keyboard detected"));
    assert!(!probe_stderr.contains("panicked at"));
}

#[test]
fn test_probe_dynamic_capabilities_when_interface_a_absent() {
    let mut set = create_mock_monka_set();
    // Simulate Bluetooth connection where Interface A (Bulk Pipe) is absent
    set.interface_a = None;

    let mut mock = MockTransport::new();
    mock.set_feature_response(0, vec![0u8; 64]);

    let output = probe_device_with_transport(&mut mock, Some(&set), true);
    assert!(!output.interface_a_detected);
    assert!(output.interface_b_detected);

    // Capabilities must NOT claim LCD display or dual composite interface
    assert_eq!(output.capabilities, vec!["81-key RGB matrix".to_string()]);
    assert!(!output
        .capabilities
        .contains(&"LCD display 128x128 RGB565".to_string()));
    assert!(!output
        .capabilities
        .contains(&"dual composite interface".to_string()));
}

#[test]
fn test_probe_reflects_transport_query_error_state() {
    let set = create_mock_monka_set();
    let mut mock = MockTransport::new();
    mock.inject_error(monkey_core::TransportError::Disconnected);

    let output = probe_device_with_transport(&mut mock, Some(&set), false);
    assert_eq!(output.transport_state, "Unknown/Error: Device disconnected");
}

#[test]
fn test_info_empty_serial_number_formats_as_na() {
    let mut set = create_mock_monka_set();
    // Simulate macOS empty string serial number
    set.serial_number = Some("".to_string());

    let info = build_info_output(&set);
    assert_eq!(info.serial_number, Some("N/A".to_string()));

    let human = format_info_human(&info);
    assert!(human.contains("Serial Number:          N/A"));
}
