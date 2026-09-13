use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};
pub use zerocopy::byteorder::little_endian::{U16, U32};

use crate::error::TransportError;

/// Magic byte indicating a MonKey vendor command.
pub const FEATURE_REPORT_MAGIC: u8 = 0x04;

/// Protocol marker expected at payload offset 14-15 (0x0E..0x0F).
pub const FEATURE_REPORT_MARKER: [u8; 2] = [0xAA, 0x55];

/// Size in bytes of a standard feature report packet.
pub const FEATURE_REPORT_SIZE: usize = 64;

/// Size in bytes of a single bulk chunk packet (Interface A bulk pipe).
pub const BULK_CHUNK_SIZE: usize = 4096;

/// Fixed 64-byte vendor feature report layout per D-07, D-08, and D-10.
///
/// Transmitted over Interface B (`0x000C:0x0001` / `0xFFFF`) to issue
/// commands and control keyboard state without heap allocation.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct FeatureReportPacket {
    /// Magic byte, expected to be `0x04` ([`FEATURE_REPORT_MAGIC`]).
    pub magic: u8,
    /// Opcode command identifier (e.g. `0x18` start, `0x13` RGB, `0x02` save).
    pub command: u8,
    /// Command-specific arguments (6 bytes).
    pub args: [u8; 6],
    /// Chunk index counter or total chunks (offset 8).
    pub chunk_index: u8,
    /// Reserved zero-filled bytes (offsets 9..14).
    pub reserved: [u8; 5],
    /// Protocol synchronization marker `[0xAA, 0x55]` at offsets 14..16.
    pub marker: [u8; 2],
    /// Payload buffer (48 bytes, offsets 16..64).
    pub payload: [u8; 48],
}

impl Default for FeatureReportPacket {
    fn default() -> Self {
        Self {
            magic: FEATURE_REPORT_MAGIC,
            command: 0,
            args: [0; 6],
            chunk_index: 0,
            reserved: [0; 5],
            marker: FEATURE_REPORT_MARKER,
            payload: [0; 48],
        }
    }
}

impl FeatureReportPacket {
    /// Creates a new `FeatureReportPacket` with the specified command,
    /// initialized with valid magic (`0x04`) and marker (`0xAA, 0x55`).
    pub fn new(command: u8) -> Self {
        Self {
            command,
            ..Default::default()
        }
    }

    /// Validates the packet header magic and marker per D-09.
    pub fn validate_header(&self) -> Result<(), TransportError> {
        if self.magic != FEATURE_REPORT_MAGIC {
            return Err(TransportError::ProtocolViolation(format!(
                "Invalid magic byte: expected 0x{:02X}, got 0x{:02X}",
                FEATURE_REPORT_MAGIC, self.magic
            )));
        }

        if self.marker != FEATURE_REPORT_MARKER {
            return Err(TransportError::ProtocolViolation(format!(
                "Invalid marker: expected {:02X?}, got {:02X?}",
                FEATURE_REPORT_MARKER, self.marker
            )));
        }

        Ok(())
    }
}

/// Fixed 4096-byte bulk transfer chunk layout per D-07, D-08, and D-10.
///
/// Transmitted over Interface A (`0xFF68:0x0061`) for high-bandwidth
/// transfers such as 128x128 LCD display frames (8 chunks = 32,768 bytes).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BulkChunkPacket {
    /// 4096-byte bulk payload data.
    pub data: [u8; BULK_CHUNK_SIZE],
}

impl Default for BulkChunkPacket {
    fn default() -> Self {
        Self {
            data: [0; BULK_CHUNK_SIZE],
        }
    }
}

impl BulkChunkPacket {
    /// Creates a new `BulkChunkPacket` wrapping the provided 4096-byte data buffer.
    pub fn new(data: [u8; BULK_CHUNK_SIZE]) -> Self {
        Self { data }
    }
}

// Compile-time size verification assertions per D-10
const _: () = assert!(core::mem::size_of::<FeatureReportPacket>() == 64);
const _: () = assert!(core::mem::size_of::<BulkChunkPacket>() == 4096);
