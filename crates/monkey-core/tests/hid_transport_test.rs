use monkey_core::error::TransportError;
use monkey_core::transport::hid::{ConnectionState, HidTransport};

#[test]
fn test_frame_bulk_buffer_report_id_zero() {
    // 4096-byte payload representing a bulk LCD / display chunk
    let payload = vec![0xAB; 4096];
    let framed = HidTransport::frame_bulk_buffer(0, &payload);

    // Frame must be 1 byte longer than payload to accommodate userspace Report ID 0
    assert_eq!(framed.len(), 4097);
    assert_eq!(framed[0], 0x00, "First byte must be userspace prefix 0x00 for Report ID 0");
    assert_eq!(&framed[1..], payload.as_slice(), "Payload must follow prefix byte without alteration");
}

#[test]
fn test_frame_bulk_buffer_nonzero_report_id() {
    let payload = [0x11, 0x22, 0x33, 0x44];
    let framed = HidTransport::frame_bulk_buffer(0x05, &payload);

    assert_eq!(framed.len(), 5);
    assert_eq!(framed[0], 0x05, "First byte must be the specified Report ID");
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
}

#[test]
fn test_evaluate_connection_state_wired_timeout() {
    // Wired connection query timing out also indicates sleeping/standby
    let state = HidTransport::evaluate_state_query(false, Err(TransportError::Timeout));
    assert_eq!(state, Ok(ConnectionState::WirelessSleeping));
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
    // Matching wireless signatures
    assert!(HidTransport::is_wireless_product("Monka 3075 Pro Wireless"));
    assert!(HidTransport::is_wireless_product("2.4G Wireless Keyboard Receiver"));
    assert!(HidTransport::is_wireless_product("USB Gaming Receiver"));
    assert!(HidTransport::is_wireless_product("Wireless Dongle"));

    // Wired / standard signatures
    assert!(!HidTransport::is_wireless_product("RKGK890"));
    assert!(!HidTransport::is_wireless_product("Gaming Keyboard"));
    assert!(!HidTransport::is_wireless_product("Monka 3075 Pro"));
    assert!(!HidTransport::is_wireless_product("USB DEVICE"));
}
