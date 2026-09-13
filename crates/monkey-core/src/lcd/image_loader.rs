//! Image loading, aspect-preserving preprocessing, and GIF decoding.
//!
//! Every decode path here consumes untrusted files, so all entry points apply
//! [`decode_limits`] before any pixel buffer is allocated.

use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;
use image::{DynamicImage, ImageDecoder, Limits, RgbImage};

use crate::error::{MonkeyError, Result};

use super::dither::convert_image_to_frame;
use super::LCD_FRAME_BYTES;

/// Maximum decoded width or height accepted from an untrusted image (T-03-01).
pub const MAX_IMAGE_DIMENSION: u32 = 4096;
/// Maximum number of frames decoded from an untrusted animated GIF (T-03-01).
pub const MAX_GIF_FRAMES: usize = 500;
/// Maximum single allocation any decoder may request (T-03-01).
///
/// A frame at [`MAX_IMAGE_DIMENSION`] squared in RGBA needs 64 MiB, so this
/// leaves headroom for one frame plus the GIF compositing canvas.
pub const MAX_DECODE_ALLOC_BYTES: u64 = 128 * 1024 * 1024;

/// Decoder limits applied to every untrusted image and GIF decode.
pub fn decode_limits() -> Limits {
    let mut limits = Limits::no_limits();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOC_BYTES);
    limits
}

/// A preprocessed GIF frame ready for the raw LCD streamer.
#[derive(Debug, Clone)]
pub struct LcdAnimationFrame {
    pub frame: [u8; LCD_FRAME_BYTES],
    pub delay: Duration,
}

/// Center-crops to 1:1 and scales to 128x128.
pub fn preprocess_image(image: DynamicImage) -> RgbImage {
    let (width, height) = (image.width(), image.height());
    let side = width.min(height);
    if side == 0 {
        return RgbImage::new(128, 128);
    }
    let x = (width - side) / 2;
    let y = (height - side) / 2;
    let cropped = image.crop_imm(x, y, side, side);
    cropped
        .resize_exact(128, 128, image::imageops::FilterType::Lanczos3)
        .to_rgb8()
}

/// Loads any format enabled in the workspace image dependency.
pub fn load_image(path: impl AsRef<Path>) -> Result<RgbImage> {
    let path = path.as_ref();
    let mut reader = image::ImageReader::open(path)
        .map_err(|e| MonkeyError::Image(format!("{}: {e}", path.display())))?
        .with_guessed_format()
        .map_err(|e| MonkeyError::Image(format!("{}: {e}", path.display())))?;
    reader.limits(decode_limits());
    Ok(preprocess_image(reader.decode()?))
}

/// Loads and converts a static image to the exact 32 KiB LCD frame.
pub fn load_image_frame(
    path: impl AsRef<Path>,
    dither: bool,
    little_endian: bool,
) -> Result<[u8; LCD_FRAME_BYTES]> {
    let image = load_image(path)?;
    Ok(convert_image_to_frame(&image, dither, little_endian))
}

fn decode_gif_internal(
    path: impl AsRef<Path>,
    dither: bool,
    little_endian: bool,
) -> Result<Vec<LcdAnimationFrame>> {
    let path = path.as_ref();
    let file = std::fs::File::open(path)
        .map_err(|e| MonkeyError::Image(format!("{}: {e}", path.display())))?;
    let mut decoder = GifDecoder::new(BufReader::new(file))?;
    decoder.set_limits(decode_limits())?;

    // Frames are consumed lazily; `collect_frames` would materialize every
    // frame at source resolution before any of them is downscaled.
    let mut frames = Vec::new();
    for frame in decoder.into_frames() {
        if frames.len() >= MAX_GIF_FRAMES {
            return Err(MonkeyError::Image(format!(
                "GIF exceeds the {MAX_GIF_FRAMES} frame limit"
            )));
        }
        let frame = frame?;
        let delay = delay_duration(frame.delay());
        let prepared = preprocess_image(DynamicImage::ImageRgba8(frame.into_buffer()));
        frames.push(LcdAnimationFrame {
            frame: convert_image_to_frame(&prepared, dither, little_endian),
            delay,
        });
    }

    Ok(frames)
}

/// Decodes all GIF frames, preserving their GIF delay metadata.
pub fn decode_gif(path: impl AsRef<Path>) -> Result<Vec<LcdAnimationFrame>> {
    decode_gif_internal(path, false, true)
}

/// GIF decoding with conversion options for CLI playback.
pub fn decode_gif_frames(
    path: impl AsRef<Path>,
    dither: bool,
    little_endian: bool,
) -> Result<Vec<LcdAnimationFrame>> {
    decode_gif_internal(path, dither, little_endian)
}

fn delay_duration(delay: image::Delay) -> Duration {
    let (numerator_ms, denominator) = delay.numer_denom_ms();
    if numerator_ms == 0 || denominator == 0 {
        Duration::from_millis(100)
    } else {
        Duration::from_secs_f64(numerator_ms as f64 / denominator as f64 / 1000.0)
    }
}
