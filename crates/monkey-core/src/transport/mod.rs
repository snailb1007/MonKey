pub mod mock;

use crate::error::TransportError;
pub use mock::{MockTransport, TransportCall};

/// Synchronous hardware transport abstraction per D-04.
pub trait Transport: Send {
    /// Writes bulk data to the device (Interface A bulk pipe).
    fn write_bulk(&mut self, report_id: u8, data: &[u8]) -> Result<usize, TransportError>;

    /// Sends a feature report to the device (Interface B control pipe).
    fn send_feature_report(&mut self, data: &[u8]) -> Result<(), TransportError>;

    /// Reads a feature report from the device (Interface B control pipe).
    fn get_feature_report(&mut self, report_id: u8, buf: &mut [u8]) -> Result<usize, TransportError>;

    /// Reads an input report from the device with a millisecond timeout.
    fn read_input_report(&mut self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, TransportError>;
}
