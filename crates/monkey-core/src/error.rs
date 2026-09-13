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

