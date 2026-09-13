use monkey_core::protocol::{
    BulkPacket, CommandId, FeaturePacket, HardwareChannel, SafetyRails, TransactionManager,
    VendorReportId, WriteMode,
};
use monkey_core::transport::MockTransport;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_safety_rails_whitelist() {
    let safety = SafetyRails::new();

    // Whitelisted commands
    assert!(safety.validate_command(CommandId::RgbControl).is_ok());
    assert!(safety.validate_command(CommandId::LcdStartTransfer).is_ok());
    assert!(safety.validate_command(CommandId::LcdChunkAck).is_ok());
    assert!(safety.validate_command(CommandId::LcdEndTransfer).is_ok());
    assert!(safety.validate_command(CommandId::GetVersion).is_ok());

    // Non-whitelisted command should fail
    assert!(safety.validate_command(CommandId::Reboot).is_err());
    assert_eq!(safety.blocked_count(), 1);
}

#[test]
fn test_safety_rails_flash_debouncing() {
    let safety = SafetyRails::with_flash_debounce(Duration::from_millis(100));

    // RAM preview write is always allowed
    assert!(safety.check_write_allowed(WriteMode::RamPreview).is_ok());
    assert!(safety.check_write_allowed(WriteMode::RamPreview).is_ok());

    // First Flash commit is allowed
    assert!(safety.check_write_allowed(WriteMode::FlashCommit).is_ok());

    // Immediate second Flash commit should be throttled
    assert!(safety.check_write_allowed(WriteMode::FlashCommit).is_err());

    // After debounce duration, it should succeed
    std::thread::sleep(Duration::from_millis(110));
    assert!(safety.check_write_allowed(WriteMode::FlashCommit).is_ok());
}

#[test]
fn test_transaction_manager_bulk_streaming() {
    let mut mock = MockTransport::new();
    let safety = SafetyRails::new();
    let mut manager = TransactionManager::new(&mut mock, &safety)
        .with_delays(Duration::from_millis(0), Duration::from_millis(1));

    let chunks = vec![
        BulkPacket::new(0, 2, &[1u8; 10], 0x1234).unwrap(),
        BulkPacket::new(1, 2, &[2u8; 10], 0x5678).unwrap(),
    ];

    let mut sent_indices = Vec::new();
    let duration = manager
        .stream_bulk_chunks(&chunks, |sent, total| {
            sent_indices.push((sent, total));
        })
        .expect("bulk streaming should succeed");

    assert_eq!(sent_indices, vec![(1, 2), (2, 2)]);
    assert!(duration >= Duration::from_millis(1));
    assert_eq!(mock.calls().len(), 2);
}

#[test]
fn test_hardware_channel_worker_thread() {
    let mock = MockTransport::new();
    let safety = Arc::new(SafetyRails::new());
    let channel = HardwareChannel::spawn(mock, safety);

    let chunks = vec![BulkPacket::new(0, 1, &[42u8; 16], 0xAAAA).unwrap()];

    let duration = channel
        .stream_bulk(chunks)
        .expect("channel bulk transfer should succeed");
    assert!(duration >= Duration::ZERO);

    let feature =
        FeaturePacket::new(VendorReportId::Feature, CommandId::GetVersion, &[0u8; 4]).unwrap();
    let feat_res = channel.send_feature(CommandId::GetVersion, feature, WriteMode::RamPreview);
    assert!(feat_res.is_ok());

    channel.close();
}
