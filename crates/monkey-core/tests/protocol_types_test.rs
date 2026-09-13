use monkey_core::error::TransportError;
use monkey_core::protocol::{
    BulkChunkPacket, FeatureReportPacket, BULK_CHUNK_SIZE, FEATURE_REPORT_MAGIC,
    FEATURE_REPORT_MARKER, FEATURE_REPORT_SIZE, U16, U32,
};
use zerocopy::{FromBytes, IntoBytes};

#[test]
fn test_feature_report_packet_size_and_layout() {
    assert_eq!(core::mem::size_of::<FeatureReportPacket>(), 64);
    assert_eq!(FEATURE_REPORT_SIZE, 64);

    let default_packet = FeatureReportPacket::default();
    assert_eq!(default_packet.magic, FEATURE_REPORT_MAGIC);
    assert_eq!(default_packet.command, 0x00);
    assert_eq!(default_packet.marker, FEATURE_REPORT_MARKER);
    assert_eq!(default_packet.args, [0u8; 6]);
    assert_eq!(default_packet.chunk_index, 0);
    assert_eq!(default_packet.reserved, [0u8; 5]);
    assert_eq!(default_packet.payload, [0u8; 48]);
}

#[test]
fn test_bulk_chunk_packet_size_and_layout() {
    assert_eq!(core::mem::size_of::<BulkChunkPacket>(), 4096);
    assert_eq!(BULK_CHUNK_SIZE, 4096);

    let packet = BulkChunkPacket::default();
    assert_eq!(packet.data.len(), 4096);
    assert_eq!(packet.data[0], 0);
    assert_eq!(packet.data[4095], 0);
}

#[test]
fn test_feature_report_zero_copy_round_trip() {
    let mut raw_bytes = [0u8; 64];
    raw_bytes[0] = 0x04; // magic
    raw_bytes[1] = 0x13; // command (RGB apply)
    raw_bytes[2] = 0x01; // arg0: mode
    raw_bytes[3] = 0xFF; // arg1: R
    raw_bytes[4] = 0x00; // arg2: G
    raw_bytes[5] = 0x7F; // arg3: B
    raw_bytes[8] = 0x01; // chunk_index
    raw_bytes[14] = 0xAA; // marker 0
    raw_bytes[15] = 0x55; // marker 1
    raw_bytes[16] = 0x42; // payload byte 0
    raw_bytes[63] = 0x99; // payload byte 47

    // Zero-allocation transmute from byte slice
    let packet = FeatureReportPacket::ref_from_bytes(&raw_bytes).expect("zero-copy parsing");
    assert_eq!(packet.magic, 0x04);
    assert_eq!(packet.command, 0x13);
    assert_eq!(packet.args[0], 0x01);
    assert_eq!(packet.args[1], 0xFF);
    assert_eq!(packet.args[2], 0x00);
    assert_eq!(packet.args[3], 0x7F);
    assert_eq!(packet.chunk_index, 0x01);
    assert_eq!(packet.marker, [0xAA, 0x55]);
    assert_eq!(packet.payload[0], 0x42);
    assert_eq!(packet.payload[47], 0x99);

    // Header validation succeeds
    assert!(packet.validate_header().is_ok());

    // Round-trip into bytes
    let serialized = packet.as_bytes();
    assert_eq!(serialized.len(), 64);
    assert_eq!(serialized, &raw_bytes);
}

#[test]
fn test_feature_report_header_validation_failures() {
    // Test invalid magic byte
    let mut invalid_magic = FeatureReportPacket::new(0x18);
    invalid_magic.magic = 0x05; // Should be 0x04
    match invalid_magic.validate_header() {
        Err(TransportError::ProtocolViolation(msg)) => {
            assert!(msg.contains("Invalid magic byte"));
        }
        other => panic!("Expected ProtocolViolation for magic byte, got: {:?}", other),
    }

    // Test invalid marker bytes
    let mut invalid_marker = FeatureReportPacket::new(0x18);
    invalid_marker.marker = [0x00, 0x00]; // Should be [0xAA, 0x55]
    match invalid_marker.validate_header() {
        Err(TransportError::ProtocolViolation(msg)) => {
            assert!(msg.contains("Invalid marker"));
        }
        other => panic!("Expected ProtocolViolation for marker, got: {:?}", other),
    }
}

#[test]
fn test_bulk_chunk_zero_copy_round_trip() {
    let mut raw_data = [0u8; 4096];
    raw_data[0] = 0xDE;
    raw_data[1] = 0xAD;
    raw_data[2048] = 0xBE;
    raw_data[4095] = 0xEF;

    // Zero-allocation transmute from byte slice
    let packet = BulkChunkPacket::ref_from_bytes(&raw_data).expect("zero-copy bulk parsing");
    assert_eq!(packet.data[0], 0xDE);
    assert_eq!(packet.data[1], 0xAD);
    assert_eq!(packet.data[2048], 0xBE);
    assert_eq!(packet.data[4095], 0xEF);

    // Round-trip into bytes
    let bytes = packet.as_bytes();
    assert_eq!(bytes.len(), 4096);
    assert_eq!(bytes, &raw_data);
}

#[test]
fn test_little_endian_primitives_safety() {
    // Verify explicit little-endian ordering
    let val_16: U16 = U16::new(0x1234);
    assert_eq!(val_16.as_bytes(), &[0x34, 0x12]);
    assert_eq!(val_16.get(), 0x1234);

    let val_32: U32 = U32::new(0x12345678);
    assert_eq!(val_32.as_bytes(), &[0x78, 0x56, 0x34, 0x12]);
    assert_eq!(val_32.get(), 0x12345678);
}
