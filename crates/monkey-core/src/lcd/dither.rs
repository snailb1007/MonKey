//! Serpentine Floyd-Steinberg error diffusion for 128x128 RGB frames.

use image::RgbImage;

use super::color::{rgb565_to_format_bytes, rgb888_to_rgb565, ColorFormat};
use super::{LCD_FRAME_BYTES, LCD_HEIGHT, LCD_PIXELS, LCD_WIDTH};

/// Converts an RGB image to the fixed-size LCD framebuffer.
///
/// A non-128x128 image is resized here as a convenience; file loading uses the
/// higher-quality center-crop path in [`crate::lcd::image_loader`].
pub fn convert_image_to_frame(
    image: &RgbImage,
    dither: bool,
    little_endian: bool,
) -> [u8; LCD_FRAME_BYTES] {
    let format = if little_endian {
        ColorFormat::LittleEndian
    } else {
        ColorFormat::BigEndian
    };
    let source = if image.width() == LCD_WIDTH as u32 && image.height() == LCD_HEIGHT as u32 {
        None
    } else {
        Some(image::imageops::resize(
            image,
            LCD_WIDTH as u32,
            LCD_HEIGHT as u32,
            image::imageops::FilterType::Triangle,
        ))
    };
    let pixels = source.as_ref().unwrap_or(image);
    if dither {
        dither_rgb565(pixels, format)
    } else {
        let mut out = [0u8; LCD_FRAME_BYTES];
        for (idx, pixel) in pixels.pixels().enumerate().take(LCD_PIXELS) {
            let value = rgb888_to_rgb565(pixel[0], pixel[1], pixel[2]);
            let bytes = rgb565_to_format_bytes(value, format);
            out[idx * 2..idx * 2 + 2].copy_from_slice(&bytes);
        }
        out
    }
}

/// Applies serpentine Floyd-Steinberg dithering and writes RGB565 bytes.
pub fn dither_rgb565(image: &RgbImage, format: ColorFormat) -> [u8; LCD_FRAME_BYTES] {
    debug_assert_eq!(image.width(), LCD_WIDTH as u32);
    debug_assert_eq!(image.height(), LCD_HEIGHT as u32);

    // Three float error planes make the implementation clear and avoid the
    // clipping artifacts caused by integer error truncation.
    let mut work = [[0.0f32; 3]; LCD_PIXELS];
    for (idx, pixel) in image.pixels().enumerate().take(LCD_PIXELS) {
        work[idx] = [pixel[0] as f32, pixel[1] as f32, pixel[2] as f32];
    }

    let mut out = [0u8; LCD_FRAME_BYTES];
    for y in 0..LCD_HEIGHT {
        let reverse = y % 2 == 1;
        for step in 0..LCD_WIDTH {
            let x = if reverse { LCD_WIDTH - 1 - step } else { step };
            let idx = y * LCD_WIDTH + x;
            let old = work[idx];
            let quantized = [
                quantize(old[0], 31.0),
                quantize(old[1], 63.0),
                quantize(old[2], 31.0),
            ];
            let pixel = rgb888_to_rgb565(quantized[0], quantized[1], quantized[2]);
            let bytes = rgb565_to_format_bytes(pixel, format);
            out[idx * 2..idx * 2 + 2].copy_from_slice(&bytes);

            let error = [
                old[0] - quantized[0] as f32,
                old[1] - quantized[1] as f32,
                old[2] - quantized[2] as f32,
            ];
            let right = if reverse {
                x.checked_sub(1)
            } else {
                x.checked_add(1)
            };
            let down_left = if reverse {
                x.checked_add(1)
            } else {
                x.checked_sub(1)
            };
            if let Some(nx) = right.filter(|&v| v < LCD_WIDTH) {
                add_error(&mut work[y * LCD_WIDTH + nx], error, 7.0 / 16.0);
            }
            if y + 1 < LCD_HEIGHT {
                if let Some(nx) = down_left.filter(|&v| v < LCD_WIDTH) {
                    add_error(&mut work[(y + 1) * LCD_WIDTH + nx], error, 3.0 / 16.0);
                }
                add_error(&mut work[(y + 1) * LCD_WIDTH + x], error, 5.0 / 16.0);
                if let Some(nx) = right.filter(|&v| v < LCD_WIDTH) {
                    add_error(&mut work[(y + 1) * LCD_WIDTH + nx], error, 1.0 / 16.0);
                }
            }
        }
    }
    out
}

fn quantize(value: f32, levels: f32) -> u8 {
    let clamped = value.clamp(0.0, 255.0);
    ((clamped / 255.0 * levels).round() * 255.0 / levels).round() as u8
}

fn add_error(pixel: &mut [f32; 3], error: [f32; 3], weight: f32) {
    for channel in 0..3 {
        pixel[channel] = (pixel[channel] + error[channel] * weight).clamp(-255.0, 510.0);
    }
}
