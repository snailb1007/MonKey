use thiserror::Error;

/// Structured transport error variants for MonKey hardware communication.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("HID error: {0}")]
    HidError(String),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Operation timed out")]
    Timeout,

    #[error("Buffer too small: needed {needed} bytes, provided {provided} bytes")]
    BufferTooSmall { needed: usize, provided: usize },

    #[error("Invalid report ID: {0}")]
    InvalidReportId(u8),

    #[error("Device disconnected")]
    Disconnected,

    #[error("Protocol violation: {0}")]
    ProtocolViolation(String),
}

impl From<hidapi::HidError> for TransportError {
    fn from(err: hidapi::HidError) -> Self {
        TransportError::HidError(err.to_string())
    }
}

/// Comprehensive domain-specific errors for MonKey keyboard operations.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum MonkeyError {
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Hardware safety rail violation: {0}")]
    SafetyViolation(String),

    #[error("Flash transaction error: {0}")]
    FlashTransaction(String),

    #[error("Checksum mismatch: expected 0x{expected:04X}, calculated 0x{calculated:04X}")]
    ChecksumMismatch { expected: u16, calculated: u16 },

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Image error: {0}")]
    Image(String),
}

impl From<image::ImageError> for MonkeyError {
    fn from(err: image::ImageError) -> Self {
        MonkeyError::Image(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, MonkeyError>;
