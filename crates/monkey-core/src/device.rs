use std::ffi::CString;
use crate::error::TransportError;

/// Vendor ID for Monka 3075 Pro / RKGK890 (0x05AC = 1452) per D-01.
pub const MONKA_VID: u16 = 0x05AC;

/// Product ID for Monka 3075 Pro / RKGK890 (0x024F = 591) per D-01.
pub const MONKA_PID: u16 = 0x024F;

/// Product identifier string descriptor OEM signature per D-01.
pub const PRODUCT_IDENTIFIER: &str = "RKGK890";

/// Discovered USB HID device candidate matching Monka VID/PID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredDevice {
    pub vid: u16,
    pub pid: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
    pub path: CString,
    pub usage_page: u16,
    pub usage: u16,
    pub interface_number: i32,
}

impl DiscoveredDevice {
    /// Creates a DiscoveredDevice record from a hidapi DeviceInfo reference.
    pub fn from_device_info(info: &hidapi::DeviceInfo) -> Self {
        Self {
            vid: info.vendor_id(),
            pid: info.product_id(),
            manufacturer: info.manufacturer_string().map(ToString::to_string),
            product: info.product_string().map(ToString::to_string),
            serial_number: info.serial_number().map(ToString::to_string),
            path: info.path().to_owned(),
            usage_page: info.usage_page(),
            usage: info.usage(),
            interface_number: info.interface_number(),
        }
    }
}

/// Initializes `hidapi::HidApi` context configured with non-exclusive macOS device access
/// via the `macos-shared-device` feature flag (`hid_darwin_set_open_exclusive(0)`) per D-03.
pub fn init_hidapi() -> Result<hidapi::HidApi, TransportError> {
    hidapi::HidApi::new().map_err(|e| TransportError::HidError(e.to_string()))
}

/// Scans the system for connected Monka 3075 Pro devices matching `MONKA_VID` and `MONKA_PID`.
pub fn find_monka_devices(api: &hidapi::HidApi) -> Vec<DiscoveredDevice> {
    api.device_list()
        .filter(|d| d.vendor_id() == MONKA_VID && d.product_id() == MONKA_PID)
        .map(DiscoveredDevice::from_device_info)
        .collect()
}
