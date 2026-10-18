use crate::error::TransportError;
use crate::protocol::types::FEATURE_REPORT_MAGIC;
use crate::transport::Transport;
use serde::{Deserialize, Serialize};

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
    #[cfg(target_os = "macos")]
    usb_control: Option<crate::transport::macos_usb::MacosUsbControlPipe>,
}

impl HidTransport {
    /// Creates a new HidTransport wrapping an open `hidapi::HidDevice`.
    /// Automatically determines whether the device is a wireless receiver or Bluetooth device.
    pub fn new(device: hidapi::HidDevice) -> Self {
        let is_bt_bus = device
            .get_device_info()
            .ok()
            .map(|info| matches!(info.bus_type(), hidapi::BusType::Bluetooth))
            .unwrap_or(false);
        let product = device
            .get_product_string()
            .ok()
            .flatten()
            .unwrap_or_default();
        let is_wireless = is_bt_bus || Self::is_wireless_product(&product);
        #[cfg(target_os = "macos")]
        let usb_control = if !is_wireless {
            crate::transport::macos_usb::MacosUsbControlPipe::open().ok()
        } else {
            None
        };
        Self {
            device,
            is_wireless,
            #[cfg(target_os = "macos")]
            usb_control,
        }
    }

    /// Creates a new HidTransport with explicit wireless status.
    pub fn with_wireless(device: hidapi::HidDevice, is_wireless: bool) -> Self {
        #[cfg(target_os = "macos")]
        let usb_control = if !is_wireless {
            crate::transport::macos_usb::MacosUsbControlPipe::open().ok()
        } else {
            None
        };
        Self {
            device,
            is_wireless,
            #[cfg(target_os = "macos")]
            usb_control,
        }
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

    /// Checks whether a product string matches known wireless dongle/receiver or Bluetooth signatures.
    pub fn is_wireless_product(product: &str) -> bool {
        let p = product.to_lowercase();
        p.contains("wireless")
            || p.contains("2.4g")
            || p.contains("receiver")
            || p.contains("dongle")
            || p.contains("bluetooth")
            || p.starts_with("bt")
            || p.contains(" bt")
            || p.contains("-bt")
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

    /// Normalizes caller-supplied feature bytes into the exact 65-byte hidapi
    /// wire frame `[Report ID 0x00, 64-byte payload]` for the Windows hidapi
    /// boundary only (GH #18: `HidD_SetFeature` incorrect function).
    ///
    /// Accepted inputs: a raw64 Monka payload (magic `0x04` first) which gains
    /// the `0x00` Report ID prefix, or an already-framed 65-byte buffer
    /// (`0x00` prefix + magic `0x04`) which passes through identically.
    /// Anything else (zero-length, oversized, nonzero prefix, wrong magic)
    /// fails before any device handle is touched, and a nonzero prefix is never
    /// reinterpreted as magic `0x04`. Private: exercised by the colocated unit
    /// tests below without opening a `HidDevice`; the Windows branch of
    /// `send_feature_report` is the sole production caller (hence dead on
    /// non-Windows builds outside tests).
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    fn normalize_feature_frame(data: &[u8]) -> Result<[u8; 65], TransportError> {
        if data.len() == 65 {
            if data[0] != 0 {
                return Err(TransportError::InvalidReportId(data[0]));
            }
            if data[1] != FEATURE_REPORT_MAGIC {
                return Err(TransportError::ProtocolViolation(format!(
                    "Feature frame magic mismatch: expected 0x{FEATURE_REPORT_MAGIC:02X}, got 0x{:02X}",
                    data[1]
                )));
            }
            let mut frame = [0u8; 65];
            frame.copy_from_slice(data);
            return Ok(frame);
        }
        if data.len() == 64 {
            if data[0] != FEATURE_REPORT_MAGIC {
                return Err(TransportError::ProtocolViolation(format!(
                    "Feature payload magic mismatch: expected 0x{FEATURE_REPORT_MAGIC:02X}, got 0x{:02X}",
                    data[0]
                )));
            }
            let mut frame = [0u8; 65];
            frame[0] = 0;
            frame[1..].copy_from_slice(data);
            return Ok(frame);
        }
        if data.len() < 64 {
            return Err(TransportError::BufferTooSmall {
                needed: 64,
                provided: data.len(),
            });
        }
        Err(TransportError::ProtocolViolation(format!(
            "Feature frame must be 64 or 65 bytes, got {} bytes",
            data.len()
        )))
    }

    /// Evaluates connection state based on device wireless mode and non-destructive query outcome per DISC-05.
    pub fn evaluate_state_query(
        is_wireless: bool,
        query_result: Result<usize, TransportError>,
    ) -> Result<ConnectionState, TransportError> {
        match query_result {
            Ok(n) if n > 0 => {
                if is_wireless {
                    Ok(ConnectionState::WirelessAwake)
                } else {
                    Ok(ConnectionState::WiredUsb)
                }
            }
            Ok(_) => {
                if is_wireless {
                    Ok(ConnectionState::WirelessSleeping)
                } else {
                    Err(TransportError::Timeout)
                }
            }
            Err(TransportError::Timeout) => {
                if is_wireless {
                    Ok(ConnectionState::WirelessSleeping)
                } else {
                    Err(TransportError::Timeout)
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Detects active connection state via non-destructive read with finite timeout (50ms) per DISC-05 and D-12.
    /// Emits zero flash commits and zero DFU/ISP mode triggers.
    /// Buffer is 65 bytes (1 byte Report ID prefix + 64 bytes payload) to prevent macOS buffer overrun on Report ID 0.
    pub fn detect_connection_state(&mut self) -> Result<ConnectionState, TransportError> {
        let mut probe_buf = [0u8; 65];
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

    /// Sends a feature report to the device (Interface B control pipe on macOS USB, or hidapi feature report).
    fn send_feature_report(&mut self, data: &[u8]) -> Result<(), TransportError> {
        // macOS path (base behavior, byte-for-byte): when the separate
        // Interface B control pipe is attached, route the payload there
        // unmodified. hidapi already applies the report-ID convention of that
        // transport (`macos-shared-device`), so no Windows-style `[0x00; 65]`
        // framing is applied here.
        #[cfg(target_os = "macos")]
        if let Some(ref mut ctrl) = self.usb_control {
            return ctrl.send_feature_report(data);
        }
        // Windows hidapi boundary (GH #18): the configuration collection
        // expects the exact `[0x00, payload64]` frame. Normalization runs only
        // on this branch, so non-Windows control paths never see it.
        #[cfg(target_os = "windows")]
        {
            let frame = Self::normalize_feature_frame(data)?;
            return self
                .device
                .send_feature_report(&frame)
                .map_err(TransportError::from);
        }
        // Legacy direct path (macOS without a control pipe, Linux, other):
        // preserves base byte-for-byte hidapi behavior.
        #[cfg(not(target_os = "windows"))]
        {
            self.device
                .send_feature_report(data)
                .map_err(TransportError::from)
        }
    }

    /// Reads a feature report from the device (Interface 3 control pipe on macOS USB, or hidapi feature report).
    /// Ensures buf[0] = report_id before invocation.
    fn get_feature_report(
        &mut self,
        report_id: u8,
        buf: &mut [u8],
    ) -> Result<usize, TransportError> {
        Self::prepare_feature_buffer(report_id, buf)?;
        #[cfg(target_os = "macos")]
        if let Some(ref mut ctrl) = self.usb_control {
            return ctrl.get_feature_report(report_id, buf);
        }
        self.device
            .get_feature_report(buf)
            .map_err(TransportError::from)
    }

    /// Reads an input report from the device with a millisecond timeout.
    /// Maps a 0-byte return to TransportError::Timeout.
    fn read_input_report(
        &mut self,
        buf: &mut [u8],
        timeout_ms: i32,
    ) -> Result<usize, TransportError> {
        let bytes_read = self
            .device
            .read_timeout(buf, timeout_ms)
            .map_err(TransportError::from)?;
        if bytes_read == 0 {
            return Err(TransportError::Timeout);
        }
        Ok(bytes_read)
    }
}

#[cfg(test)]
mod tests {
    use super::HidTransport;
    use crate::transport::TransportError;

    fn raw64_tail() -> [u8; 64] {
        // Framing-only fixture: magic 0x04 at [0], command 0x13 at [1].
        let mut raw = [0u8; 64];
        raw[0] = crate::FEATURE_REPORT_MAGIC;
        raw[1] = 0x13;
        raw[3..6].copy_from_slice(&[0x11, 0x22, 0x33]);
        raw
    }

    #[test]
    fn normalize_raw64_prefixes_report_id_zero() {
        let raw = raw64_tail();
        let frame = HidTransport::normalize_feature_frame(&raw).expect("raw64 must normalize");
        assert_eq!(frame.len(), 65);
        assert_eq!(frame[0], 0x00);
        assert_eq!(frame[1], crate::FEATURE_REPORT_MAGIC);
        assert_eq!(&frame[1..], &raw[..]);
    }

    #[test]
    fn normalize_prefixed65_passes_through_identically() {
        let raw = raw64_tail();
        let mut prefixed = [0u8; 65];
        prefixed[0] = 0x00;
        prefixed[1..].copy_from_slice(&raw);
        let frame =
            HidTransport::normalize_feature_frame(&prefixed).expect("prefixed65 must pass through");
        assert_eq!(frame, prefixed);
    }

    #[test]
    fn normalize_rejects_zero_length_before_any_io() {
        let err = HidTransport::normalize_feature_frame(&[]).unwrap_err();
        assert!(matches!(err, TransportError::BufferTooSmall { .. }));
    }

    #[test]
    fn normalize_rejects_oversized_before_any_io() {
        let oversized = [0u8; 66];
        let err = HidTransport::normalize_feature_frame(&oversized).unwrap_err();
        assert!(matches!(err, TransportError::ProtocolViolation(_)));
    }

    #[test]
    fn normalize_rejects_nonzero_prefix_without_magic_reinterpretation() {
        let raw = raw64_tail();
        let mut bad = [0u8; 65];
        bad[0] = 0x01;
        bad[1..].copy_from_slice(&raw);
        let err = HidTransport::normalize_feature_frame(&bad).unwrap_err();
        assert!(matches!(err, TransportError::InvalidReportId(0x01)));
    }

    #[test]
    fn normalize_rejects_wrong_magic_in_both_shapes() {
        let mut raw = raw64_tail();
        raw[0] = 0xFF;
        assert!(matches!(
            HidTransport::normalize_feature_frame(&raw).unwrap_err(),
            TransportError::ProtocolViolation(_)
        ));
        let mut prefixed = [0u8; 65];
        prefixed[0] = 0x00;
        prefixed[1] = 0xFF;
        assert!(matches!(
            HidTransport::normalize_feature_frame(&prefixed).unwrap_err(),
            TransportError::ProtocolViolation(_)
        ));
    }
}
