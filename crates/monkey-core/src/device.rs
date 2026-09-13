use std::ffi::CString;
use serde::{Deserialize, Serialize};
use crate::error::TransportError;

/// Vendor ID for Monka 3075 Pro / RKGK890 (0x05AC = 1452) per D-01.
pub const MONKA_VID: u16 = 0x05AC;

/// Product ID for Monka 3075 Pro / RKGK890 (0x024F = 591) per D-01.
pub const MONKA_PID: u16 = 0x024F;

/// Product identifier string descriptor OEM signature per D-01.
pub const PRODUCT_IDENTIFIER: &str = "RKGK890";

/// Role and endpoint capability of a Monka keyboard HID interface per D-02 and DISC-01.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InterfaceRole {
    /// Interface A: Vendor Bulk Pipe (0xFF68:0x0061), max 4096B OUT.
    InterfaceA,
    /// Interface B: Standard/Vendor Control Pipe (0x000C:0x0001 or 0xFFFF elements), feature reports & control.
    InterfaceB,
}

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

    /// Classifies this device's interface role based on usage tuples and interface number.
    pub fn role(&self) -> Option<InterfaceRole> {
        classify_interface(self.usage_page, self.usage, self.interface_number)
    }
}

/// Classifies an interface into InterfaceA, InterfaceB, or None per D-02:
/// - Interface A: usage_page == 0xFF68 && usage == 0x0061 (Vendor Bulk Pipe, 4096B OUT)
/// - Interface B: (usage_page == 0x000C && usage == 0x0001) || usage_page == 0xFFFF (Control Pipe)
/// - Fallback: if usage_page == 0 && usage == 0 (e.g. Linux libusb), map interface 0 -> A, 1 -> B
pub fn classify_interface(usage_page: u16, usage: u16, interface_number: i32) -> Option<InterfaceRole> {
    if usage_page == 0xFF68 && usage == 0x0061 {
        Some(InterfaceRole::InterfaceA)
    } else if (usage_page == 0x000C && usage == 0x0001) || usage_page == 0xFFFF {
        Some(InterfaceRole::InterfaceB)
    } else if usage_page == 0 && usage == 0 {
        match interface_number {
            0 => Some(InterfaceRole::InterfaceA),
            1 => Some(InterfaceRole::InterfaceB),
            _ => None,
        }
    } else {
        None
    }
}

/// Composite device set representing a physical Monka keyboard with its dual HID interfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonkaDeviceSet {
    pub interface_a: Option<DiscoveredDevice>,
    pub interface_b: Option<DiscoveredDevice>,
    pub vid: u16,
    pub pid: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
    pub observed_usages: Vec<(u16, u16)>,
}

impl MonkaDeviceSet {
    pub fn new(vid: u16, pid: u16) -> Self {
        Self {
            interface_a: None,
            interface_b: None,
            vid,
            pid,
            manufacturer: None,
            product: None,
            serial_number: None,
            observed_usages: Vec::new(),
        }
    }

    pub fn is_complete(&self) -> bool {
        self.interface_a.is_some() && self.interface_b.is_some()
    }

    pub fn has_interface_a(&self) -> bool {
        self.interface_a.is_some()
    }

    pub fn has_interface_b(&self) -> bool {
        self.interface_b.is_some()
    }
}

fn physical_device_key(device: &DiscoveredDevice) -> Option<String> {
    if let Some(ref sn) = device.serial_number {
        if !sn.trim().is_empty() {
            return Some(format!("sn:{}", sn.trim()));
        }
    }
    let path_str = device.path.to_str().unwrap_or_default();
    if let Some(pos) = path_str.rfind(':') {
        if pos > 0 && path_str[pos..].starts_with(":1.") {
            return Some(format!("path_parent:{}", &path_str[..pos]));
        }
    }
    if let Some(pos) = path_str.find("/IOUSBHostInterface@") {
        return Some(format!("path_parent:{}", &path_str[..pos]));
    }
    None
}

/// Parses numeric registry entry ID from macOS `DevSrvsID:<u64>` path.
fn parse_devsrvs_id(path: &CString) -> Option<u64> {
    let s = path.to_str().ok()?;
    s.strip_prefix("DevSrvsID:")?.parse::<u64>().ok()
}

/// Determines whether a discovered device interface belongs to an existing `MonkaDeviceSet`.
fn matches_set(dev: &DiscoveredDevice, set: &MonkaDeviceSet, is_ambiguous: bool) -> bool {
    let role = match dev.role() {
        Some(r) => r,
        None => return false,
    };

    // If this device shares the exact path with an interface in this set (e.g. multi-usage descriptor on macOS)
    if set.interface_a.as_ref().map(|d| &d.path) == Some(&dev.path)
        || set.interface_b.as_ref().map(|d| &d.path) == Some(&dev.path)
    {
        return true;
    }

    // If the set already has an interface for this role with a different path, do not pair
    let role_already_present = match role {
        InterfaceRole::InterfaceA => set.interface_a.is_some(),
        InterfaceRole::InterfaceB => set.interface_b.is_some(),
    };
    if role_already_present {
        return false;
    }

    // 1. Check physical device key (e.g. non-empty serial number or parent USB path)
    if let Some(dev_key) = physical_device_key(dev) {
        let set_key = set.interface_a.as_ref()
            .and_then(physical_device_key)
            .or_else(|| set.interface_b.as_ref().and_then(physical_device_key));
        return set_key.as_ref() == Some(&dev_key);
    }

    // 2. Check macOS DevSrvsID proximity (IOKit registry entry IDs for composite interfaces on same device are within delta <= 32)
    if let Some(dev_id) = parse_devsrvs_id(&dev.path) {
        let existing_ids: Vec<u64> = [set.interface_a.as_ref(), set.interface_b.as_ref()]
            .into_iter()
            .flatten()
            .filter_map(|d| parse_devsrvs_id(&d.path))
            .collect();
        if !existing_ids.is_empty() {
            return existing_ids.iter().any(|&eid| dev_id.abs_diff(eid) <= 32);
        }
    }

    // 3. Fallback: pair unkeyed devices only if there is zero ambiguity (exactly one unkeyed device per role)
    if !is_ambiguous {
        let needs_role = match role {
            InterfaceRole::InterfaceA => set.interface_a.is_none(),
            InterfaceRole::InterfaceB => set.interface_b.is_none(),
        };
        return needs_role;
    }

    false
}

/// Groups discovered Monka HID candidate records into logical `MonkaDeviceSet` collections,
/// deduplicating multi-record descriptors (e.g. macOS IOHID duplicate usage pairs) and grouping
/// candidates sharing the same physical USB device.
pub fn group_monka_devices(devices: &[DiscoveredDevice]) -> Vec<MonkaDeviceSet> {
    let mut sets: Vec<MonkaDeviceSet> = Vec::new();

    // Check if there are multiple unkeyed candidates that cannot be disambiguated by physical keys or DevSrvsID proximity
    let unkeyed_role_a_count = devices
        .iter()
        .filter(|d| {
            d.role() == Some(InterfaceRole::InterfaceA)
                && physical_device_key(d).is_none()
                && parse_devsrvs_id(&d.path).is_none()
        })
        .count();
    let unkeyed_role_b_count = devices
        .iter()
        .filter(|d| {
            d.role() == Some(InterfaceRole::InterfaceB)
                && physical_device_key(d).is_none()
                && parse_devsrvs_id(&d.path).is_none()
        })
        .count();
    let is_ambiguous = unkeyed_role_a_count > 1 || unkeyed_role_b_count > 1;

    for dev in devices {
        let role = match dev.role() {
            Some(r) => r,
            None => continue,
        };

        let mut target_set_idx = None;

        for (i, set) in sets.iter().enumerate() {
            if matches_set(dev, set, is_ambiguous) {
                target_set_idx = Some(i);
                break;
            }
        }

        let set_idx = match target_set_idx {
            Some(idx) => idx,
            None => {
                let new_set = MonkaDeviceSet::new(dev.vid, dev.pid);
                sets.push(new_set);
                sets.len() - 1
            }
        };

        let set = &mut sets[set_idx];

        if set.manufacturer.is_none() && dev.manufacturer.is_some() {
            set.manufacturer = dev.manufacturer.clone();
        }
        if set.product.is_none() && dev.product.is_some() {
            set.product = dev.product.clone();
        }
        if set.serial_number.is_none() && dev.serial_number.is_some() {
            set.serial_number = dev.serial_number.clone();
        }

        let usage_tuple = (dev.usage_page, dev.usage);
        if !set.observed_usages.contains(&usage_tuple) {
            set.observed_usages.push(usage_tuple);
        }

        match role {
            InterfaceRole::InterfaceA => {
                if set.interface_a.is_none() {
                    set.interface_a = Some(dev.clone());
                }
            }
            InterfaceRole::InterfaceB => {
                if set.interface_b.is_none() {
                    set.interface_b = Some(dev.clone());
                }
            }
        }
    }

    sets
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

/// Scans and groups connected Monka 3075 Pro devices into complete dual-interface device sets.
pub fn find_monka_device_sets(api: &hidapi::HidApi) -> Vec<MonkaDeviceSet> {
    let devices = find_monka_devices(api);
    group_monka_devices(&devices)
}

/// Opens an exact device interface by its enumerated path rather than VID/PID,
/// ensuring non-exclusive access to the intended composite endpoint per D-03.
pub fn open_device_path(api: &hidapi::HidApi, device: &DiscoveredDevice) -> Result<hidapi::HidDevice, TransportError> {
    api.open_path(&device.path).map_err(|e| TransportError::HidError(e.to_string()))
}
