use crate::error::{MonkeyError, Result};
use crate::protocol::types::{BulkChunkPacket, BULK_CHUNK_SIZE};

/// Default chunk size for bulk frame streaming (4096 bytes on-wire report).
pub const DEFAULT_CHUNK_SIZE: usize = 4096;

/// Plan representing an outbound bulk transfer broken into chunks.
#[derive(Debug, Clone)]
pub struct BulkTransferPlan<'a> {
    data: &'a [u8],
    chunk_size: usize,
    pub total_chunks: usize,
    pub total_bytes: usize,
}

impl<'a> BulkTransferPlan<'a> {
    pub fn new(data: &'a [u8], chunk_size: usize) -> Result<Self> {
        if chunk_size == 0 || chunk_size > BULK_CHUNK_SIZE {
            return Err(MonkeyError::InvalidParameter(format!(
                "Chunk size must be between 1 and {BULK_CHUNK_SIZE}"
            )));
        }
        let total_chunks = data.len().div_ceil(chunk_size);
        Ok(Self {
            data,
            chunk_size,
            total_chunks,
            total_bytes: data.len(),
        })
    }

    pub fn get_chunk(&self, index: usize) -> Option<BulkChunkRef<'a>> {
        if index >= self.data.len().div_ceil(self.chunk_size) {
            return None;
        }
        let start = index * self.chunk_size;
        let end = start.saturating_add(self.chunk_size).min(self.data.len());
        Some(BulkChunkRef {
            index,
            total_chunks: self.total_chunks,
            data: &self.data[start..end],
        })
    }
}

/// A borrowed slice reference for a single chunk in a transfer plan.
#[derive(Debug, Clone, Copy)]
pub struct BulkChunkRef<'a> {
    pub index: usize,
    pub total_chunks: usize,
    pub data: &'a [u8],
}

/// Slices raw data into chunks of uniform size (except possibly the final chunk).
pub fn slice_into_chunks(data: &[u8], chunk_size: usize) -> Vec<&[u8]> {
    if chunk_size == 0 || data.is_empty() {
        return Vec::new();
    }
    data.chunks(chunk_size).collect()
}

/// Reassembles an array of borrowed chunk slices back into a continuous owned byte vector.
pub fn reassemble_chunks(chunks: &[&[u8]]) -> Result<Vec<u8>> {
    if chunks.is_empty() {
        return Ok(Vec::new());
    }
    let expected_len = chunks[0].len();
    let mut total_len = 0;
    for (i, chunk) in chunks.iter().enumerate() {
        if chunk.len() > expected_len {
            return Err(MonkeyError::Protocol(format!(
                "Chunk size at index {} ({} bytes) exceeds base chunk size ({})",
                i,
                chunk.len(),
                expected_len
            )));
        }
        if i + 1 < chunks.len() && chunk.len() < expected_len {
            return Err(MonkeyError::Protocol(format!(
                "Inconsistent chunk size at non-terminal index {}: expected {}, got {}",
                i,
                expected_len,
                chunk.len()
            )));
        }
        total_len += chunk.len();
    }
    let mut out = Vec::with_capacity(total_len);
    for chunk in chunks {
        out.extend_from_slice(chunk);
    }
    Ok(out)
}

/// Host-side transfer metadata. Only `packet.data` is sent on the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkTransferChunk {
    pub index: u8,
    pub total_chunks: u8,
    pub payload_len: usize,
    pub packet: BulkChunkPacket,
}

/// Splits bytes into raw reports, zero-padding unused bytes in each report.
/// The sequence counter is host metadata, never an LCD wire header.
pub struct ChunkIterator<'a> {
    data: &'a [u8],
    chunk_index: u8,
    total_chunks: u8,
    chunk_payload_size: usize,
}

impl<'a> ChunkIterator<'a> {
    pub fn new(data: &'a [u8], chunk_payload_size: usize) -> Result<Self> {
        if chunk_payload_size == 0 || chunk_payload_size > BULK_CHUNK_SIZE {
            return Err(MonkeyError::Protocol(format!(
                "Invalid chunk payload size {}: must be between 1 and {}",
                chunk_payload_size, BULK_CHUNK_SIZE
            )));
        }
        let count = data.len().div_ceil(chunk_payload_size);
        let total_chunks = u8::try_from(count).map_err(|_| {
            MonkeyError::InvalidParameter(format!("Transfer needs {count} chunks; maximum is 255"))
        })?;
        Ok(Self {
            data,
            chunk_index: 0,
            total_chunks,
            chunk_payload_size,
        })
    }

    pub fn total_chunks(&self) -> u8 {
        self.total_chunks
    }
}

impl<'a> Iterator for ChunkIterator<'a> {
    type Item = BulkTransferChunk;

    fn next(&mut self) -> Option<Self::Item> {
        if self.chunk_index >= self.total_chunks {
            return None;
        }

        let start = self.chunk_index as usize * self.chunk_payload_size;
        let end = (start + self.chunk_payload_size).min(self.data.len());
        let slice = &self.data[start..end];
        let mut packet = BulkChunkPacket::default();
        packet.data[..slice.len()].copy_from_slice(slice);
        let chunk = BulkTransferChunk {
            index: self.chunk_index,
            total_chunks: self.total_chunks,
            payload_len: slice.len(),
            packet,
        };
        self.chunk_index += 1;
        Some(chunk)
    }
}

/// Slices raw LCD framebuffer bytes (32768 bytes = 128x128x2) into exactly 8 x 4096 raw chunks.
/// Note: Monka raw LCD protocol uses exact 4096-byte chunk boundaries.
pub fn slice_lcd_frame(frame_buffer: &[u8]) -> Result<Vec<BulkChunkPacket>> {
    const LCD_FRAME_SIZE: usize = 32768; // 128 * 128 * 2
    if frame_buffer.len() != LCD_FRAME_SIZE {
        return Err(MonkeyError::Protocol(format!(
            "Invalid LCD frame buffer size: expected {} bytes, got {}",
            LCD_FRAME_SIZE,
            frame_buffer.len()
        )));
    }

    Ok(ChunkIterator::new(frame_buffer, BULK_CHUNK_SIZE)?
        .map(|chunk| chunk.packet)
        .collect())
}

/// Calculates CRC-16 across all raw byte chunks in a sequence, providing a unified checksum
/// across the chunk boundaries.
#[must_use]
pub fn calculate_chunks_crc16(chunks: &[BulkChunkPacket]) -> u16 {
    let crc = crc::Crc::<u16>::new(&crc::CRC_16_MODBUS);
    let mut hasher = crc.digest();
    for chunk in chunks {
        hasher.update(&chunk.data);
    }
    hasher.finalize()
}

/// Verifies that a sequence of chunks matches an expected overall CRC-16 checksum.
pub fn verify_chunks_crc16(chunks: &[BulkChunkPacket], expected: u16) -> Result<()> {
    let calculated = calculate_chunks_crc16(chunks);
    if calculated == expected {
        Ok(())
    } else {
        Err(MonkeyError::ChecksumMismatch {
            expected,
            calculated,
        })
    }
}
