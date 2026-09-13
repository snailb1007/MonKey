use monkey_core::error::MonkeyError;
use monkey_core::protocol::{
    codecs::{BulkHeader, BulkPacket, FeaturePacket},
    crc::{calculate_crc16, verify_crc16},
    framing::{reassemble_chunks, slice_into_chunks, slice_lcd_frame, BulkTransferPlan},
    types::{CommandId, VendorReportId, BULK_CHUNK_SIZE},
};
use zerocopy::IntoBytes;

#[test]
fn test_crc16_known_vectors() {
    let data = b"123456789";
    let crc = calculate_crc16(data);
    assert_eq!(crc, 0x4B37);
    assert!(verify_crc16(data, crc));
    assert!(!verify_crc16(data, crc ^ 0xFFFF));
}

#[test]
fn test_feature_report_header_codec() {
    let packet = FeaturePacket::new(
        VendorReportId::Feature,
        CommandId::RgbControl,
        &[0x01, 0x02, 0x03],
    )
    .expect("create packet");
    let bytes = packet.to_bytes();
    assert_eq!(bytes.len(), 64);
    assert_eq!(bytes[0], 0x04);
    assert_eq!(bytes[1], 0x13);
    assert_eq!(bytes[2], 0x03);

    let parsed = FeaturePacket::from_bytes_slice(&bytes).expect("Should parse");
    assert_eq!(parsed.header.report_id, 0x04);
    assert_eq!(parsed.header.command_id, 0x13);
    assert_eq!(parsed.header.payload_len, 0x03);
    assert_eq!(&parsed.payload[..3], &[0x01, 0x02, 0x03]);
}

#[test]
fn test_bulk_data_header_codec() {
    let header = BulkHeader::new(0, 8, 4088, 0x1234);
    let bytes = header.as_bytes();
    assert_eq!(bytes.len(), 8);
    assert_eq!(header.chunk_index, 0);
    assert_eq!(header.total_chunks, 8);
    assert_eq!(header.payload_len.get(), 4088);
    assert_eq!(header.crc16.get(), 0x1234);

    let packet = BulkPacket::new(0, 8, &[0xAA; 100], 0x1234).expect("valid bulk packet");
    let wire_bytes = packet.to_bytes();
    assert_eq!(wire_bytes.len(), 4096);

    let parsed = BulkPacket::from_bytes_slice(&wire_bytes).expect("parse wire bytes");
    assert_eq!(parsed.header.chunk_index, 0);
    assert_eq!(parsed.header.total_chunks, 8);
    assert_eq!(parsed.header.payload_len.get(), 100);
    assert_eq!(parsed.payload, vec![0xAA; 100]);
}

#[test]
fn test_framing_slice_and_reassemble() {
    let total_len = 32768;
    let data: Vec<u8> = (0..total_len).map(|i| (i % 256) as u8).collect();
    let chunks = slice_into_chunks(&data, BULK_CHUNK_SIZE);
    assert_eq!(chunks.len(), 8);

    for chunk in &chunks {
        assert_eq!(chunk.len(), BULK_CHUNK_SIZE);
    }

    let plan = BulkTransferPlan::new(&data, BULK_CHUNK_SIZE);
    assert_eq!(plan.total_chunks, 8);
    assert_eq!(plan.total_bytes, 32768);

    let mut chunk_refs = Vec::new();
    for i in 0..plan.total_chunks {
        let chunk = plan.get_chunk(i).expect("chunk exists");
        chunk_refs.push(chunk.data);
    }

    let reassembled = reassemble_chunks(&chunk_refs).expect("reassembly ok");
    assert_eq!(reassembled, data);

    let lcd_chunks = slice_lcd_frame(&data).expect("slice lcd frame");
    assert_eq!(lcd_chunks.len(), 8);
    for chunk in lcd_chunks {
        assert_eq!(chunk.len(), 4096);
    }
}

#[test]
fn test_framing_error_on_mismatched_size() {
    let c1 = vec![0u8; 100];
    let c2 = vec![0u8; 50];
    let c3 = vec![0u8; 100];
    let err = reassemble_chunks(&[&c1, &c2, &c3]).unwrap_err();

    match err {
        MonkeyError::Protocol(msg) => assert!(msg.contains("Inconsistent chunk size")),
        _ => panic!("Unexpected error variant"),
    }
}
