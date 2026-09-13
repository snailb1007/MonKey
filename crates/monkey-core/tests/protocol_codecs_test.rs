use monkey_core::protocol::{
    crc::{calculate_crc16, verify_crc16},
    framing::{
        calculate_chunks_crc16, reassemble_chunks, slice_into_chunks, slice_lcd_frame,
        verify_chunks_crc16, BulkTransferPlan, ChunkIterator,
    },
    FeatureReportPacket, BULK_CHUNK_SIZE,
};

#[test]
fn crc_remains_an_independent_utility() {
    assert_eq!(calculate_crc16(b"123456789"), 0x4B37);
    assert!(verify_crc16(b"123456789", 0x4B37));
    assert!(!verify_crc16(b"123456789", 0x1234));
}

// Independently assembled from the existing layout and phase research;
// this is a layout fixture, not a captured Monka response.
fn feature_fixture() -> [u8; 64] {
    let mut bytes = [0; 64];
    bytes[..9].copy_from_slice(&[0x04, 0x13, 1, 2, 3, 4, 5, 6, 8]);
    bytes[14..19].copy_from_slice(&[0xAA, 0x55, 0x12, 0x34, 0x56]);
    bytes
}

#[test]
fn feature_layout_matches_independent_fixture() {
    let expected = feature_fixture();
    let mut packet = FeatureReportPacket::with_payload(0x13, &[0x12, 0x34, 0x56]).unwrap();
    packet.args = [1, 2, 3, 4, 5, 6];
    packet.chunk_index = 8;
    assert_eq!(packet.as_bytes(), &expected);
    let decoded = FeatureReportPacket::parse_from_slice(&expected).unwrap();
    assert_eq!(decoded, &packet);
    assert_eq!(decoded.as_bytes(), expected);
}

#[test]
fn feature_parser_rejects_bad_lengths_magic_and_markers() {
    let fixture = feature_fixture();
    for len in [0, 1, 63, 65, 128] {
        assert!(FeatureReportPacket::parse_from_slice(&vec![0; len]).is_err());
    }
    let mut oversized = fixture.to_vec();
    oversized.push(0);
    assert!(FeatureReportPacket::parse_from_slice(&oversized).is_err());
    for offset in [0, 14, 15] {
        let mut corrupt = fixture;
        corrupt[offset] ^= 0xFF;
        assert!(FeatureReportPacket::parse_from_slice(&corrupt).is_err());
    }
}

#[test]
fn feature_constructor_enforces_payload_boundaries() {
    for len in [0, 1, 48] {
        let packet = FeatureReportPacket::with_payload(0xF5, &vec![0x42; len]).unwrap();
        assert_eq!(&packet.payload[..len], &vec![0x42; len]);
        assert!(packet.payload[len..].iter().all(|b| *b == 0));
        packet.validate_header().unwrap();
    }
    assert!(FeatureReportPacket::with_payload(0x13, &[0; 49]).is_err());
}

#[test]
fn lcd_frame_is_eight_unchanged_raw_reports() {
    let data: Vec<u8> = (0..32768).map(|i| (i % 251) as u8).collect();
    let chunks = slice_lcd_frame(&data).unwrap();
    assert_eq!(chunks.len(), 8);
    let reassembled: Vec<_> = chunks.iter().flat_map(|chunk| chunk.data).collect();
    assert_eq!(reassembled, data);
    for len in [0, 32767, 32769] {
        assert!(slice_lcd_frame(&vec![0; len]).is_err());
    }

    // CRC-16 across chunk boundaries
    let crc = calculate_chunks_crc16(&chunks);
    assert_eq!(crc, calculate_chunks_crc16(&chunks));
    assert!(verify_chunks_crc16(&chunks, crc).is_ok());
    assert!(verify_chunks_crc16(&chunks, crc ^ 0xFFFF).is_err());
}

#[test]
fn transfer_plan_rejects_bad_sizes_and_reassembles() {
    for size in [0, 4097, usize::MAX] {
        assert!(BulkTransferPlan::new(&[], size).is_err());
        assert!(BulkTransferPlan::new(&[1], size).is_err());
    }
    let data = vec![7; 4097];
    let plan = BulkTransferPlan::new(&data, BULK_CHUNK_SIZE).unwrap();
    assert_eq!(plan.total_chunks, 2);
    assert_eq!(plan.total_bytes, 4097);
    let first = plan.get_chunk(0).unwrap();
    let last = plan.get_chunk(1).unwrap();
    assert_eq!((first.index, first.total_chunks), (0, 2));
    assert_eq!(last.data.len(), 1);
    assert!(plan.get_chunk(2).is_none());
    assert!(plan.get_chunk(usize::MAX).is_none());
    assert_eq!(reassemble_chunks(&[first.data, last.data]).unwrap(), data);
    assert_eq!(slice_into_chunks(&data, 4096), vec![first.data, last.data]);
    let empty = BulkTransferPlan::new(&[], 4096).unwrap();
    assert_eq!(empty.total_chunks, 0);
    assert!(empty.get_chunk(0).is_none());
    assert!(reassemble_chunks(&[&[1, 2], &[3], &[4, 5]]).is_err());
    assert!(reassemble_chunks(&[&[1], &[2, 3]]).is_err());
}

#[test]
fn iterator_checks_count_before_narrowing_and_terminates_at_255() {
    for size in [0, 4097, usize::MAX] {
        assert!(ChunkIterator::new(&[], size).is_err());
        assert!(ChunkIterator::new(&[1], size).is_err());
    }
    for size in [1, 128, 4096] {
        assert_eq!(ChunkIterator::new(&[], size).unwrap().count(), 0);
        let data = vec![1; size * 255];
        let mut iter = ChunkIterator::new(&data, size).unwrap();
        assert_eq!(iter.total_chunks(), 255);
        for index in 0..255 {
            let chunk = iter.next().unwrap();
            assert_eq!(chunk.index, index);
            assert_eq!(chunk.total_chunks, 255);
            assert_eq!(chunk.payload_len, size);
        }
        assert!(iter.next().is_none());
        assert!(iter.next().is_none());
        assert!(ChunkIterator::new(&vec![0; size * 255 + 1], size).is_err());
        assert!(ChunkIterator::new(&vec![0; size * 256], size).is_err());
    }
    let err = ChunkIterator::new(&[0; 32768], 128).err().unwrap();
    assert!(err.to_string().contains("256"));
}

#[test]
fn iterator_metadata_is_host_only_and_final_payload_is_padded() {
    let data = vec![0x7E; 4097];
    let chunks: Vec<_> = ChunkIterator::new(&data, 4096).unwrap().collect();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].packet.data, [0x7E; 4096]);
    assert_eq!(chunks[1].payload_len, 1);
    assert_eq!(chunks[1].packet.data[0], 0x7E);
    assert!(chunks[1].packet.data[1..].iter().all(|b| *b == 0));
    let small = ChunkIterator::new(&[1, 2, 3], 2)
        .unwrap()
        .collect::<Vec<_>>();
    assert_eq!(&small[0].packet.data[..3], &[1, 2, 0]);
    assert_eq!(&small[1].packet.data[..3], &[3, 0, 0]);
}
