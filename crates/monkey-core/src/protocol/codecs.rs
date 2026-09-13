use crate::error::{MonkeyError, Result};
use crate::protocol::types::{CommandId, VendorReportId};
use zerocopy::{BigEndian, FromBytes, Immutable, IntoBytes, KnownLayout, U16};

/// Size of standard Feature Report packet (64 bytes).
pub const FEATURE_REPORT_LEN: usize = 64;

/// Size of Vendor Bulk Data packet (4096 bytes).
pub const BULK_REPORT_LEN: usize = 4096;

/// Fixed header for standard 64-byte feature reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C, packed)]
pub struct FeatureHeader {
    pub report_id: u8,
    pub command_id: u8,
    pub payload_len: u8,
    pub reserved: u8,
}

impl FeatureHeader {
    pub fn new(report_id: VendorReportId, command: CommandId, payload_len: u8) -> Self {
        Self {
            report_id: report_id as u8,
            command_id: command as u8,
            payload_len,
            reserved: 0x00,
        }
    }
}

/// Standard 64-byte Feature Report packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C, packed)]
pub struct FeaturePacket {
    pub header: FeatureHeader,
    pub payload: [u8; 60],
}

impl FeaturePacket {
    pub fn new(report_id: VendorReportId, command: CommandId, data: &[u8]) -> Result<Self> {
        if data.len() > 60 {
            return Err(MonkeyError::Protocol(format!(
                "Payload length {} exceeds maximum 60 bytes for feature report",
                data.len()
            )));
        }
        let mut payload = [0u8; 60];
        payload[..data.len()].copy_from_slice(data);
        Ok(Self {
            header: FeatureHeader::new(report_id, command, data.len() as u8),
            payload,
        })
    }

    pub fn to_bytes(&self) -> [u8; FEATURE_REPORT_LEN] {
        let mut buf = [0u8; FEATURE_REPORT_LEN];
        buf.copy_from_slice(self.as_bytes());
        buf
    }

    pub fn from_bytes_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != FEATURE_REPORT_LEN {
            return Err(MonkeyError::Protocol(format!(
                "Invalid feature packet length: expected {}, got {}",
                FEATURE_REPORT_LEN,
                bytes.len()
            )));
        }
        Self::read_from_bytes(bytes)
            .map_err(|e| MonkeyError::Protocol(format!("Failed to parse feature packet: {:?}", e)))
    }
}

/// Header for 4096-byte bulk packet transfers (e.g. LCD frames or flash data).
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C, packed)]
pub struct BulkHeader {
    pub magic: U16<BigEndian>,
    pub chunk_index: u8,
    pub total_chunks: u8,
    pub payload_len: U16<BigEndian>,
    pub crc16: U16<BigEndian>,
}

impl BulkHeader {
    pub const MAGIC: u16 = 0x55AA;

    pub fn new(chunk_index: u8, total_chunks: u8, payload_len: u16, crc16: u16) -> Self {
        Self {
            magic: U16::new(Self::MAGIC),
            chunk_index,
            total_chunks,
            payload_len: U16::new(payload_len),
            crc16: U16::new(crc16),
        }
    }
}

/// A 4096-byte bulk packet container with header and payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkPacket {
    pub header: BulkHeader,
    pub payload: Vec<u8>,
}

impl BulkPacket {
    pub const HEADER_LEN: usize = 8;
    pub const MAX_PAYLOAD_LEN: usize = BULK_REPORT_LEN - Self::HEADER_LEN; // 4088

    pub fn new(chunk_index: u8, total_chunks: u8, data: &[u8], crc16: u16) -> Result<Self> {
        if data.len() > Self::MAX_PAYLOAD_LEN {
            return Err(MonkeyError::Protocol(format!(
                "Bulk payload length {} exceeds maximum {}",
                data.len(),
                Self::MAX_PAYLOAD_LEN
            )));
        }
        let header = BulkHeader::new(chunk_index, total_chunks, data.len() as u16, crc16);
        Ok(Self {
            header,
            payload: data.to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![0u8; BULK_REPORT_LEN];
        buf[..Self::HEADER_LEN].copy_from_slice(self.header.as_bytes());
        buf[Self::HEADER_LEN..Self::HEADER_LEN + self.payload.len()].copy_from_slice(&self.payload);
        buf
    }

    pub fn from_bytes_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != BULK_REPORT_LEN {
            return Err(MonkeyError::Protocol(format!(
                "Invalid bulk packet length: expected {}, got {}",
                BULK_REPORT_LEN,
                bytes.len()
            )));
        }
        let header = BulkHeader::read_from_bytes(&bytes[..Self::HEADER_LEN])
            .map_err(|e| MonkeyError::Protocol(format!("Failed to parse bulk header: {:?}", e)))?;
        if header.magic.get() != BulkHeader::MAGIC {
            return Err(MonkeyError::Protocol(format!(
                "Invalid bulk magic: expected 0x{:04X}, got 0x{:04X}",
                BulkHeader::MAGIC,
                header.magic.get()
            )));
        }
        let payload_len = header.payload_len.get() as usize;
        if payload_len > Self::MAX_PAYLOAD_LEN {
            return Err(MonkeyError::Protocol(format!(
                "Payload len {} exceeds max {}",
                payload_len,
                Self::MAX_PAYLOAD_LEN
            )));
        }
        let payload = bytes[Self::HEADER_LEN..Self::HEADER_LEN + payload_len].to_vec();
        Ok(Self { header, payload })
    }
}
