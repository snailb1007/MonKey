use monkey_core::error::TransportError;
use monkey_core::protocol::SafetyRails;
use monkey_core::transport::{MockTransport, Transport, TransportCall};

#[test]
fn test_call_recording_fifo_sequence() {
    let mut transport = MockTransport::new();
    let safety = SafetyRails::new().with_hardware_writes_permitted(true);

    // 1. write_bulk
    let bulk_data = vec![0x11, 0x22, 0x33, 0x44];
    let written = transport
        .write_bulk(0x00, &bulk_data, &safety)
        .expect("write_bulk");
    assert_eq!(written, 4);

    // 2. send_feature_report
    let feature_data = vec![0x04, 0x13, 0xAA, 0x55];
    transport
        .send_feature_report(&feature_data, &safety)
        .expect("send_feature_report");

    // 3. get_feature_report
    transport.set_feature_response(0x02, vec![0x02, 0x01, 0x00]);
    let mut get_buf = [0u8; 8];
    let read_len = transport
        .get_feature_report(0x02, &mut get_buf)
        .expect("get_feature_report");
    assert_eq!(read_len, 3);
    assert_eq!(&get_buf[..3], &[0x02, 0x01, 0x00]);

    // 4. read_input_report
    transport.queue_input_report(vec![0x01, 0xFF]);
    let mut in_buf = [0u8; 8];
    let in_len = transport
        .read_input_report(&mut in_buf, 500)
        .expect("read_input_report");
    assert_eq!(in_len, 2);
    assert_eq!(&in_buf[..2], &[0x01, 0xFF]);

    // Assert recorded calls sequence and payloads
    let calls = transport.calls();
    assert_eq!(calls.len(), 4);
    assert_eq!(
        calls[0],
        TransportCall::WriteBulk {
            report_id: 0x00,
            data: vec![0x11, 0x22, 0x33, 0x44]
        }
    );
    assert_eq!(
        calls[1],
        TransportCall::SendFeature {
            data: vec![0x04, 0x13, 0xAA, 0x55]
        }
    );
    assert_eq!(calls[2], TransportCall::GetFeature { report_id: 0x02 });
    assert_eq!(calls[3], TransportCall::ReadInput { timeout_ms: 500 });
}

#[test]
fn test_canned_feature_report_and_buffer_too_small() {
    let mut transport = MockTransport::new();
    let canned = vec![0x04, 0x01, 0x02, 0x03, 0x04, 0x05];
    transport.set_feature_response(0x04, canned.clone());

    // Buffer too small test
    let mut short_buf = [0u8; 4];
    let err = transport
        .get_feature_report(0x04, &mut short_buf)
        .unwrap_err();
    assert_eq!(
        err,
        TransportError::BufferTooSmall {
            needed: 6,
            provided: 4
        }
    );

    // Sufficient buffer test
    let mut exact_buf = [0u8; 6];
    let n = transport
        .get_feature_report(0x04, &mut exact_buf)
        .expect("success");
    assert_eq!(n, 6);
    assert_eq!(&exact_buf, canned.as_slice());

    // Unseeded report ID test
    let mut buf = [0u8; 16];
    let unseeded_err = transport.get_feature_report(0x99, &mut buf).unwrap_err();
    assert_eq!(unseeded_err, TransportError::InvalidReportId(0x99));
}

#[test]
fn test_input_report_fifo_queue_drain_and_timeout() {
    let mut transport = MockTransport::new();

    transport.queue_input_report(vec![0xAA, 0x01]);
    transport.queue_input_report(vec![0xBB, 0x02, 0x03]);

    let mut buf = [0u8; 16];

    // Drain first report
    let n1 = transport.read_input_report(&mut buf, 100).expect("drain 1");
    assert_eq!(n1, 2);
    assert_eq!(&buf[..2], &[0xAA, 0x01]);

    // Drain second report
    let n2 = transport.read_input_report(&mut buf, 100).expect("drain 2");
    assert_eq!(n2, 3);
    assert_eq!(&buf[..3], &[0xBB, 0x02, 0x03]);

    // Queue is empty -> timeout
    let timeout_err = transport.read_input_report(&mut buf, 100).unwrap_err();
    assert_eq!(timeout_err, TransportError::Timeout);
}

#[test]
fn test_assert_no_writes_safety_guard() {
    let mut transport = MockTransport::new();

    // Read-only calls: get_feature and read_input
    transport.set_feature_response(0x01, vec![0x01]);
    let mut buf = [0u8; 8];
    let _ = transport.get_feature_report(0x01, &mut buf);
    transport.queue_input_report(vec![0x02]);
    let _ = transport.read_input_report(&mut buf, 50);

    // Should pass when no writes have occurred (D-12 read-only invariant)
    assert!(transport.assert_no_writes().is_ok());

    // Now invoke write_bulk
    let safety = SafetyRails::new().with_hardware_writes_permitted(true);
    let _ = transport.write_bulk(0x00, &[0xDE, 0xAD], &safety);
    let err = transport.assert_no_writes().unwrap_err();
    assert!(err.contains("Expected no write calls"));

    // Reset and try send_feature_report
    transport.clear_calls();
    assert!(transport.assert_no_writes().is_ok());
    let _ = transport.send_feature_report(&[0x04, 0x13], &safety);
    let err2 = transport.assert_no_writes().unwrap_err();
    assert!(err2.contains("Expected no write calls"));
}

#[test]
fn test_injected_errors_fail_without_side_effects() {
    let mut transport = MockTransport::new();
    transport.set_feature_response(0x01, vec![0x01, 0x02]);
    transport.queue_input_report(vec![0x03, 0x04]);

    // Inject Disconnected error
    transport.inject_error(TransportError::Disconnected);

    let mut buf = [0u8; 16];

    // All operations fail immediately
    let safety = SafetyRails::new().with_hardware_writes_permitted(true);
    assert_eq!(
        transport.write_bulk(0x00, &[0x01], &safety).unwrap_err(),
        TransportError::Disconnected
    );
    assert_eq!(
        transport.send_feature_report(&[0x01], &safety).unwrap_err(),
        TransportError::Disconnected
    );
    assert_eq!(
        transport.get_feature_report(0x01, &mut buf).unwrap_err(),
        TransportError::Disconnected
    );
    assert_eq!(
        transport.read_input_report(&mut buf, 100).unwrap_err(),
        TransportError::Disconnected
    );

    // Verify no calls were recorded during failure
    assert!(transport.calls().is_empty());

    // Clear error and verify queued items are intact
    transport.clear_injected_error();
    let n = transport
        .read_input_report(&mut buf, 100)
        .expect("queue intact");
    assert_eq!(n, 2);
    assert_eq!(&buf[..2], &[0x03, 0x04]);
    assert_eq!(transport.calls().len(), 1);
}

#[test]
fn test_get_feature_report_report_zero_layout_alignment() {
    let mut transport = MockTransport::new();
    let payload = vec![0x42u8; 64];
    transport.set_feature_response(0, payload.clone());

    // Buffer of 64 bytes is too small (needs 65 for leading Report ID 0)
    let mut short_buf = [0u8; 64];
    let err = transport.get_feature_report(0, &mut short_buf).unwrap_err();
    assert_eq!(
        err,
        TransportError::BufferTooSmall {
            needed: 65,
            provided: 64,
        }
    );

    // Buffer of 65 bytes succeeds, seeding buf[0] = 0 and placing payload at buf[1..]
    let mut probe_buf = [0xFFu8; 65];
    let n = transport
        .get_feature_report(0, &mut probe_buf)
        .expect("success");
    assert_eq!(n, 65);
    assert_eq!(probe_buf[0], 0x00, "Report ID must be placed at index 0");
    assert_eq!(
        &probe_buf[1..],
        payload.as_slice(),
        "Payload must be copied to buf[1..]"
    );
}

#[test]
fn test_mock_transport_rejects_writes_without_permission() {
    let mut transport = MockTransport::new();
    let blocked_safety = SafetyRails::new();

    let err1 = transport
        .write_bulk(0x00, &[0x01], &blocked_safety)
        .unwrap_err();
    assert!(matches!(err1, TransportError::ProtocolViolation(msg) if msg.contains("Hardware writes are disabled")));

    let err2 = transport
        .send_feature_report(&[0x01], &blocked_safety)
        .unwrap_err();
    assert!(matches!(err2, TransportError::ProtocolViolation(msg) if msg.contains("Hardware writes are disabled")));
}
