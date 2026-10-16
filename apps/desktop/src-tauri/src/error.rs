pub use crate::dto::{DesktopErrorPayload, ErrorCode};
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum DesktopError {
    #[error("Device not connected: {0}")]
    NotConnected(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Operation unsupported: {0}")]
    Unsupported(String),

    #[error("Device is busy with another operation: {0}")]
    Busy(String),

    #[error("Unsafe operation rejected: {0}")]
    UnsafeOperation(String),

    #[error("Low battery ({0}%): persistence operations disabled below 20% on wireless")]
    LowBattery(u8),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Operation cancelled: {0}")]
    Cancelled(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl DesktopError {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::NotConnected(_) => ErrorCode::NotConnected,
            Self::PermissionDenied(_) => ErrorCode::PermissionDenied,
            Self::Unsupported(_) => ErrorCode::Unsupported,
            Self::Busy(_) => ErrorCode::Busy,
            Self::UnsafeOperation(_) => ErrorCode::UnsafeOperation,
            Self::LowBattery(_) => ErrorCode::LowBattery,
            Self::InvalidInput(_) => ErrorCode::InvalidInput,
            Self::IoError(_) => ErrorCode::IoError,
            Self::Cancelled(_) => ErrorCode::Cancelled,
            Self::Internal(_) => ErrorCode::Internal,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Busy(_) | Self::IoError(_))
    }

    pub fn to_payload(&self) -> DesktopErrorPayload {
        DesktopErrorPayload {
            code: self.code(),
            message: self.to_string(),
            retryable: self.is_retryable(),
        }
    }
}

pub type DesktopResult<T> = Result<T, DesktopError>;
