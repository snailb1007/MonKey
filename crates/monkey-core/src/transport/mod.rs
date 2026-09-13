pub mod hid;
pub mod mock;

use crate::error::TransportError;
use crate::protocol::SafetyRails;
pub use hid::{ConnectionState, HidTransport};
pub use mock::{MockTransport, TransportCall};

/// Synchronous hardware transport abstraction per D-04.
/// Direct writes to hardware require passing a reference to `SafetyRails` to ensure
/// that raw transport writes cannot bypass the safety gate pipeline.
pub trait Transport: Send {
    /// Writes bulk data to the device (Interface A bulk pipe) with mandatory safety checks.
    fn write_bulk(
        &mut self,
        report_id: u8,
        data: &[u8],
        safety: &SafetyRails,
    ) -> Result<usize, TransportError>;

    /// Sends a feature report to the device (Interface B control pipe) with mandatory safety checks.
    fn send_feature_report(
        &mut self,
        data: &[u8],
        safety: &SafetyRails,
    ) -> Result<(), TransportError>;

    /// Reads a feature report from the device (Interface B control pipe).
    ///
    /// # Buffer Layout Convention
    /// Following the `hidapi` driver specification, `buf` must include space for the leading
    /// Report ID byte at `buf[0]`.
    /// - For unnumbered Report ID 0, `buf` must be at least 65 bytes (`1 + 64`), where `buf[0]` is 0
    ///   and payload is written into `buf[1..=64]`. Total returned byte count is 65.
    /// - For numbered Report ID > 0, `buf[0]` contains the report ID and payload is written into `buf[1..]`.
    fn get_feature_report(
        &mut self,
        report_id: u8,
        buf: &mut [u8],
    ) -> Result<usize, TransportError>;

    /// Reads an input report from the device with a millisecond timeout.
    fn read_input_report(
        &mut self,
        buf: &mut [u8],
        timeout_ms: i32,
    ) -> Result<usize, TransportError>;
}
