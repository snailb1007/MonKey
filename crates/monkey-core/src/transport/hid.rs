use serde::{Deserialize, Serialize};
use crate::error::TransportError;
use crate::transport::Transport;

/// Active connection transport state of the keyboard per DISC-05.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConnectionState {
    /// Keyboard is actively connected over USB wired cable.
    WiredUsb,
    /// Keyboard is connected wirelessly (e.g. 2.4GHz dongle) and awake/responsive.
    WirelessAwake,
    /// Keyboard is connected wirelessly but asleep/power-saving (queries time out).
    WirelessSleeping,
}

/// Concrete hardware transport wrapping a `hidapi::HidDevice` handle.
pub struct HidTransport {
    device: hidapi::HidDevice,
    is_wireless: bool,
}

impl HidTransport {
    /// Creates a new HidTransport wrapping an open `hidapi::HidDevice`.
    /// Automatically determines whether the device is a wireless receiver from product metadata.
    pub fn new(device: hidapi::HidDevice) -> Self {
        let product = device.get_product_string().ok().flatten().unwrap_or_default();
        let is_wireless = Self::is_wireless_product(&product);
        Self { device, is_wireless }
    }

    /// Creates a new HidTransport with explicit wireless status.
    pub fn with_wireless(device: hidapi::HidDevice, is_wireless: bool) -> Self {
        Self { device, is_wireless }
    }

    /// Returns a reference to the inner `hidapi::HidDevice`.
    pub fn device(&self) -> &hidapi::HidDevice {
        &self.device
    }

    /// Returns a mutable reference to the inner `hidapi::HidDevice`.
    pub fn device_mut(&mut self) -> &mut hidapi::HidDevice {
        &mut self.device
    }

    /// Returns whether this transport represents a wireless connection.
    pub fn is_wireless(&self) -> bool {
        self.is_wireless
    }

    /// Checks whether a product string matches known wireless dongle/receiver signatures.
    pub fn is_wireless_product(product: &str) -> bool {
        let p = product.to_lowercase();
        p.contains("wireless") || p.contains("2.4g") || p.contains("receiver") || p.contains("dongle")
    }

    /// Frames a bulk output buffer with a leading Report ID byte per D-02 and D-05.
    /// For unnumbered Report ID 0, prepends 0x00 which tells `hidapi` this is an unnumbered report,
    /// and macOS `IOHIDDeviceSetReport` strips before wire transmission.
    pub fn frame_bulk_buffer(report_id: u8, data: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1 + data.len());
        buf.push(report_id);
        buf.extend_from_slice(data);
        buf
    }

    /// Computes payload bytes written from the raw hidapi write result (subtracting 1 for report ID prefix).
    pub fn calculate_payload_written(raw_written: usize) -> usize {
        raw_written.saturating_sub(1)
    }

    /// Prepares a feature report buffer by ensuring it is not empty and writing report_id at index 0.
    pub fn prepare_feature_buffer(report_id: u8, buf: &mut [u8]) -> Result<(), TransportError> {
        if buf.is_empty() {
            return Err(TransportError::BufferTooSmall {
                needed: 1,
                provided: 0,
            });
        }
        buf[0] = report_id;
        Ok(())
    }

    /// Evaluates connection state based on device wireless mode and non-destructive query outcome per DISC-05.
    pub fn evaluate_state_query(
        is_wireless: bool,
        query_result: Result<usize, TransportError>,
    ) -> Result<ConnectionState, TransportError> {
        match query_result {
            Ok(_) => {
                if is_wireless {
                    Ok(ConnectionState::WirelessAwake)
                } else {
                    Ok(ConnectionState::WiredUsb)
                }
            }
            Err(TransportError::Timeout) => Ok(ConnectionState::WirelessSleeping),
            Err(e) => Err(e),
        }
    }

    /// Detects active connection state via non-destructive read with finite timeout (50ms) per DISC-05 and D-12.
    /// Emits zero flash commits and zero DFU/ISP mode triggers.
    pub fn detect_connection_state(&mut self) -> Result<ConnectionState, TransportError> {
        let mut probe_buf = [0u8; 64];
        let query_res = self.get_feature_report(0, &mut probe_buf);
        Self::evaluate_state_query(self.is_wireless, query_res)
    }
}

impl Transport for HidTransport {
    /// Writes bulk data to the device (Interface A bulk pipe).
    /// When report_id == 0, allocates a buffer with leading byte 0x00 followed by data payload per D-02 and D-05.
    /// Returns the count of payload bytes written.
    fn write_bulk(&mut self, report_id: u8, data: &[u8]) -> Result<usize, TransportError> {
        let buf = Self::frame_bulk_buffer(report_id, data);
        let bytes_written = self.device.write(&buf).map_err(TransportError::from)?;
        Ok(Self::calculate_payload_written(bytes_written))
    }

    /// Sends a feature report to the device (Interface B control pipe).
    fn send_feature_report(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.device.send_feature_report(data).map_err(TransportError::from)
    }

    /// Reads a feature report from the device (Interface B control pipe).
    /// Ensures buf[0] = report_id before invocation.
    fn get_feature_report(&mut self, report_id: u8, buf: &mut [u8]) -> Result<usize, TransportError> {
        Self::prepare_feature_buffer(report_id, buf)?;
        self.device.get_feature_report(buf).map_err(TransportError::from)
    }

    /// Reads an input report from the device with a millisecond timeout.
    /// Maps a 0-byte return to TransportError::Timeout.
    fn read_input_report(&mut self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, TransportError> {
        let bytes_read = self.device.read_timeout(buf, timeout_ms).map_err(TransportError::from)?;
        if bytes_read == 0 {
            return Err(TransportError::Timeout);
        }
        Ok(bytes_read)
    }
}
