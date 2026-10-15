use std::fs;
use tempfile::tempdir;

use crate::asset::{convert_image_bytes_to_asset, process_image_sequence};
use crate::dto::*;
use crate::error::DesktopError;
use crate::worker::{HardwareWorkerHandle, MockHardwareBackend};
use monkey_core::transport::MockTransport;

#[test]
fn test_asset_conversion_single_image_and_rgb565_preview() {
    // 2x2 white PNG
    let png_bytes = make_test_png(2, 2, 255, 255, 255);
    let asset = convert_image_bytes_to_asset("test.png", &png_bytes)
        .expect("Image conversion should succeed");

    assert_eq!(asset.preview.width, 128);
    assert_eq!(asset.preview.height, 128);
    assert_eq!(asset.preview.frame_count, 1);
    assert!(asset
        .preview
        .preview_data_url
        .starts_with("data:image/png;base64,"));
    assert_eq!(asset.frames.len(), 1);
    assert_eq!(asset.frames[0].len(), monkey_core::lcd::LCD_FRAME_BYTES);
}

#[test]
fn test_asset_rejects_oversized_file() {
    let large_bytes = vec![0u8; 33 * 1024 * 1024]; // 33 MiB > 32 MiB
    let res = convert_image_bytes_to_asset("huge.png", &large_bytes);
    match res {
        Err(DesktopError::InvalidInput(msg)) => {
            assert!(msg.contains("exceeds maximum allowed size"));
        }
        other => panic!("Expected InvalidInput for oversized file, got: {other:?}"),
    }
}

#[test]
fn test_asset_rejects_corrupted_image() {
    let corrupt_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x00, 0x01, 0x02];
    let res = convert_image_bytes_to_asset("corrupt.png", &corrupt_bytes);
    match res {
        Err(DesktopError::InvalidInput(msg)) => {
            assert!(msg.contains("Failed to decode image"));
        }
        other => panic!("Expected InvalidInput for corrupted image, got: {other:?}"),
    }
}

#[test]
fn test_sequence_traversal_rejects_symlink_escape() {
    let dir = tempdir().expect("tempdir");
    let outside_dir = tempdir().expect("outside tempdir");

    // Write an image in the outside dir
    let outside_file = outside_dir.path().join("secret.png");
    fs::write(&outside_file, make_test_png(2, 2, 0, 0, 0)).unwrap();

    // Create a symlink pointing outside inside the sequence dir
    let symlink_path = dir.path().join("link_escape.png");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside_file, &symlink_path).unwrap();
        let res = process_image_sequence(dir.path());
        match res {
            Err(DesktopError::UnsafeOperation(msg)) => {
                assert!(msg.contains("Symlink escape detected"));
            }
            Err(DesktopError::InvalidInput(msg)) => {
                assert!(msg.contains("Symlink escape") || msg.contains("outside"));
            }
            other => panic!("Expected rejection of symlink escape, got: {other:?}"),
        }
    }
}

#[test]
fn test_lcd_pattern_streaming_executes_chunks_over_mock() {
    let mock = MockTransport::new();
    let (backend, transport_ref) = MockHardwareBackend::with_mock_transport(mock);
    let worker = HardwareWorkerHandle::spawn_mock(backend);

    let devices = worker.list_devices().unwrap();
    let dev_id = &devices[0].id;

    // Generate pattern frame
    let frame = monkey_core::lcd::generate_test_pattern(monkey_core::lcd::TestPatternType::RgbBars);

    let job_id = worker
        .start_lcd(dev_id, vec![frame], 0)
        .expect("start_lcd should return job_id");

    // Poll status until completed
    let mut status = worker.job_status(&job_id).unwrap();
    let start = std::time::Instant::now();
    while status.state == JobState::Running && start.elapsed().as_secs() < 3 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        status = worker.job_status(&job_id).unwrap();
    }

    assert_eq!(status.state, JobState::Completed);
    assert_eq!(status.frames_sent, 1);

    // Verify 8 chunks of 4096 bytes were written via write_bulk
    let transport = transport_ref.lock().unwrap();
    let history = transport.calls();
    let bulk_writes: Vec<_> = history
        .iter()
        .filter(|call| {
            matches!(
                call,
                monkey_core::transport::TransportCall::WriteBulk { .. }
            )
        })
        .collect();
    assert_eq!(
        bulk_writes.len(),
        8,
        "One LCD frame must write exactly 8 chunks of 4096 bytes"
    );

    worker.shutdown();
}

#[test]
fn test_lcd_streaming_cancellation() {
    let mock = MockTransport::new();
    let (backend, _) = MockHardwareBackend::with_mock_transport(mock);
    let worker = HardwareWorkerHandle::spawn_mock(backend);

    let devices = worker.list_devices().unwrap();
    let dev_id = &devices[0].id;

    // Send 10 frames with 50ms pacing so we can stop mid-flight
    let frame = monkey_core::lcd::generate_test_pattern(monkey_core::lcd::TestPatternType::Blue);
    let frames = vec![frame; 10];

    let job_id = worker.start_lcd(dev_id, frames, 50).unwrap();

    // Immediately stop job
    std::thread::sleep(std::time::Duration::from_millis(15));
    let stopped = worker.stop_lcd(&job_id).expect("stop_lcd must succeed");
    assert!(stopped);

    // Verify status transitions to Cancelled
    let mut status = worker.job_status(&job_id).unwrap();
    let start = std::time::Instant::now();
    while status.state == JobState::Running && start.elapsed().as_secs() < 3 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        status = worker.job_status(&job_id).unwrap();
    }

    assert_eq!(status.state, JobState::Cancelled);
    assert!(
        status.frames_sent < 10,
        "Job must cancel before completing all frames"
    );

    worker.shutdown();
}

#[test]
fn test_lcd_streaming_rejected_when_interface_a_missing_with_zero_writes() {
    let mock = MockTransport::new();
    let (mut backend, transport_ref) = MockHardwareBackend::with_mock_transport(mock);
    // Explicitly configure mock device to lack Interface A
    backend.has_interface_a = false;
    let worker = HardwareWorkerHandle::spawn_mock(backend);

    let devices = worker.list_devices().unwrap();
    let dev_id = &devices[0].id;

    let frame = monkey_core::lcd::generate_test_pattern(monkey_core::lcd::TestPatternType::Red);
    let res = worker.start_lcd(dev_id, vec![frame], 0);

    match res {
        Err(DesktopError::Unsupported(msg)) => {
            assert!(msg.contains("Interface A"));
        }
        other => panic!("Expected Unsupported error for missing Interface A, got: {other:?}"),
    }

    // Prove ZERO I/O writes occurred on rejection
    let transport = transport_ref.lock().unwrap();
    transport
        .assert_no_writes()
        .expect("Zero writes must occur when InterfacePolicy rejects streaming");

    worker.shutdown();
}

fn make_test_png(w: u32, h: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
    use image::{ImageBuffer, Rgb};
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(w, h, Rgb([r, g, b]));
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}
