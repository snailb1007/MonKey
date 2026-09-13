use anyhow::Context;
use serde::Serialize;

use monkey_core::device::{find_monka_devices, init_hidapi, MONKA_PID, MONKA_VID, PRODUCT_IDENTIFIER};
use crate::output::OutputFormat;

#[derive(Debug, Clone, Serialize)]
pub struct BasicDeviceInfo {
    pub vid_hex: String,
    pub pid_hex: String,
    pub product: String,
    pub device_count: usize,
}

pub fn run_info(format: OutputFormat) -> anyhow::Result<()> {
    let api = init_hidapi().context("Failed to initialize HID API")?;
    let devices = find_monka_devices(&api);

    let info = BasicDeviceInfo {
        vid_hex: format!("0x{:04x}", MONKA_VID),
        pid_hex: format!("0x{:04x}", MONKA_PID),
        product: PRODUCT_IDENTIFIER.to_string(),
        device_count: devices.len(),
    };

    let human = format!(
        "MonKey Device Info:\n  VID: {}\n  PID: {}\n  Product: {}\n  Endpoints Found: {}",
        info.vid_hex, info.pid_hex, info.product, info.device_count
    );

    format.print(&human, &info)?;
    Ok(())
}
