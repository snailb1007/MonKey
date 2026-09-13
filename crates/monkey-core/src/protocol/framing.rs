use crate::error::{MonkeyError, Result};
use crate::protocol::codecs::BulkPacket;
use crate::protocol::crc::calculate_crc16;

/// Default chunk size for bulk frame streaming (4096 bytes on-wire report).
pub const DEFAULT_CHUNK_SIZE: usize = 4096;

/// Plan representing an outbound bulk transfer broken into chunks.
#[derive(Debug, Clone)]
pub struct BulkTransferPlan<'a> {
    pub data: &'a [u8],
    pub chunk_size: usize,
    pub total_chunks: usize,
    pub total_bytes: usize,
}

impl<'a> BulkTransferPlan<'a> {
    pub fn new(data: &'a [u8], chunk_size: usize) -> Self {
        let total_chunks = if data.is_empty() {
            0
        } else {
            data.len().div_ceil(chunk_size)
        };
        Self {
            data,
            chunk_size,
            total_chunks,
            total_bytes: data.len(),
        }
    }

    pub fn get_chunk(&self, index: usize) -> Option<BulkChunkRef<'a>> {
        if index >= self.total_chunks {
            return None;
        }
        let start = index * self.chunk_size;
        let end = (start + self.chunk_size).min(self.total_bytes);
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

/// Chunking helper that splits an arbitrary payload into sequence-numbered `BulkPacket` instances.
pub struct ChunkIterator<'a> {
    data: &'a [u8],
    chunk_index: u8,
    total_chunks: u8,
    chunk_payload_size: usize,
}

impl<'a> ChunkIterator<'a> {
    pub fn new(data: &'a [u8], chunk_payload_size: usize) -> Result<Self> {
        if chunk_payload_size == 0 || chunk_payload_size > BulkPacket::MAX_PAYLOAD_LEN {
            return Err(MonkeyError::Protocol(format!(
                "Invalid chunk payload size {}: must be between 1 and {}",
                chunk_payload_size,
                BulkPacket::MAX_PAYLOAD_LEN
            )));
        }
        let total_chunks = if data.is_empty() {
            0
        } else {
            (data.len().div_ceil(chunk_payload_size)) as u8
        };
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
    type Item = Result<BulkPacket>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.chunk_index >= self.total_chunks {
            return None;
        }

        let start = self.chunk_index as usize * self.chunk_payload_size;
        let end = (start + self.chunk_payload_size).min(self.data.len());
        let slice = &self.data[start..end];
        let crc = calculate_crc16(slice);

        let packet = BulkPacket::new(self.chunk_index, self.total_chunks, slice, crc);
        self.chunk_index += 1;
        Some(packet)
    }
}
