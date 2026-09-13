use std::time::{Duration, Instant};

use image::{DynamicImage, Rgb, RgbImage};
use monkey_core::lcd::{
    clamp_safe_frame_delay, convert_image_to_frame, decode_gif_frames, generate_test_pattern,
    load_image, rgb565_to_bytes, rgb888_to_rgb565, FpsRegulator, FrameChunker, LcdPacingConfig,
    LcdStreamer, TestPatternType, LCD_CHUNK_COUNT, LCD_CHUNK_SIZE, LCD_FRAME_BYTES, MAX_GIF_FRAMES,
    MAX_IMAGE_DIMENSION, MAX_SAFE_FPS, MIN_SAFE_FRAME_DELAY,
};
use monkey_core::{MockTransport, TransportCall};

#[test]
fn rgb565_exact_bytes_and_endianness() {
    assert_eq!(rgb888_to_rgb565(255, 0, 0), 0xF800);
    assert_eq!(rgb888_to_rgb565(0, 255, 0), 0x07E0);
    assert_eq!(rgb888_to_rgb565(0, 0, 255), 0x001F);
    assert_eq!(rgb565_to_bytes(0xF800, true), [0x00, 0xF8]);
    assert_eq!(rgb565_to_bytes(0x07E0, true), [0xE0, 0x07]);
    assert_eq!(rgb565_to_bytes(0x001F, true), [0x1F, 0x00]);
    assert_eq!(rgb565_to_bytes(0xF800, false), [0xF8, 0x00]);
}

#[test]
fn conversion_and_dithering_produce_exact_frame_size() {
    let image = RgbImage::from_fn(128, 128, |x, y| {
        let v = ((x * 2 + y) & 0xff) as u8;
        Rgb([v, 255 - v, v / 2])
    });
    let plain = convert_image_to_frame(&image, false, true);
    let dithered = convert_image_to_frame(&image, true, true);
    assert_eq!(plain.len(), LCD_FRAME_BYTES);
    assert_eq!(dithered.len(), LCD_FRAME_BYTES);
    assert_ne!(
        plain, dithered,
        "dithering must diffuse non-quantized error"
    );
}

#[test]
fn frame_chunker_is_zero_copy_and_exactly_eight_chunks() {
    let frame = [0xA5u8; LCD_FRAME_BYTES];
    let chunker = FrameChunker::new(&frame).unwrap();
    let chunks: Vec<&[u8]> = chunker.collect();
    assert_eq!(chunks.len(), 8);
    assert!(chunks.iter().all(|chunk| chunk.len() == LCD_CHUNK_SIZE));
    assert_eq!(chunks[0].as_ptr(), frame.as_ptr());
    assert_eq!(chunks[7].as_ptr(), unsafe {
        frame.as_ptr().add(7 * LCD_CHUNK_SIZE)
    });
}

#[test]
fn streamer_calls_interface_a_eight_times() {
    let frame = [0u8; LCD_FRAME_BYTES];
    let mut transport = MockTransport::new();
    let config = LcdPacingConfig {
        inter_chunk_delay: Duration::ZERO,
        target_fps: 12,
    };
    let metrics = LcdStreamer::new(&mut transport, config)
        .unwrap()
        .send_frame(&frame)
        .unwrap();
    assert_eq!(metrics.chunks_sent, 8);
    assert_eq!(metrics.bytes_sent, LCD_FRAME_BYTES);
    let writes: Vec<_> = transport
        .calls()
        .iter()
        .filter_map(|call| match call {
            TransportCall::WriteBulk { report_id, data } => Some((*report_id, data.len())),
            _ => None,
        })
        .collect();
    assert_eq!(writes, vec![(0, 4096); 8]);
}

#[test]
fn test_lcd_pacing_zero_target_fps_falls_back_to_safe_period() {
    let config = LcdPacingConfig {
        target_fps: 0,
        ..Default::default()
    };
    // Must not panic, and must not degrade into an unpaced (zero-delay) loop.
    assert_eq!(config.frame_period(), MIN_SAFE_FRAME_DELAY);
    assert!(config.validate().is_err());
}

#[test]
fn pacing_and_regulator_hold_safe_timing() {
    let frame = [0u8; LCD_FRAME_BYTES];
    let mut transport = MockTransport::new();
    let started = Instant::now();
    LcdStreamer::new(
        &mut transport,
        LcdPacingConfig {
            inter_chunk_delay: Duration::from_millis(3),
            target_fps: 12,
        },
    )
    .unwrap()
    .send_frame(&frame)
    .unwrap();
    assert!(started.elapsed() >= Duration::from_millis(18));
    assert!(LcdPacingConfig::default().frame_period() >= Duration::from_millis(80));
    let mut regulator = FpsRegulator::new(12).unwrap();
    assert_eq!(regulator.wait_for_next_frame(), Duration::ZERO);
    assert_eq!(regulator.frame_period(), Duration::from_micros(83_333));
    assert!(FpsRegulator::new(9).is_err());
}

#[test]
fn diagnostic_patterns_have_expected_pixels() {
    let red = generate_test_pattern(TestPatternType::Red);
    assert_eq!(&red[..2], &[0x00, 0xF8]);
    let bars = generate_test_pattern(TestPatternType::RgbBars);
    assert_eq!(&bars[..2], &[0x00, 0xF8]);
    assert_eq!(&bars[84..86], &[0xE0, 0x07]);
    assert_eq!(&bars[170..172], &[0x1F, 0x00]);
    let geometry = generate_test_pattern(TestPatternType::Geometry);
    assert_eq!(&geometry[..2], &[0xFF, 0xFF]);
}

#[test]
fn image_preprocessing_and_gif_decode_resize_to_lcd() {
    let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(300, 100, Rgb([255, 0, 0])));
    let prepared = monkey_core::lcd::preprocess_image(image);
    assert_eq!(prepared.dimensions(), (128, 128));
    assert_eq!(
        convert_image_to_frame(&prepared, false, true).len(),
        LCD_FRAME_BYTES
    );

    let path = std::env::temp_dir().join(format!("monkey-phase3-{}.gif", std::process::id()));
    let file = std::fs::File::create(&path).unwrap();
    let mut encoder = image::codecs::gif::GifEncoder::new(file);
    let frame = image::Frame::from_parts(
        image::RgbaImage::from_pixel(16, 8, image::Rgba([0, 255, 0, 255])),
        0,
        0,
        image::Delay::from_numer_denom_ms(40, 1),
    );
    encoder.encode_frames(std::iter::once(frame)).unwrap();
    drop(encoder);
    let frames = decode_gif_frames(&path, false, true).unwrap();
    let _ = std::fs::remove_file(path);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].frame.len(), LCD_FRAME_BYTES);
    assert_eq!(frames[0].delay, Duration::from_millis(40));
}

fn scratch(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("monkey-lcd-{}-{name}", std::process::id()))
}

/// T-03-01: an image past the dimension cap must be refused before decode.
#[test]
fn oversize_image_is_rejected_before_decode() {
    let path = scratch("oversize.png");
    image::RgbImage::from_pixel(MAX_IMAGE_DIMENSION + 8, 4, image::Rgb([1, 2, 3]))
        .save(&path)
        .unwrap();
    let result = load_image(&path);
    let _ = std::fs::remove_file(&path);
    assert!(
        result.is_err(),
        "image wider than {MAX_IMAGE_DIMENSION} must be rejected"
    );
}

/// T-03-01: a GIF past the frame cap must be refused, with a message that says so.
#[test]
fn gif_frame_bomb_is_rejected_with_a_clear_message() {
    let path = scratch("bomb.gif");
    let file = std::fs::File::create(&path).unwrap();
    let mut encoder = image::codecs::gif::GifEncoder::new(file);
    encoder
        .encode_frames((0..MAX_GIF_FRAMES + 5).map(|i| {
            image::Frame::from_parts(
                image::RgbaImage::from_pixel(4, 4, image::Rgba([(i % 255) as u8, 0, 0, 255])),
                0,
                0,
                image::Delay::from_numer_denom_ms(10, 1),
            )
        }))
        .unwrap();
    drop(encoder);
    let result = decode_gif_frames(&path, false, true);
    let _ = std::fs::remove_file(&path);
    let err = result.expect_err("frame bomb must be rejected").to_string();
    assert!(
        err.contains("frame limit"),
        "error must name the frame limit, got: {err}"
    );
}

/// T-03-03: untrusted GIF delays can never drive the panel above the safe rate.
#[test]
fn untrusted_frame_delays_are_clamped_to_the_safe_band() {
    let clamped = clamp_safe_frame_delay(Duration::from_millis(1));
    let fps = 1.0 / clamped.as_secs_f64();
    assert!(
        fps <= MAX_SAFE_FPS as f64 + 0.01,
        "clamped rate {fps} exceeds {MAX_SAFE_FPS} FPS"
    );
    // A slower-than-safe delay must be preserved, not accelerated.
    assert_eq!(
        clamp_safe_frame_delay(Duration::from_millis(500)),
        Duration::from_millis(500)
    );
}

/// The frame geometry is derived, never restated as a literal.
#[test]
fn frame_geometry_constants_agree() {
    assert_eq!(LCD_CHUNK_COUNT * LCD_CHUNK_SIZE, LCD_FRAME_BYTES);
    assert_eq!(monkey_core::LCD_FRAME_BYTES, LCD_FRAME_BYTES);
    assert_eq!(monkey_core::TARGET_FPS_MAX, MAX_SAFE_FPS as f64);
}
