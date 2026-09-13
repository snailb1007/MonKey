use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use monkey_core::{
    slice_lcd_frame, BulkChunkPacket, FeatureReportPacket, HardwareChannel, SafetyRails,
    TransactionManager, Transport, TransportCall, TransportError, WriteMode,
};

// Logs attempts before returning errors, so assertions detect an invalid send
// even when transport would have rejected it. Shared history survives worker ownership.
#[derive(Default)]
struct RecordingTransport {
    calls: Arc<Mutex<Vec<TransportCall>>>,
    bulk_results: VecDeque<Result<usize, TransportError>>,
    feature_error: Option<TransportError>,
}

impl Transport for RecordingTransport {
    fn write_bulk(
        &mut self,
        report_id: u8,
        data: &[u8],
        safety: &SafetyRails,
    ) -> Result<usize, TransportError> {
        safety
            .validate_hardware_write_permitted()
            .map_err(|e| TransportError::ProtocolViolation(e.to_string()))?;
        self.calls.lock().unwrap().push(TransportCall::WriteBulk {
            report_id,
            data: data.to_vec(),
        });
        self.bulk_results.pop_front().unwrap_or(Ok(data.len()))
    }

    fn send_feature_report(
        &mut self,
        data: &[u8],
        safety: &SafetyRails,
    ) -> Result<(), TransportError> {
        safety
            .validate_hardware_write_permitted()
            .map_err(|e| TransportError::ProtocolViolation(e.to_string()))?;
        self.calls.lock().unwrap().push(TransportCall::SendFeature {
            data: data.to_vec(),
        });
        match &self.feature_error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    fn get_feature_report(&mut self, report_id: u8, _: &mut [u8]) -> Result<usize, TransportError> {
        self.calls
            .lock()
            .unwrap()
            .push(TransportCall::GetFeature { report_id });
        Err(TransportError::Timeout)
    }

    fn read_input_report(
        &mut self,
        _: &mut [u8],
        timeout_ms: i32,
    ) -> Result<usize, TransportError> {
        self.calls
            .lock()
            .unwrap()
            .push(TransportCall::ReadInput { timeout_ms });
        Err(TransportError::Timeout)
    }
}

// Deliberately independent of CommandId/SAFE_WRITE_COMMANDS: catches a wrong enum
// value or a policy accidentally widened to an assumed opcode.
fn expected_allowed(opcode: u8, mode: WriteMode) -> bool {
    match mode {
        WriteMode::RamPreview => matches!(opcode, 0x18 | 0x13 | 0x20 | 0xF0 | 0xF5),
        WriteMode::FlashCommit => opcode == 0x02,
    }
}

#[test]
fn manager_checks_every_opcode_and_mode_against_actual_packet() {
    let mut transport = RecordingTransport::default();
    let calls = transport.calls.clone();
    let rails = SafetyRails::with_flash_debounce(Duration::ZERO)
        .with_hardware_writes_permitted(true)
        .with_active_transaction(true)
        .with_verified_ram_preview(true);
    let mut manager =
        TransactionManager::new(&mut transport, &rails).with_delays(Duration::ZERO, Duration::ZERO);
    for opcode in 0..=u8::MAX {
        for mode in [WriteMode::RamPreview, WriteMode::FlashCommit] {
            // Mutate after construction: validation must inspect the current bytes.
            let mut packet = FeatureReportPacket::new(0x13);
            packet.command = opcode;
            calls.lock().unwrap().clear();
            let result = manager.send_feature_command(&packet, mode);
            let allowed = expected_allowed(opcode, mode);
            assert_eq!(result.is_ok(), allowed, "{opcode:02X}, {mode:?}");
            let history = calls.lock().unwrap();
            if allowed {
                assert_eq!(
                    *history,
                    vec![TransportCall::SendFeature {
                        data: packet.as_bytes().to_vec()
                    }]
                );
            } else {
                assert!(history.is_empty(), "invalid input reached transport");
            }
        }
    }
}

#[test]
fn worker_checks_every_opcode_and_mode_before_transport() {
    let transport = RecordingTransport::default();
    let calls = transport.calls.clone();
    let channel = HardwareChannel::spawn(
        transport,
        Arc::new(
            SafetyRails::with_flash_debounce(Duration::ZERO)
                .with_hardware_writes_permitted(true)
                .with_active_transaction(true)
                .with_verified_ram_preview(true),
        ),
    );
    for opcode in 0..=u8::MAX {
        for mode in [WriteMode::RamPreview, WriteMode::FlashCommit] {
            let mut packet = FeatureReportPacket::new(0x13);
            packet.command = opcode;
            calls.lock().unwrap().clear();
            let result = channel.send_feature(packet, mode);
            let allowed = expected_allowed(opcode, mode);
            assert_eq!(result.is_ok(), allowed, "{opcode:02X}, {mode:?}");
            let history = calls.lock().unwrap();
            if allowed {
                assert_eq!(
                    *history,
                    vec![TransportCall::SendFeature {
                        data: packet.as_bytes().to_vec()
                    }]
                );
            } else {
                assert!(history.is_empty());
            }
        }
    }
    channel.close();
}

#[test]
fn invalid_headers_never_reach_manager_or_worker_transport() {
    for mode in [WriteMode::RamPreview, WriteMode::FlashCommit] {
        for invalid in 0..3 {
            let mut packet = FeatureReportPacket::new(if mode == WriteMode::FlashCommit {
                0x02
            } else {
                0x13
            });
            match invalid {
                0 => packet.magic = 0,
                1 => packet.marker[0] = 0,
                _ => packet.marker[1] = 0,
            }
            let mut transport = RecordingTransport::default();
            let calls = transport.calls.clone();
            let rails = Arc::new(SafetyRails::new());
            {
                let mut manager = TransactionManager::new(&mut transport, &rails);
                assert!(manager.send_feature_command(&packet, mode).is_err());
            }
            assert!(calls.lock().unwrap().is_empty());
            let channel = HardwareChannel::spawn(transport, rails);
            assert!(channel.send_feature(packet, mode).is_err());
            assert!(calls.lock().unwrap().is_empty());
            channel.close();
        }
    }
}

#[test]
fn flash_debounce_and_mode_rejection_precede_transport() {
    let mut transport = RecordingTransport::default();
    let calls = transport.calls.clone();
    let rails = SafetyRails::with_flash_debounce(Duration::from_millis(100))
        .with_hardware_writes_permitted(true)
        .with_active_transaction(true)
        .with_verified_ram_preview(true);
    let mut manager =
        TransactionManager::new(&mut transport, &rails).with_delays(Duration::ZERO, Duration::ZERO);
    let commit = FeatureReportPacket::new(0x02);
    assert!(manager
        .send_feature_command(&commit, WriteMode::RamPreview)
        .is_err());
    let mut malformed = commit;
    malformed.magic = 0;
    assert!(manager
        .send_feature_command(&malformed, WriteMode::FlashCommit)
        .is_err());
    // Invalid calls must not reserve the debounce window.
    manager
        .send_feature_command(&commit, WriteMode::FlashCommit)
        .unwrap();
    assert!(manager
        .send_feature_command(&commit, WriteMode::FlashCommit)
        .is_err());
    assert_eq!(calls.lock().unwrap().len(), 1);
    let preview = FeatureReportPacket::new(0x13);
    manager
        .send_feature_command(&preview, WriteMode::RamPreview)
        .unwrap();
    std::thread::sleep(Duration::from_millis(110));
    manager
        .send_feature_command(&commit, WriteMode::FlashCommit)
        .unwrap();
    assert_eq!(calls.lock().unwrap().len(), 3);
}

#[test]
fn manager_and_worker_stream_eight_exact_raw_lcd_reports() {
    let frame: Vec<u8> = (0..32768).map(|i| (i % 251) as u8).collect();
    let packets = slice_lcd_frame(&frame).unwrap();
    let expected: Vec<_> = frame
        .as_chunks::<4096>()
        .0
        .iter()
        .map(|data| TransportCall::WriteBulk {
            report_id: 0,
            data: data.to_vec(),
        })
        .collect();
    let mut transport = RecordingTransport::default();
    let calls = transport.calls.clone();
    let rails = Arc::new(
        SafetyRails::new()
            .with_hardware_writes_permitted(true)
            .with_active_transaction(true),
    );
    let mut progress = Vec::new();
    {
        let mut manager = TransactionManager::new(&mut transport, &rails)
            .with_delays(Duration::ZERO, Duration::ZERO);
        manager
            .stream_bulk_chunks(&packets, |sent, total| progress.push((sent, total)))
            .unwrap();
        manager
            .stream_bulk_chunks(&[], |_, _| panic!("empty transfer callback"))
            .unwrap();
    }
    assert_eq!(progress, (1..=8).map(|i| (i, 8)).collect::<Vec<_>>());
    assert_eq!(*calls.lock().unwrap(), expected);
    calls.lock().unwrap().clear();
    let channel = HardwareChannel::spawn(transport, rails);
    channel.stream_bulk(packets).unwrap();
    assert_eq!(*calls.lock().unwrap(), expected);
    channel.close();
}

#[test]
fn bulk_errors_and_short_writes_abort_without_false_progress() {
    for outcome in [Ok(0), Ok(4095), Ok(4097), Err(TransportError::Timeout)] {
        for failed_index in [0, 1] {
            let mut transport = RecordingTransport::default();
            transport
                .bulk_results
                .extend(std::iter::repeat_n(Ok(4096), failed_index));
            transport.bulk_results.push_back(outcome.clone());
            let calls = transport.calls.clone();
            let rails = SafetyRails::new()
                .with_hardware_writes_permitted(true)
                .with_active_transaction(true);
            let mut manager = TransactionManager::new(&mut transport, &rails)
                .with_delays(Duration::ZERO, Duration::ZERO);
            let packets = vec![BulkChunkPacket::default(); 3];
            let mut progress = Vec::new();
            let err = manager
                .stream_bulk_chunks(&packets, |sent, _| progress.push(sent))
                .unwrap_err();
            assert!(err
                .to_string()
                .contains(&format!("chunk {}/3", failed_index + 1)));
            assert_eq!(progress.len(), failed_index);
            assert_eq!(calls.lock().unwrap().len(), failed_index + 1);
        }
    }
}

#[test]
fn worker_propagates_bulk_failure_and_stops_remaining_chunks() {
    for outcome in [Ok(4095), Err(TransportError::Timeout)] {
        let mut transport = RecordingTransport::default();
        transport.bulk_results.push_back(outcome);
        let calls = transport.calls.clone();
        let channel = HardwareChannel::spawn(
            transport,
            Arc::new(
                SafetyRails::new()
                    .with_hardware_writes_permitted(true)
                    .with_active_transaction(true),
            ),
        );
        assert!(channel
            .stream_bulk(vec![BulkChunkPacket::default(); 3])
            .is_err());
        assert_eq!(calls.lock().unwrap().len(), 1);
        channel.close();
    }
}

#[test]
fn feature_transport_failure_is_propagated_without_retry() {
    let mut transport = RecordingTransport {
        feature_error: Some(TransportError::Timeout),
        ..RecordingTransport::default()
    };
    let calls = transport.calls.clone();
    let rails = Arc::new(
        SafetyRails::new()
            .with_hardware_writes_permitted(true)
            .with_active_transaction(true),
    );
    let packet = FeatureReportPacket::new(0xF5);
    {
        let mut manager = TransactionManager::new(&mut transport, &rails);
        assert!(manager
            .send_feature_command(&packet, WriteMode::RamPreview)
            .is_err());
    }
    assert_eq!(calls.lock().unwrap().len(), 1);
    calls.lock().unwrap().clear();
    let channel = HardwareChannel::spawn(transport, rails);
    assert!(channel.send_feature(packet, WriteMode::RamPreview).is_err());
    assert_eq!(calls.lock().unwrap().len(), 1);
    channel.close();
}
