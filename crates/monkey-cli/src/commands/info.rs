use std::io::Write;
use anyhow::Context;
use serde::{Deserialize, Serialize};

use monkey_core::device::{
    find_monka_device_sets, init_hidapi, MonkaDeviceSet, MONKA_PID, MONKA_VID, PRODUCT_IDENTIFIER,
};
use crate::output::OutputFormat;

/// Structured description of a composite interface endpoint per DISC-02.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InterfaceInfoOutput {
    pub name: String,
    pub role: String,
    pub usage_page: String,
    pub usage: String,
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub max_feature_bytes: usize,
    pub path: String,
}

/// Structured hardware identity inspection output per D-01, D-11, and DISC-02.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceInfoOutput {
    pub vid: String,
    pub pid: String,
    pub vid_hex: String,
    pub pid_hex: String,
    pub vid_dec: u16,
    pub pid_dec: u16,
    pub product: String,
    pub product_name: String,
    pub manufacturer: Option<String>,
    pub serial_number: Option<String>,
    pub release_number: Option<String>,
    pub solution_vendor: Option<String>,
    pub interfaces: Vec<InterfaceInfoOutput>,
}

/// Builds structured `DeviceInfoOutput` from a discovered `MonkaDeviceSet`.
pub fn build_info_output(set: &MonkaDeviceSet) -> DeviceInfoOutput {
    let mut interfaces = Vec::new();

    if let Some(ref dev_a) = set.interface_a {
        interfaces.push(InterfaceInfoOutput {
            name: "Interface A (Vendor Bulk Pipe)".to_string(),
            role: "InterfaceA".to_string(),
            usage_page: format!("0x{:04x}", dev_a.usage_page),
            usage: format!("0x{:04x}", dev_a.usage),
            max_input_bytes: 64,
            max_output_bytes: 4096,
            max_feature_bytes: 0,
            path: dev_a.path.to_string_lossy().into_owned(),
        });
    }

    if let Some(ref dev_b) = set.interface_b {
        interfaces.push(InterfaceInfoOutput {
            name: "Interface B (Standard/Vendor Control Pipe)".to_string(),
            role: "InterfaceB".to_string(),
            usage_page: format!("0x{:04x}", dev_b.usage_page),
            usage: format!("0x{:04x}", dev_b.usage),
            max_input_bytes: 16,
            max_output_bytes: 1,
            max_feature_bytes: 64,
            path: dev_b.path.to_string_lossy().into_owned(),
        });
    }

    let prod = set
        .product
        .clone()
        .unwrap_or_else(|| PRODUCT_IDENTIFIER.to_string());

    DeviceInfoOutput {
        vid: format!("0x{:04x}", set.vid),
        pid: format!("0x{:04x}", set.pid),
        vid_hex: format!("0x{:04x}", set.vid),
        pid_hex: format!("0x{:04x}", set.pid),
        vid_dec: set.vid,
        pid_dec: set.pid,
        product: prod.clone(),
        product_name: prod,
        manufacturer: set
            .manufacturer
            .clone()
            .or_else(|| Some("Shenzhen HFD Technology Co., Ltd.".to_string())),
        serial_number: set.serial_number.clone().or_else(|| Some("N/A".to_string())),
        release_number: Some("1.00".to_string()),
        solution_vendor: Some("Shenzhen HFD Technology Co., Ltd. (RKGK OEM)".to_string()),
        interfaces,
    }
}

/// Formats `DeviceInfoOutput` into a human-readable terminal report.
pub fn format_info_human(info: &DeviceInfoOutput) -> String {
    let mut out = String::new();
    out.push_str("============================================================\n");
    out.push_str("              MonKey Hardware Information                   \n");
    out.push_str("============================================================\n");
    out.push_str(&format!("  Vendor ID (VID):        {} ({})\n", info.vid_hex, info.vid_dec));
    out.push_str(&format!("  Product ID (PID):       {} ({})\n", info.pid_hex, info.pid_dec));
    out.push_str(&format!("  Product Name:           {}\n", info.product_name));
    if let Some(ref mfr) = info.manufacturer {
        out.push_str(&format!("  Manufacturer:           {}\n", mfr));
    }
    if let Some(ref sn) = info.serial_number {
        out.push_str(&format!("  Serial Number:          {}\n", sn));
    }
    if let Some(ref rel) = info.release_number {
        out.push_str(&format!("  Release Number:         {}\n", rel));
    }
    if let Some(ref vendor) = info.solution_vendor {
        out.push_str(&format!("  Solution OEM:           {}\n", vendor));
    }

    out.push_str("\nComposite HID Interfaces:\n");
    if info.interfaces.is_empty() {
        out.push_str("  (No endpoints active)\n");
    } else {
        for iface in &info.interfaces {
            out.push_str("  ----------------------------------------------------------\n");
            out.push_str(&format!("  {}\n", iface.name));
            out.push_str(&format!("    Role:                 {}\n", iface.role));
            out.push_str(&format!(
                "    Usage Page / Usage:   {} / {}\n",
                iface.usage_page, iface.usage
            ));
            out.push_str(&format!(
                "    Buffer Sizes:         IN: {}B | OUT: {}B | Feature: {}B\n",
                iface.max_input_bytes, iface.max_output_bytes, iface.max_feature_bytes
            ));
            out.push_str(&format!("    Path:                 {}\n", iface.path));
        }
    }
    out.push_str("============================================================");
    out
}

/// Executes `monkey info` writing to stdout.
pub fn run_info(format: OutputFormat) -> anyhow::Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    run_info_with_writer(format, &mut handle)
}

/// Executes `monkey info` using a custom writer for headless testability.
pub fn run_info_with_writer<W: Write>(format: OutputFormat, writer: &mut W) -> anyhow::Result<()> {
    let api = init_hidapi().context("Failed to initialize HID API")?;
    let sets = if std::env::var("MONKEY_SIMULATE_EMPTY").is_ok() {
        Vec::new()
    } else {
        find_monka_device_sets(&api)
    };

    if sets.is_empty() {
        anyhow::bail!(
            "No Monka 3075 Pro / RKGK890 keyboard detected (VID: 0x{:04x}, PID: 0x{:04x}). Please check USB connection.",
            MONKA_VID,
            MONKA_PID
        );
    }

    run_info_with_device_set(&sets[0], format, writer)
}

/// Formats and outputs information for a specific `MonkaDeviceSet`.
pub fn run_info_with_device_set<W: Write>(
    set: &MonkaDeviceSet,
    format: OutputFormat,
    writer: &mut W,
) -> anyhow::Result<()> {
    let info = build_info_output(set);
    let human = format_info_human(&info);
    format.write_to(writer, &human, &info)?;
    Ok(())
}
