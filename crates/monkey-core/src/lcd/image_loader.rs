//! Image loading, aspect-preserving preprocessing, and GIF decoding.

use std::path::Path;
use std::time::Duration;

use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;
use image::{DynamicImage, ImageError, RgbImage};

use super::dither::convert_image_to_frame;
use super::LCD_FRAME_BYTES;

/// A preprocessed GIF frame ready for the raw LCD streamer.
#[derive(Debug, Clone)]
pub struct LcdAnimationFrame {
    pub frame: [u8; LCD_FRAME_BYTES],
    pub delay: Duration,
}

/// Center-crops an image to a square, then resizes it to 128x128.
pub fn preprocess_image(image: DynamicImage) -> RgbImage {
    let rgb = image.into_rgb8();
    let side = rgb.width().min(rgb.height());
    if side == 0 {
        return RgbImage::new(128, 128);
    }
    let left = (rgb.width() - side) / 2;
    let top = (rgb.height() - side) / 2;
    let cropped = image::imageops::crop_imm(&rgb, left, top, side, side).to_image();
    image::imageops::resize(&cropped, 128, 128, image::imageops::FilterType::Lanczos3)
}

/// Loads any format enabled in the workspace image dependency.
pub fn load_image(path: impl AsRef<Path>) -> Result<RgbImage, ImageError> {
    image::open(path).map(preprocess_image)
}

/// Loads and converts a static image to the exact 32 KiB LCD frame.
pub fn load_image_frame(
    path: impl AsRef<Path>,
    dither: bool,
    little_endian: bool,
) -> Result<[u8; LCD_FRAME_BYTES], ImageError> {
    let image = load_image(path)?;
    Ok(convert_image_to_frame(&image, dither, little_endian))
}

/// Decodes all GIF frames, preserving their GIF delay metadata.
pub fn decode_gif(path: impl AsRef<Path>) -> Result<Vec<LcdAnimationFrame>, ImageError> {
    let file = std::fs::File::open(path)?;
    let decoder = GifDecoder::new(std::io::BufReader::new(file))?;
    decoder.into_frames().collect_frames().map(|frames| {
        frames
            .into_iter()
            .map(|frame| {
                let delay = delay_duration(frame.delay());
                let prepared = preprocess_image(DynamicImage::ImageRgba8(frame.into_buffer()));
                LcdAnimationFrame {
                    frame: convert_image_to_frame(&prepared, false, true),
                    delay,
                }
            })
            .collect()
    })
}

/// GIF decoding with conversion options for CLI playback.
pub fn decode_gif_frames(
    path: impl AsRef<Path>,
    dither: bool,
    little_endian: bool,
) -> Result<Vec<LcdAnimationFrame>, ImageError> {
    let file = std::fs::File::open(path)?;
    let decoder = GifDecoder::new(std::io::BufReader::new(file))?;
    decoder.into_frames().collect_frames().map(|frames| {
        frames
            .into_iter()
            .map(|frame| {
                let delay = delay_duration(frame.delay());
                let prepared = preprocess_image(DynamicImage::ImageRgba8(frame.into_buffer()));
                LcdAnimationFrame {
                    frame: convert_image_to_frame(&prepared, dither, little_endian),
                    delay,
                }
            })
            .collect()
    })
}

fn delay_duration(delay: image::Delay) -> Duration {
    let (numerator_ms, denominator) = delay.numer_denom_ms();
    if numerator_ms == 0 || denominator == 0 {
        Duration::from_millis(100)
    } else {
        Duration::from_secs_f64(numerator_ms as f64 / denominator as f64 / 1000.0)
    }
}
