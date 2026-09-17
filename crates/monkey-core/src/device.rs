use crate::bench::{BenchmarkConfig, LatencyReport, ThroughputReport};
use crate::error::{MonkeyError, TransportError};
use crate::lcd::{LcdPacingConfig, LcdStreamMetrics, LcdStreamer, LCD_FRAME_BYTES};
use crate::protocol::SafetyRails;
use crate::rgb::{LightingConfig, RgbManager};
use crate::transport::{HidTransport, SafeTransport, Transport};
use serde::{Deserialize, Serialize};
use std::ffi::CString;

/// Vendor ID for Monka 3075 Pro / RKGK890 (0x05AC = 1452) per D-01.
pub const MONKA_VID: u16 = 0x05AC;

/// Product ID for Monka 3075 Pro / RKGK890 (0x024F = 591) per D-01.
pub const MONKA_PID: u16 = 0x024F;

/// Known ISP / Bootloader Product ID (e.g. 0x7140) across HFD / Ajazz / RKGK boards.
/// Connecting or writing commands in bootloader state risks permanent flash bricking.
pub const ISP_BOOTLOADER_PID: u16 = 0x7140;

/// Checks whether a given VID/PID pair corresponds to a known dangerous ISP bootloader state.
#[must_use]
pub fn is_isp_bootloader(vid: u16, pid: u16) -> bool {
    pid == ISP_BOOTLOADER_PID || (vid == MONKA_VID && pid == ISP_BOOTLOADER_PID)
}

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
pub fn classify_interface(
    usage_page: u16,
    usage: u16,
    interface_number: i32,
) -> Option<InterfaceRole> {
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

    pub fn is_wireless(&self) -> bool {
        let product_match = self
            .product
            .as_deref()
            .map(crate::transport::HidTransport::is_wireless_product)
            .unwrap_or(false);
        let iface_match = [&self.interface_a, &self.interface_b]
            .iter()
            .filter_map(|iface| iface.as_ref())
            .any(|d| {
                d.product
                    .as_deref()
                    .map(crate::transport::HidTransport::is_wireless_product)
                    .unwrap_or(false)
            });
        product_match || iface_match
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
        let set_key = set
            .interface_a
            .as_ref()
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
        let mut min_diff = u64::MAX;

        for (i, set) in sets.iter().enumerate() {
            if matches_set(dev, set, is_ambiguous) {
                // If matched via DevSrvsID proximity, find the set with minimum distance
                if let Some(dev_id) = parse_devsrvs_id(&dev.path) {
                    let set_dev_id = set
                        .interface_a
                        .as_ref()
                        .or(set.interface_b.as_ref())
                        .and_then(|d| parse_devsrvs_id(&d.path));
                    if let Some(sid) = set_dev_id {
                        let diff = dev_id.abs_diff(sid);
                        if diff < min_diff {
                            min_diff = diff;
                            target_set_idx = Some(i);
                        }
                        continue;
                    }
                }

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
/// Explicitly excludes any device in ISP / bootloader state (`ISP_BOOTLOADER_PID`).
pub fn find_monka_devices(api: &hidapi::HidApi) -> Vec<DiscoveredDevice> {
    api.device_list()
        .filter(|d| {
            d.vendor_id() == MONKA_VID
                && d.product_id() == MONKA_PID
                && !is_isp_bootloader(d.vendor_id(), d.product_id())
        })
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
///
/// Validates that the device VID/PID matches expected Monka hardware and is NOT
/// an ISP bootloader before opening the interface handle.
pub fn open_device_path(
    api: &hidapi::HidApi,
    device: &DiscoveredDevice,
) -> Result<hidapi::HidDevice, TransportError> {
    if is_isp_bootloader(device.vid, device.pid) {
        return Err(TransportError::ProtocolViolation(format!(
            "Refusing to open device in ISP bootloader mode (VID: 0x{:04X}, PID: 0x{:04X})",
            device.vid, device.pid
        )));
    }
    if device.vid != MONKA_VID || device.pid != MONKA_PID {
        return Err(TransportError::DeviceNotFound(format!(
            "Invalid device identifier (VID: 0x{:04X}, PID: 0x{:04X})",
            device.vid, device.pid
        )));
    }
    api.open_path(&device.path)
        .map_err(|e| TransportError::HidError(e.to_string()))
}

/// Declarative interface selection policy for Monka 3075 Pro hardware endpoints per D-02.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfacePolicy {
    /// Interface B first, fallback to Interface A (used by `rgb`).
    PreferB,
    /// Interface A only; hard fail if missing (used by `lcd`).
    RequireA,
    /// Interface B only; hard fail if missing (used by `probe`).
    RequireB,
    /// Interface A then Interface B (used by `bench` when running bulk transfers).
    BulkFirst,
    /// Interface B then Interface A (used by `bench` when running feature/transaction transfers).
    ControlFirst,
    /// Open whatever valid Monka interface is available.
    Any,
}

impl InterfacePolicy {
    /// Resolves target device descriptor and role based on the interface policy.
    pub fn resolve(
        &self,
        set: &MonkaDeviceSet,
    ) -> Result<(DiscoveredDevice, InterfaceRole), OpenError> {
        match self {
            Self::RequireA => {
                let dev = set
                    .interface_a
                    .clone()
                    .ok_or(OpenError::InterfaceUnavailable(InterfaceRole::InterfaceA))?;
                Ok((dev, InterfaceRole::InterfaceA))
            }
            Self::RequireB => {
                let dev = set
                    .interface_b
                    .clone()
                    .ok_or(OpenError::InterfaceUnavailable(InterfaceRole::InterfaceB))?;
                Ok((dev, InterfaceRole::InterfaceB))
            }
            Self::PreferB => {
                if let Some(b) = &set.interface_b {
                    Ok((b.clone(), InterfaceRole::InterfaceB))
                } else if let Some(a) = &set.interface_a {
                    Ok((a.clone(), InterfaceRole::InterfaceA))
                } else {
                    Err(OpenError::InterfaceUnavailable(InterfaceRole::InterfaceB))
                }
            }
            Self::BulkFirst => {
                if let Some(a) = &set.interface_a {
                    Ok((a.clone(), InterfaceRole::InterfaceA))
                } else if let Some(b) = &set.interface_b {
                    Ok((b.clone(), InterfaceRole::InterfaceB))
                } else {
                    Err(OpenError::InterfaceUnavailable(InterfaceRole::InterfaceA))
                }
            }
            Self::ControlFirst | Self::Any => {
                if let Some(b) = &set.interface_b {
                    Ok((b.clone(), InterfaceRole::InterfaceB))
                } else if let Some(a) = &set.interface_a {
                    Ok((a.clone(), InterfaceRole::InterfaceA))
                } else {
                    Err(OpenError::InterfaceUnavailable(InterfaceRole::InterfaceB))
                }
            }
        }
    }

    /// Convenience wrapper to resolve policy against a device set.
    pub fn resolve_policy(
        set: &MonkaDeviceSet,
        policy: Self,
    ) -> Result<(DiscoveredDevice, InterfaceRole), OpenError> {
        policy.resolve(set)
    }
}

/// Granular error variants representing hardware device initialization stages per D-03.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OpenError {
    #[error("Failed to initialize HID subsystem: {0}")]
    HidInit(#[from] TransportError),

    #[error("No Monka 3075 Pro / RKGK890 keyboard detected (VID: 0x{MONKA_VID:04x}, PID: 0x{MONKA_PID:04x}). Please check USB connection.")]
    NoDevice,

    #[error("Requested interface {0:?} is not available on detected keyboard")]
    InterfaceUnavailable(InterfaceRole),

    #[error("Failed to open interface {0:?}: {1}")]
    InterfaceOpenFailed(InterfaceRole, TransportError),
}

/// Non-fail-fast diagnostic inspection tree evaluated by `MonkaDevice::diagnose()` per D-03.
#[derive(Debug)]
pub struct DeviceDiagnostics {
    pub hid_init: Result<(), TransportError>,
    pub device_set: Option<MonkaDeviceSet>,
    pub interface_a_status: InterfaceCheckStatus,
    pub interface_b_status: InterfaceCheckStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceCheckStatus {
    NotPresent,
    OpenSuccess,
    OpenFailed(TransportError),
}

/// Structured capability tuple inspection output per D-11, D-12, and DISC-03.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeOutput {
    pub model: String,
    pub hardware_revision: String,
    pub firmware_version: String,
    pub transport_state: String,
    pub capabilities: Vec<String>,
    pub interface_a_detected: bool,
    pub interface_b_detected: bool,
    pub read_only_verified: bool,
}

/// Parses hardware revision and firmware version strings from raw feature report query slice.
pub fn parse_probe_response(slice: &[u8]) -> (String, String) {
    if slice.is_empty() {
        return ("unknown".to_string(), "unknown".to_string());
    }
    let payload =
        if slice[0] == 0 && slice.len() > 1 && slice[1] == crate::protocol::FEATURE_REPORT_MAGIC {
            &slice[1..]
        } else if slice[0] == crate::protocol::FEATURE_REPORT_MAGIC {
            slice
        } else {
            return ("unknown".to_string(), "unknown".to_string());
        };

    let payload = if payload.len() >= 64 {
        &payload[..64]
    } else {
        return ("unknown".to_string(), "unknown".to_string());
    };

    if let Ok(packet) = crate::protocol::FeatureReportPacket::parse_from_slice(payload) {
        let rev = format!("rev{}.{}", packet.command, packet.args[0]);
        let ver = format!("v{}.{}.{}", packet.args[1], packet.args[2], packet.args[3]);
        (rev, ver)
    } else {
        ("unknown".to_string(), "unknown".to_string())
    }
}

/// Determines dynamic hardware capabilities based on detected interface presence per D-02 and DISC-01.
pub fn determine_capabilities(interface_a: bool, interface_b: bool) -> Vec<String> {
    let mut capabilities = vec!["81-key RGB matrix".to_string()];
    if interface_a {
        capabilities.push("LCD display 128x128 RGB565".to_string());
    }
    if interface_a && interface_b {
        capabilities.push("dual composite interface".to_string());
    }
    capabilities
}

/// Concrete hardware device handle encapsulating transport I/O and safety rails per D-01 and D-04.
pub struct MonkaDevice {
    transport: Box<dyn Transport>,
    safety: SafetyRails,
    device_set: Option<MonkaDeviceSet>,
    role: Option<InterfaceRole>,
    is_wireless: bool,
}

impl MonkaDevice {
    /// Opens a physical Monka keyboard handle matching the specified interface policy.
    pub fn open(policy: InterfacePolicy) -> Result<Self, OpenError> {
        let api = init_hidapi().map_err(OpenError::HidInit)?;
        let sets = find_monka_device_sets(&api);
        let device_set = sets.into_iter().next().ok_or(OpenError::NoDevice)?;
        let is_wireless = device_set.is_wireless();

        let (target_dev, role) = Self::resolve_policy(&device_set, policy)?;
        let hid_dev = open_device_path(&api, &target_dev)
            .map_err(|e| OpenError::InterfaceOpenFailed(role, e))?;

        Ok(Self {
            transport: Box::new(HidTransport::new(hid_dev)),
            safety: SafetyRails::new(),
            device_set: Some(device_set),
            role: Some(role),
            is_wireless,
        })
    }

    /// Creates a MonkaDevice from an existing transport implementation (for headless testing and mocks).
    pub fn from_transport(transport: Box<dyn Transport>) -> Self {
        Self {
            transport,
            safety: SafetyRails::new(),
            device_set: None,
            role: None,
            is_wireless: false,
        }
    }

    /// Builder helper to attach mock device set metadata.
    #[must_use]
    pub fn with_device_set(mut self, device_set: MonkaDeviceSet) -> Self {
        self.device_set = Some(device_set);
        self
    }

    /// Builder helper to configure hardware write consent.
    #[must_use]
    pub fn with_hardware_writes_allowed(self, allowed: bool) -> Self {
        self.safety.set_hardware_writes_allowed(allowed);
        self
    }

    /// Builder helper to declare wireless transport status.
    #[must_use]
    pub fn with_wireless(mut self, wireless: bool) -> Self {
        self.is_wireless = wireless;
        self
    }

    /// Discovers connected Monka device sets without opening endpoint handles.
    pub fn discover() -> Result<Vec<MonkaDeviceSet>, OpenError> {
        let api = init_hidapi().map_err(OpenError::HidInit)?;
        Ok(find_monka_device_sets(&api))
    }

    /// Manufactures an internal SafeTransport guard for deep driver operations.
    pub fn safe_transport(&mut self) -> SafeTransport<'_> {
        SafeTransport::new(&mut *self.transport, &self.safety)
    }

    /// Returns a reference to the bound safety rails.
    pub fn safety(&self) -> &SafetyRails {
        &self.safety
    }

    /// Returns whether the device is connected wirelessly.
    pub fn is_wireless(&self) -> bool {
        self.is_wireless
    }

    /// Returns the active interface role, if opened via policy.
    pub fn role(&self) -> Option<InterfaceRole> {
        self.role
    }

    /// Returns the active device set, if available.
    pub fn device_set(&self) -> Option<&MonkaDeviceSet> {
        self.device_set.as_ref()
    }

    /// Resolves target device descriptor and role based on the interface policy.
    pub fn resolve_policy(
        set: &MonkaDeviceSet,
        policy: InterfacePolicy,
    ) -> Result<(DiscoveredDevice, InterfaceRole), OpenError> {
        policy.resolve(set)
    }

    /// Executes multi-stage non-fail-fast diagnostic inspection for `monkey doctor` per D-03.
    pub fn diagnose() -> DeviceDiagnostics {
        let api = match init_hidapi() {
            Ok(api) => api,
            Err(e) => {
                return DeviceDiagnostics {
                    hid_init: Err(e),
                    device_set: None,
                    interface_a_status: InterfaceCheckStatus::NotPresent,
                    interface_b_status: InterfaceCheckStatus::NotPresent,
                };
            }
        };

        let sets = find_monka_device_sets(&api);
        let device_set = sets.into_iter().next();

        let (interface_a_status, interface_b_status) = if let Some(ref set) = device_set {
            let status_a = match set.interface_a {
                Some(ref dev_a) => match open_device_path(&api, dev_a) {
                    Ok(_) => InterfaceCheckStatus::OpenSuccess,
                    Err(e) => InterfaceCheckStatus::OpenFailed(e),
                },
                None => InterfaceCheckStatus::NotPresent,
            };

            let status_b = match set.interface_b {
                Some(ref dev_b) => match open_device_path(&api, dev_b) {
                    Ok(_) => InterfaceCheckStatus::OpenSuccess,
                    Err(e) => InterfaceCheckStatus::OpenFailed(e),
                },
                None => InterfaceCheckStatus::NotPresent,
            };

            (status_a, status_b)
        } else {
            (
                InterfaceCheckStatus::NotPresent,
                InterfaceCheckStatus::NotPresent,
            )
        };

        DeviceDiagnostics {
            hid_init: Ok(()),
            device_set,
            interface_a_status,
            interface_b_status,
        }
    }

    // --- Deep Module Operations (D-04) ---

    pub fn stream_frame_with_progress<F>(
        &mut self,
        frame: &[u8; LCD_FRAME_BYTES],
        config: LcdPacingConfig,
        on_chunk: F,
    ) -> Result<LcdStreamMetrics, MonkeyError>
    where
        F: FnMut(usize, usize),
    {
        let (raw, safety) = self.safe_transport().into_parts();
        let mut streamer = LcdStreamer::new(raw, safety, config)?;
        streamer.send_frame_with_progress(frame, on_chunk)
    }

    pub fn stream_frame(
        &mut self,
        frame: &[u8; LCD_FRAME_BYTES],
        config: LcdPacingConfig,
    ) -> Result<LcdStreamMetrics, MonkeyError> {
        self.stream_frame_with_progress(frame, config, |_, _| {})
    }

    pub fn apply_rgb_preview(&mut self, config: &LightingConfig) -> Result<(), MonkeyError> {
        let (raw, safety) = self.safe_transport().into_parts();
        let mut manager = RgbManager::new(raw, safety);
        manager.apply_preview(config)
    }

    pub fn apply_rgb_commit(
        &mut self,
        config: &LightingConfig,
        is_wireless: bool,
        battery: Option<u8>,
        force: bool,
    ) -> Result<(), MonkeyError> {
        let (raw, safety) = self.safe_transport().into_parts();
        let mut manager = RgbManager::new(raw, safety);
        manager.apply_commit(config, is_wireless, battery, force)
    }

    pub fn readback_rgb_status(&mut self) -> Result<Option<LightingConfig>, MonkeyError> {
        let (raw, safety) = self.safe_transport().into_parts();
        let mut manager = RgbManager::new(raw, safety);
        manager.readback_status()
    }

    pub fn probe(&mut self) -> Result<ProbeOutput, MonkeyError> {
        let mut probe_buf = [0u8; 65];
        let query_res = self.transport.get_feature_report(0, &mut probe_buf);

        let transport_state =
            match HidTransport::evaluate_state_query(self.is_wireless, query_res.clone()) {
                Ok(state) => format!("{state:?}"),
                Err(e) => {
                    tracing::warn!("Failed to query device feature report during probe: {e}");
                    format!("Unknown/Error: {e}")
                }
            };

        let (interface_a, interface_b) = match self.device_set {
            Some(ref set) => (set.has_interface_a(), set.has_interface_b()),
            None => (false, false),
        };

        let (hw_rev, fw_ver) = match query_res {
            Ok(n) if n > 0 => parse_probe_response(&probe_buf[..n]),
            _ => ("unknown".to_string(), "unknown".to_string()),
        };

        let capabilities = determine_capabilities(interface_a, interface_b);

        Ok(ProbeOutput {
            model: "Monka 3075 Pro / RKGK890".to_string(),
            hardware_revision: hw_rev,
            firmware_version: fw_ver,
            transport_state,
            capabilities,
            interface_a_detected: interface_a,
            interface_b_detected: interface_b,
            read_only_verified: true,
        })
    }

    pub fn run_bulk_benchmark<F>(
        &mut self,
        config: &BenchmarkConfig,
        on_frame: F,
    ) -> Result<ThroughputReport, MonkeyError>
    where
        F: FnMut(usize, usize),
    {
        crate::bench::run_bulk_streaming_bench_with_safe_transport(
            self.safe_transport(),
            config,
            on_frame,
        )
    }

    pub fn run_transaction_benchmark<F>(
        &mut self,
        config: &BenchmarkConfig,
        on_sample: F,
    ) -> Result<LatencyReport, MonkeyError>
    where
        F: FnMut(usize, usize),
    {
        crate::bench::run_transaction_latency_bench_with_safe_transport(
            self.safe_transport(),
            config,
            on_sample,
        )
    }
}
