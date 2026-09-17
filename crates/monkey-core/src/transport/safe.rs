use crate::error::TransportError;
use crate::protocol::SafetyRails;
use crate::transport::Transport;

/// A guard façade wrapping a mutable [`Transport`] reference and [`SafetyRails`] per D-07.
///
/// Ensures all hardware write operations (`write_bulk`, `send_feature_report`)
/// pass through safety authorization checks (`validate_hardware_write_permitted`)
/// before reaching the underlying transport adapter.
pub struct SafeTransport<'a> {
    transport: &'a mut dyn Transport,
    safety: &'a SafetyRails,
}

impl<'a> SafeTransport<'a> {
    /// Creates a new `SafeTransport` guard façade.
    pub fn new(transport: &'a mut dyn Transport, safety: &'a SafetyRails) -> Self {
        Self { transport, safety }
    }

    /// Reborrows the underlying transport and safety references without losing ownership.
    ///
    /// This allows passing a temporary `SafeTransport` to sub-managers or sub-routines.
    pub fn reborrow<'b>(&'b mut self) -> SafeTransport<'b> {
        SafeTransport {
            transport: &mut *self.transport,
            safety: self.safety,
        }
    }

    /// Returns a reference to the underlying raw transport.
    #[inline]
    pub fn as_raw(&self) -> &dyn Transport {
        self.transport
    }

    /// Returns a mutable reference to the underlying raw transport.
    #[inline]
    pub fn as_raw_mut(&mut self) -> &mut dyn Transport {
        self.transport
    }

    /// Returns a reference to the bound [`SafetyRails`].
    #[inline]
    pub fn safety(&self) -> &SafetyRails {
        self.safety
    }

    /// Writes a bulk data buffer with safety write authorization verification.
    pub fn write_bulk(&mut self, report_id: u8, data: &[u8]) -> Result<usize, TransportError> {
        self.safety
            .validate_hardware_write_permitted()
            .map_err(|e| TransportError::ProtocolViolation(e.to_string()))?;

        self.transport.write_bulk(report_id, data)
    }

    /// Sends a 64-byte HID feature report with safety write authorization verification.
    pub fn send_feature_report(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.safety
            .validate_hardware_write_permitted()
            .map_err(|e| TransportError::ProtocolViolation(e.to_string()))?;

        self.transport.send_feature_report(data)
    }

    /// Gets a feature report transparently from the keyboard.
    pub fn get_feature_report(
        &mut self,
        report_id: u8,
        buf: &mut [u8],
    ) -> Result<usize, TransportError> {
        self.transport.get_feature_report(report_id, buf)
    }

    /// Reads an input report transparently from the keyboard.
    pub fn read_input_report(
        &mut self,
        buf: &mut [u8],
        timeout_ms: i32,
    ) -> Result<usize, TransportError> {
        self.transport.read_input_report(buf, timeout_ms)
    }
}
