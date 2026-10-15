use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use base64::prelude::*;
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, ImageBuffer, ImageReader, Rgb};
use monkey_core::lcd::{convert_image_to_frame, LCD_FRAME_BYTES, LCD_HEIGHT, LCD_WIDTH};

use crate::dto::AssetPreview;
use crate::error::{DesktopError, DesktopResult};

pub const MAX_FILE_BYTES: usize = 32 * 1024 * 1024; // 32 MiB
pub const MAX_PIXELS: u64 = 40_000_000; // 40M pixels
pub const MAX_FRAMES: usize = 300;
pub const MAX_RGB565_BYTES: usize = 64 * 1024 * 1024; // 64 MiB

#[derive(Debug, Clone)]
pub struct ConvertedAsset {
    pub preview: AssetPreview,
    pub frames: Vec<[u8; LCD_FRAME_BYTES]>,
    pub delays_ms: Vec<u64>,
}

pub struct AssetStore {
    assets: Mutex<HashMap<String, ConvertedAsset>>,
    next_id: AtomicUsize,
}

impl Default for AssetStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetStore {
    pub fn new() -> Self {
        Self {
            assets: Mutex::new(HashMap::new()),
            next_id: AtomicUsize::new(1),
        }
    }

    pub fn insert(&self, asset: ConvertedAsset) -> String {
        let id_num = self.next_id.fetch_add(1, Ordering::Relaxed);
        let id = format!("asset-{id_num}");
        let mut store = self.assets.lock().unwrap();
        let mut asset = asset;
        asset.preview.id = id.clone();
        store.insert(id.clone(), asset);
        id
    }

    pub fn get(&self, id: &str) -> Option<ConvertedAsset> {
        let store = self.assets.lock().unwrap();
        store.get(id).cloned()
    }

    pub fn remove(&self, id: &str) -> Option<ConvertedAsset> {
        let mut store = self.assets.lock().unwrap();
        store.remove(id)
    }

    pub fn clear(&self) {
        let mut store = self.assets.lock().unwrap();
        store.clear();
    }
}

pub fn convert_image_bytes_to_asset(name: &str, bytes: &[u8]) -> DesktopResult<ConvertedAsset> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(DesktopError::InvalidInput(format!(
            "File size ({} bytes) exceeds maximum allowed size ({} bytes / 32 MiB)",
            bytes.len(),
            MAX_FILE_BYTES
        )));
    }

    // Check if GIF
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return convert_gif_bytes_to_asset(name, bytes);
    }

    let cursor = Cursor::new(bytes);
    let reader = ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(|e| {
            DesktopError::InvalidInput(format!("Failed to determine image format: {e}"))
        })?;

    let dynamic_img = reader
        .decode()
        .map_err(|e| DesktopError::InvalidInput(format!("Failed to decode image: {e}")))?;

    let (w, h) = (dynamic_img.width() as u64, dynamic_img.height() as u64);
    if w * h > MAX_PIXELS {
        return Err(DesktopError::InvalidInput(format!(
            "Image dimensions ({w}x{h} = {} pixels) exceed maximum allowed {MAX_PIXELS} pixels",
            w * h
        )));
    }

    let rgb_img = dynamic_img.to_rgb8();
    let frame = convert_image_to_frame(&rgb_img, true, true);
    let preview_url = rgb565_to_png_data_url(&frame)?;

    let preview = AssetPreview {
        id: "".to_string(), // will be assigned by store
        name: name.to_string(),
        width: LCD_WIDTH as u32,
        height: LCD_HEIGHT as u32,
        frame_count: 1,
        preview_data_url: preview_url,
    };

    Ok(ConvertedAsset {
        preview,
        frames: vec![frame],
        delays_ms: vec![0],
    })
}

pub fn convert_gif_bytes_to_asset(name: &str, bytes: &[u8]) -> DesktopResult<ConvertedAsset> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(DesktopError::InvalidInput(format!(
            "GIF size exceeds maximum allowed size ({MAX_FILE_BYTES} bytes)"
        )));
    }

    let decoder = GifDecoder::new(Cursor::new(bytes))
        .map_err(|e| DesktopError::InvalidInput(format!("Failed to parse GIF: {e}")))?;

    let frames_iter = decoder.into_frames();
    let mut frames = Vec::new();
    let mut delays_ms = Vec::new();

    for frame_res in frames_iter {
        let frame = frame_res
            .map_err(|e| DesktopError::InvalidInput(format!("Failed to decode GIF frame: {e}")))?;

        if frames.len() >= MAX_FRAMES {
            return Err(DesktopError::InvalidInput(format!(
                "GIF animation frame count exceeds maximum allowed {MAX_FRAMES}"
            )));
        }

        let total_rgb565 = (frames.len() + 1) * LCD_FRAME_BYTES;
        if total_rgb565 > MAX_RGB565_BYTES {
            return Err(DesktopError::InvalidInput(format!(
                "Decoded RGB565 memory size exceeds maximum {MAX_RGB565_BYTES} bytes"
            )));
        }

        let (numer_ms, denom) = frame.delay().numer_denom_ms();
        let delay_ms = if numer_ms == 0 || denom == 0 {
            100
        } else {
            (numer_ms as u64) / (denom as u64)
        };

        let rgba_img = frame.into_buffer();
        let rgb_img = image::DynamicImage::ImageRgba8(rgba_img).to_rgb8();
        let lcd_frame = convert_image_to_frame(&rgb_img, true, true);
        frames.push(lcd_frame);
        delays_ms.push(delay_ms.max(20));
    }

    if frames.is_empty() {
        return Err(DesktopError::InvalidInput(
            "GIF animation contains no decodable frames".to_string(),
        ));
    }

    let preview_url = rgb565_to_png_data_url(&frames[0])?;

    let preview = AssetPreview {
        id: "".to_string(),
        name: name.to_string(),
        width: LCD_WIDTH as u32,
        height: LCD_HEIGHT as u32,
        frame_count: frames.len(),
        preview_data_url: preview_url,
    };

    Ok(ConvertedAsset {
        preview,
        frames,
        delays_ms,
    })
}

pub fn process_image_sequence(dir: &Path) -> DesktopResult<ConvertedAsset> {
    if !dir.is_dir() {
        return Err(DesktopError::InvalidInput(format!(
            "Sequence path '{:?}' is not a directory",
            dir
        )));
    }

    let canonical_root = dir
        .canonicalize()
        .map_err(|e| DesktopError::IoError(format!("Failed to resolve sequence path: {e}")))?;

    let mut entries = Vec::new();
    let read_dir = fs::read_dir(&canonical_root)
        .map_err(|e| DesktopError::IoError(format!("Failed to read sequence directory: {e}")))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| DesktopError::IoError(e.to_string()))?;
        let path = entry.path();

        // Check symlinks for path traversal
        if path.is_symlink() {
            let target = fs::read_link(&path)
                .map_err(|e| DesktopError::IoError(format!("Failed to read symlink: {e}")))?;
            let canonical_target = if target.is_relative() {
                canonical_root.join(target).canonicalize()
            } else {
                target.canonicalize()
            }
            .map_err(|e| DesktopError::IoError(format!("Invalid symlink target: {e}")))?;

            if !canonical_target.starts_with(&canonical_root) {
                return Err(DesktopError::UnsafeOperation(format!(
                    "Symlink escape detected: {:?} points outside sequence root {:?}",
                    path, canonical_root
                )));
            }
        }

        if path.is_file() {
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "webp") {
                entries.push(path);
            }
        }
    }

    // Sort entries alphanumeric
    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    if entries.is_empty() {
        return Err(DesktopError::InvalidInput(
            "Sequence directory contains no supported image files".to_string(),
        ));
    }

    if entries.len() > MAX_FRAMES {
        return Err(DesktopError::InvalidInput(format!(
            "Image sequence contains {} files, exceeding maximum of {}",
            entries.len(),
            MAX_FRAMES
        )));
    }

    let mut frames = Vec::with_capacity(entries.len());
    let mut delays_ms = Vec::with_capacity(entries.len());

    for path in &entries {
        let bytes = fs::read(path).map_err(|e| {
            DesktopError::IoError(format!("Failed to read frame file {:?}: {e}", path))
        })?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(DesktopError::InvalidInput(format!(
                "Sequence frame {:?} exceeds maximum file size",
                path
            )));
        }
        let dynamic_img = image::load_from_memory(&bytes).map_err(|e| {
            DesktopError::InvalidInput(format!("Failed to decode sequence frame {:?}: {e}", path))
        })?;
        let rgb_img = dynamic_img.to_rgb8();
        let frame = convert_image_to_frame(&rgb_img, true, true);
        frames.push(frame);
        delays_ms.push(100); // default 10 FPS (100ms)
    }

    let preview_url = rgb565_to_png_data_url(&frames[0])?;
    let dir_name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("sequence")
        .to_string();

    let preview = AssetPreview {
        id: "".to_string(),
        name: dir_name,
        width: LCD_WIDTH as u32,
        height: LCD_HEIGHT as u32,
        frame_count: frames.len(),
        preview_data_url: preview_url,
    };

    Ok(ConvertedAsset {
        preview,
        frames,
        delays_ms,
    })
}

pub fn rgb565_to_png_data_url(frame: &[u8; LCD_FRAME_BYTES]) -> DesktopResult<String> {
    let mut rgb_img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::new(LCD_WIDTH as u32, LCD_HEIGHT as u32);

    for y in 0..LCD_HEIGHT {
        for x in 0..LCD_WIDTH {
            let idx = (y * LCD_WIDTH + x) * 2;
            let val = u16::from_le_bytes([frame[idx], frame[idx + 1]]);
            let r5 = ((val >> 11) & 0x1F) as u8;
            let g6 = ((val >> 5) & 0x3F) as u8;
            let b5 = (val & 0x1F) as u8;

            let r8 = ((r5 as u32 * 255) / 31) as u8;
            let g8 = ((g6 as u32 * 255) / 63) as u8;
            let b8 = ((b5 as u32 * 255) / 31) as u8;

            rgb_img.put_pixel(x as u32, y as u32, Rgb([r8, g8, b8]));
        }
    }

    let mut png_bytes = Cursor::new(Vec::new());
    rgb_img
        .write_to(&mut png_bytes, image::ImageFormat::Png)
        .map_err(|e| {
            DesktopError::Internal(format!("Failed to encode RGB565 preview as PNG: {e}"))
        })?;

    let base64_encoded = BASE64_STANDARD.encode(png_bytes.into_inner());
    Ok(format!("data:image/png;base64,{base64_encoded}"))
}
