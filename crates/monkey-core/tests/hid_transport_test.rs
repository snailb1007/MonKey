use monkey_core::device::{DiscoveredDevice, MonkaDevice, MonkaDeviceSet, MONKA_PID, MONKA_VID};
use monkey_core::error::{MonkeyError, TransportError};
use monkey_core::rgb::{FlowDirection, LightingConfig, LightingMode, RgbColor};
use monkey_core::transport::hid::{ConnectionState, HidTransport};
use monkey_core::transport::{MockTransport, Transport};
use std::ffi::CString;
use std::sync::{Arc, Mutex};

#[test]
fn test_frame_bulk_buffer_report_id_zero() {
    // 4096-byte payload representing a bulk LCD / display chunk
    let payload = vec![0xAB; 4096];
    let framed = HidTransport::frame_bulk_buffer(0, &payload);

    // Frame must be 1 byte longer than payload to accommodate userspace Report ID 0
    assert_eq!(framed.len(), 4097);
    assert_eq!(
        framed[0], 0x00,
        "First byte must be userspace prefix 0x00 for Report ID 0"
    );
    assert_eq!(
        &framed[1..],
        payload.as_slice(),
        "Payload must follow prefix byte without alteration"
    );
}

#[test]
fn test_frame_bulk_buffer_nonzero_report_id() {
    let payload = [0x11, 0x22, 0x33, 0x44];
    let framed = HidTransport::frame_bulk_buffer(0x05, &payload);

    assert_eq!(framed.len(), 5);
    assert_eq!(
        framed[0], 0x05,
        "First byte must be the specified Report ID"
    );
    assert_eq!(&framed[1..], &payload);
}

#[test]
fn test_frame_bulk_buffer_empty_payload() {
    let payload = [];
    let framed = HidTransport::frame_bulk_buffer(0, &payload);

    assert_eq!(framed.len(), 1);
    assert_eq!(framed[0], 0x00);
}

#[test]
fn test_calculate_payload_written() {
    // 4097 raw bytes written (1 byte prefix + 4096 payload) -> 4096 payload bytes
    assert_eq!(HidTransport::calculate_payload_written(4097), 4096);
    // 65 raw bytes written (1 byte prefix + 64 payload) -> 64 payload bytes
    assert_eq!(HidTransport::calculate_payload_written(65), 64);
    // 1 raw byte written (only the prefix was written) -> 0 payload bytes
    assert_eq!(HidTransport::calculate_payload_written(1), 0);
    // 0 raw bytes written -> 0 payload bytes
    assert_eq!(HidTransport::calculate_payload_written(0), 0);
}

#[test]
fn test_prepare_feature_buffer_success() {
    let mut buf = [0xFFu8; 65];
    let res = HidTransport::prepare_feature_buffer(0x04, &mut buf);

    assert!(res.is_ok());
    assert_eq!(buf[0], 0x04, "Report ID must be seeded into index 0");
    assert_eq!(buf[1], 0xFF, "Subsequent bytes must remain intact");
}

#[test]
fn test_prepare_feature_buffer_too_small() {
    let mut empty_buf: [u8; 0] = [];
    let res = HidTransport::prepare_feature_buffer(0, &mut empty_buf);

    assert_eq!(
        res,
        Err(TransportError::BufferTooSmall {
            needed: 1,
            provided: 0,
        })
    );
}

#[test]
fn test_evaluate_connection_state_wired_success() {
    // Wired USB device responds to non-destructive query
    let state = HidTransport::evaluate_state_query(false, Ok(64));
    assert_eq!(state, Ok(ConnectionState::WiredUsb));
}

#[test]
fn test_evaluate_connection_state_wireless_awake() {
    // Wireless dongle responds to non-destructive query while keyboard is awake
    let state = HidTransport::evaluate_state_query(true, Ok(64));
    assert_eq!(state, Ok(ConnectionState::WirelessAwake));
}

#[test]
fn test_evaluate_connection_state_wireless_sleeping() {
    // Wireless dongle query times out because keyboard is sleeping
    let state = HidTransport::evaluate_state_query(true, Err(TransportError::Timeout));
    assert_eq!(state, Ok(ConnectionState::WirelessSleeping));

    // Zero bytes read also indicates keyboard is sleeping / not responding
    let state_zero = HidTransport::evaluate_state_query(true, Ok(0));
    assert_eq!(state_zero, Ok(ConnectionState::WirelessSleeping));
}

#[test]
fn test_evaluate_connection_state_wired_zero_bytes() {
    // Wired connection returning zero bytes is treated as timeout / not responding
    let state = HidTransport::evaluate_state_query(false, Ok(0));
    assert_eq!(state, Err(TransportError::Timeout));
}

#[test]
fn test_evaluate_connection_state_wired_timeout() {
    // Wired connection query timing out should return Timeout error, not WirelessSleeping
    let state = HidTransport::evaluate_state_query(false, Err(TransportError::Timeout));
    assert_eq!(state, Err(TransportError::Timeout));
}

#[test]
fn test_evaluate_connection_state_error_propagation() {
    // Non-timeout errors like Disconnected or HidError propagate directly
    let disconn = HidTransport::evaluate_state_query(false, Err(TransportError::Disconnected));
    assert_eq!(disconn, Err(TransportError::Disconnected));

    let hid_err = HidTransport::evaluate_state_query(
        true,
        Err(TransportError::HidError("USB transfer failed".to_string())),
    );
    assert_eq!(
        hid_err,
        Err(TransportError::HidError("USB transfer failed".to_string()))
    );
}

#[test]
fn test_wireless_product_detection_heuristics() {
    // Matching wireless and Bluetooth signatures
    assert!(HidTransport::is_wireless_product("Monka 3075 Pro Wireless"));
    assert!(HidTransport::is_wireless_product(
        "2.4G Wireless Keyboard Receiver"
    ));
    assert!(HidTransport::is_wireless_product("USB Gaming Receiver"));
    assert!(HidTransport::is_wireless_product("Wireless Dongle"));
    assert!(HidTransport::is_wireless_product("BT5.1-KB"));
    assert!(HidTransport::is_wireless_product(
        "Monka Bluetooth Keyboard"
    ));
    assert!(HidTransport::is_wireless_product("BT-Keyboard"));

    // Wired / standard signatures
    assert!(!HidTransport::is_wireless_product("RKGK890"));
    assert!(!HidTransport::is_wireless_product("Gaming Keyboard"));
    assert!(!HidTransport::is_wireless_product("Monka 3075 Pro"));
    assert!(!HidTransport::is_wireless_product("USB DEVICE"));
}

// --- 08-01 T2: Windows feature-report framing + no-interface guard ---

/// 64-byte Monka feature payload with the valid magic prefix.
fn valid_feature_payload(command: u8) -> [u8; 64] {
    let mut payload = [0u8; 64];
    payload[0] = 0x04;
    payload[1] = command;
    payload
}

#[test]
fn test_normalize_feature_frame_raw64_prefixes_report_id() {
    let raw = valid_feature_payload(0x0D);
    let frame = HidTransport::normalize_feature_frame(&raw).expect("raw64 must normalize");
    assert_eq!(frame.len(), 65);
    assert_eq!(
        frame[0], 0x00,
        "hidapi Report ID 0 must lead the wire frame"
    );
    assert_eq!(
        &frame[1..],
        &raw,
        "payload must follow the prefix unaltered"
    );
}

#[test]
fn test_normalize_feature_frame_prefixed65_is_identical() {
    let raw = valid_feature_payload(0x0D);
    let mut prefixed = [0u8; 65];
    prefixed[0] = 0x00;
    prefixed[1..].copy_from_slice(&raw);
    let frame =
        HidTransport::normalize_feature_frame(&prefixed).expect("prefixed65 must pass through");
    assert_eq!(
        &frame, &prefixed,
        "already-framed input must not be reinterpreted"
    );
}

#[test]
fn test_normalize_feature_frame_rejects_zero_length() {
    assert!(
        matches!(
            HidTransport::normalize_feature_frame(&[]),
            Err(TransportError::BufferTooSmall { .. })
        ),
        "zero-length input must fail before any handle is touched"
    );
}

#[test]
fn test_normalize_feature_frame_rejects_oversized() {
    let oversized = vec![0x04u8; 100];
    assert!(
        matches!(
            HidTransport::normalize_feature_frame(&oversized),
            Err(TransportError::ProtocolViolation(_))
        ),
        "oversized input must fail before any handle is touched"
    );
}

#[test]
fn test_normalize_feature_frame_rejects_nonzero_prefix() {
    let mut bad = [0u8; 65];
    bad[0] = 0x04;
    bad[1] = 0x04;
    assert!(
        matches!(
            HidTransport::normalize_feature_frame(&bad),
            Err(TransportError::InvalidReportId(0x04))
        ),
        "a nonzero Report ID prefix must never be reinterpreted as magic04"
    );
}

#[test]
fn test_normalize_feature_frame_rejects_wrong_magic() {
    let mut bad = [0u8; 64];
    bad[0] = 0x09;
    assert!(
        matches!(
            HidTransport::normalize_feature_frame(&bad),
            Err(TransportError::ProtocolViolation(_))
        ),
        "a payload without the magic04 header must fail before any handle is touched"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn test_macos_branch_forwards_raw64_tail() {
    // The macOS-conditional control-pipe branch accepts the raw64 shape: the
    // normalized frame's tail is byte-identical to the raw64 input it forwards.
    let raw = valid_feature_payload(0x12);
    let frame = HidTransport::normalize_feature_frame(&raw).expect("raw64 must normalize");
    assert_eq!(&frame[1..], &raw, "macOS pipe-facing tail must stay raw64");
}

/// Shared-ownership mock so the test can inspect recorded calls after the
/// transport is moved into `MonkaDevice`.
struct SharedMock(Arc<Mutex<MockTransport>>);

impl Transport for SharedMock {
    fn write_bulk(&mut self, report_id: u8, data: &[u8]) -> Result<usize, TransportError> {
        self.0.lock().unwrap().write_bulk(report_id, data)
    }

    fn send_feature_report(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.0.lock().unwrap().send_feature_report(data)
    }

    fn get_feature_report(
        &mut self,
        report_id: u8,
        buf: &mut [u8],
    ) -> Result<usize, TransportError> {
        self.0.lock().unwrap().get_feature_report(report_id, buf)
    }

    fn read_input_report(
        &mut self,
        buf: &mut [u8],
        timeout_ms: i32,
    ) -> Result<usize, TransportError> {
        self.0.lock().unwrap().read_input_report(buf, timeout_ms)
    }
}

fn preview_config() -> LightingConfig {
    LightingConfig {
        mode: LightingMode::Static,
        brightness: 100,
        speed: 50,
        direction: FlowDirection::LeftToRight,
        color: RgbColor::new(255, 0, 0),
    }
}

fn interface_a_only_set() -> MonkaDeviceSet {
    let mut set = MonkaDeviceSet::new(MONKA_VID, MONKA_PID);
    set.interface_a = Some(DiscoveredDevice {
        vid: MONKA_VID,
        pid: MONKA_PID,
        manufacturer: None,
        product: None,
        serial_number: None,
        path: CString::new("path_a").expect("Valid CString"),
        usage_page: 0xFF68,
        usage: 0x0061,
        interface_number: 0,
    });
    set
}

#[test]
fn test_apply_rgb_preview_without_feature_interface_fails_fast() {
    let shared = Arc::new(Mutex::new(MockTransport::new()));
    let mut device = MonkaDevice::from_transport(Box::new(SharedMock(shared.clone())))
        .with_device_set(interface_a_only_set());
    let err = device
        .apply_rgb_preview(&preview_config())
        .expect_err("preview with no feature interface must fail");
    assert!(
        matches!(
            err,
            MonkeyError::Transport(TransportError::DeviceNotFound(_))
        ),
        "no-interface failure path must surface DeviceNotFound, got: {err:?}"
    );
    shared
        .lock()
        .unwrap()
        .assert_no_writes()
        .expect("guard must fail before any I/O is recorded");
}

#[test]
fn test_apply_rgb_preview_unbound_transport_still_succeeds() {
    // Unbound mock/test context (no device set): allowed, preserving existing behavior.
    let mut device = MonkaDevice::from_transport(Box::new(MockTransport::new()))
        .with_hardware_writes_allowed(true);
    device
        .apply_rgb_preview(&preview_config())
        .expect("unbound preview must succeed");
}

#[test]
fn test_get_feature_report_rejects_empty_buffer() {
    let mut mock = MockTransport::new();
    let mut buf = [];
    let err = mock
        .get_feature_report(0, &mut buf)
        .expect_err("empty buffer must fail");
    assert!(
        matches!(err, TransportError::BufferTooSmall { .. }),
        "got: {err:?}"
    );
}

#[test]
fn test_get_feature_report_round_trips_canned_response() {
    let mut mock = MockTransport::new();
    let mut resp = vec![0u8; 64];
    resp[0] = 0x04;
    resp[14] = 0xAA;
    resp[15] = 0x55;
    mock.set_feature_response(0, resp.clone());

    let mut buf = [0u8; 65];
    let n = mock
        .get_feature_report(0, &mut buf)
        .expect("round trip must succeed");
    assert_eq!(n, 65);
    assert_eq!(buf[0], 0x00);
    assert_eq!(&buf[1..], resp.as_slice());
}
