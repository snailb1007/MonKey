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
    decode_gif, decode_gif_frames, load_image, load_image_frame, preprocess_image,
    LcdAnimationFrame,
};
pub use streamer::{
    FpsRegulator, FrameRateRegulator, LcdPacingConfig, LcdStreamMetrics, LcdStreamer,
};

pub const LCD_WIDTH: usize = 128;
pub const LCD_HEIGHT: usize = 128;
pub const LCD_PIXELS: usize = LCD_WIDTH * LCD_HEIGHT;
pub const LCD_FRAME_BYTES: usize = LCD_PIXELS * 2;
pub const LCD_CHUNK_SIZE: usize = 4096;
pub const LCD_CHUNK_COUNT: usize = LCD_FRAME_BYTES / LCD_CHUNK_SIZE;
/// HID report ID used for the unnumbered Interface A OUT report.
pub const LCD_INTERFACE_A_REPORT_ID: u8 = 0;

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
                    let border = x == 0 || y == 0 || x == LCD_WIDTH - 1 || y == LCD_HEIGHT - 1;
                    let cross = x == LCD_WIDTH / 2 || y == LCD_HEIGHT / 2;
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
