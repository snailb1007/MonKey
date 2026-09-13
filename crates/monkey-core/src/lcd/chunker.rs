//! Exact zero-copy framing for raw 128x128 RGB565 LCD frames.

use crate::error::{MonkeyError, Result};

use super::{LCD_CHUNK_COUNT, LCD_CHUNK_SIZE, LCD_FRAME_BYTES};

/// Borrowed iterator over the eight contiguous LCD OUT chunks.
#[derive(Debug, Clone, Copy)]
pub struct FrameChunker<'a> {
    frame: &'a [u8],
    next: usize,
}

impl<'a> FrameChunker<'a> {
    /// Creates a chunker. The frame must be exactly 32 KiB.
    pub fn new(frame: &'a [u8]) -> Result<Self> {
        if frame.len() != LCD_FRAME_BYTES {
            return Err(MonkeyError::Protocol(format!(
                "Invalid LCD frame size: expected {LCD_FRAME_BYTES} bytes, got {}",
                frame.len()
            )));
        }
        Ok(Self { frame, next: 0 })
    }

    pub const fn chunk_count(&self) -> usize {
        LCD_CHUNK_COUNT
    }

    pub fn chunk(&self, index: usize) -> Option<&'a [u8]> {
        if index >= LCD_CHUNK_COUNT {
            return None;
        }
        let start = index * LCD_CHUNK_SIZE;
        Some(&self.frame[start..start + LCD_CHUNK_SIZE])
    }
}

impl<'a> Iterator for FrameChunker<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk(self.next)?;
        self.next += 1;
        Some(chunk)
    }
}

/// Returns an exact borrowed chunk for a validated LCD frame.
pub fn lcd_chunks(frame: &[u8]) -> Result<FrameChunker<'_>> {
    FrameChunker::new(frame)
}
