//! LCD rendering and raw Interface A streaming.

pub mod chunker;
pub mod color;
pub mod dither;
pub mod image_loader;
pub mod streamer;

pub use chunker::{lcd_chunks, FrameChunker};
pub use color::{
    rgb565_bytes, rgb565_to_bytes, rgb565_to_format_bytes, rgb888_to_rgb565, ColorFormat, BLACK,
    BLUE, GREEN, RED, WHITE,
};
pub use dither::{convert_image_to_frame, dither_rgb565};
pub use image_loader::{
    decode_gif, decode_gif_frames, decode_limits, load_image, load_image_frame, preprocess_image,
    LcdAnimationFrame, MAX_DECODE_ALLOC_BYTES, MAX_GIF_FRAMES, MAX_IMAGE_DIMENSION,
};
pub use streamer::{
    clamp_safe_frame_delay, validate_safe_fps, FpsRegulator, FrameRateRegulator, LcdPacingConfig,
    LcdStreamMetrics, LcdStreamer, MAX_SAFE_FPS, MIN_SAFE_FPS, MIN_SAFE_FRAME_DELAY,
};

pub const LCD_WIDTH: usize = 128;
pub const LCD_HEIGHT: usize = 128;
pub const LCD_PIXELS: usize = LCD_WIDTH * LCD_HEIGHT;
pub const LCD_FRAME_BYTES: usize = LCD_PIXELS * 2;
pub const LCD_CHUNK_SIZE: usize = 4096;
pub const LCD_CHUNK_COUNT: usize = LCD_FRAME_BYTES / LCD_CHUNK_SIZE;
/// HID report ID used for the unnumbered Interface A OUT report.
pub const LCD_INTERFACE_A_REPORT_ID: u8 = 0;

/// Native OEM TFT resolution (`160x96` RGB565, `30,720` bytes per frame) verified from `DeviceDriver.exe` (`0x4237a0`).
pub const OEM_LCD_WIDTH: usize = 160;
pub const OEM_LCD_HEIGHT: usize = 96;
pub const OEM_LCD_PIXELS: usize = OEM_LCD_WIDTH * OEM_LCD_HEIGHT;
pub const OEM_LCD_FRAME_BYTES: usize = OEM_LCD_PIXELS * 2; // 30,720 bytes (0x7800)
pub const OEM_LCD_HEADER_BYTES: usize = 256; // 0x100-byte header preceding frames

/// Resamples a `128x128` RGB565 Little-Endian frame into the native `160x96` (`30,720` bytes)
/// Little-Endian RGB565 layout expected by the Monka 3075 Pro / RKGK890 MCU.
#[must_use]
pub fn resample_128_to_oem_160x96(frame: &[u8; LCD_FRAME_BYTES]) -> [u8; OEM_LCD_FRAME_BYTES] {
    let mut out = [0u8; OEM_LCD_FRAME_BYTES];
    for y in 0..OEM_LCD_HEIGHT {
        let src_y = (y * LCD_HEIGHT) / OEM_LCD_HEIGHT;
        for x in 0..OEM_LCD_WIDTH {
            let src_x = (x * LCD_WIDTH) / OEM_LCD_WIDTH;
            let src_idx = (src_y * LCD_WIDTH + src_x) * 2;
            let dst_idx = (y * OEM_LCD_WIDTH + x) * 2;
            out[dst_idx] = frame[src_idx];
            out[dst_idx + 1] = frame[src_idx + 1];
        }
    }
    out
}

/// Built-in diagnostic patterns for byte order, channels, and geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestPatternType {
    Red,
    Green,
    Blue,
    White,
    Black,
    Grayscale,
    RgbBars,
    Geometry,
}

/// Generates a complete 128x128 RGB565 diagnostic frame in the default wire order.
pub fn generate_test_pattern(pattern: TestPatternType) -> [u8; LCD_FRAME_BYTES] {
    generate_test_pattern_with_format(pattern, ColorFormat::LittleEndian)
}

/// Generates a diagnostic frame with explicit wire byte order.
pub fn generate_test_pattern_with_format(
    pattern: TestPatternType,
    format: ColorFormat,
) -> [u8; LCD_FRAME_BYTES] {
    let mut frame = [0u8; LCD_FRAME_BYTES];
    for y in 0..LCD_HEIGHT {
        for x in 0..LCD_WIDTH {
            let (r, g, b) = match pattern {
                TestPatternType::Red => (255, 0, 0),
                TestPatternType::Green => (0, 255, 0),
                TestPatternType::Blue => (0, 0, 255),
                TestPatternType::White => (255, 255, 255),
                TestPatternType::Black => (0, 0, 0),
                TestPatternType::Grayscale => {
                    let v = (x * 255 / (LCD_WIDTH - 1)) as u8;
                    (v, v, v)
                }
                TestPatternType::RgbBars => {
                    if x < LCD_WIDTH / 3 {
                        (255, 0, 0)
                    } else if x < (LCD_WIDTH * 2) / 3 {
                        (0, 255, 0)
                    } else {
                        (0, 0, 255)
                    }
                }
                TestPatternType::Geometry => {
                    let border = x < 2 || y < 2 || x >= LCD_WIDTH - 2 || y >= LCD_HEIGHT - 2;
                    let cross = (LCD_WIDTH / 2 - 1..=LCD_WIDTH / 2).contains(&x)
                        || (LCD_HEIGHT / 2 - 1..=LCD_HEIGHT / 2).contains(&y);
                    if border {
                        (255, 255, 255)
                    } else if cross {
                        (255, 0, 0)
                    } else {
                        (0, 0, 0)
                    }
                }
            };
            let bytes = rgb565_to_format_bytes(rgb888_to_rgb565(r, g, b), format);
            let idx = (y * LCD_WIDTH + x) * 2;
            frame[idx..idx + 2].copy_from_slice(&bytes);
        }
    }
    frame
}
