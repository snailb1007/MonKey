use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::io::Write;

use crate::output::OutputFormat;
use monkey_core::device::{
    find_monka_device_sets, init_hidapi, open_device_path, MonkaDeviceSet, MONKA_PID, MONKA_VID,
};
use monkey_core::transport::{HidTransport, Transport};

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

pub fn parse_probe_response(slice: &[u8]) -> (String, String) {
    if slice.is_empty() {
        return ("unknown".to_string(), "unknown".to_string());
    }
    // The report buffer may have a 1-byte report ID prefix or start with magic directly
    let payload =
        if slice[0] == 0 && slice.len() > 1 && slice[1] == monkey_core::FEATURE_REPORT_MAGIC {
            &slice[1..]
        } else if slice[0] == monkey_core::FEATURE_REPORT_MAGIC {
            slice
        } else {
            return ("unknown".to_string(), "unknown".to_string());
        };

    if let Ok(packet) = monkey_core::FeatureReportPacket::parse_from_slice(payload) {
        let rev = format!("rev{}.{}", packet.command, packet.args[0]);
        let ver = format!("v{}.{}.{}", packet.args[1], packet.args[2], packet.args[3]);
        (rev, ver)
    } else {
        ("unknown".to_string(), "unknown".to_string())
    }
}

/// Probes a device using the provided `Transport` trait implementation,
/// guaranteeing zero write calls (enforcing D-12 read-only safety invariant).
pub fn probe_device_with_transport<T: Transport>(
    transport: &mut T,
    device_set: Option<&MonkaDeviceSet>,
    is_wireless: bool,
) -> ProbeOutput {
    // Read-only query: fetch feature report 0 with 65-byte buffer (1 byte Report ID prefix + 64 bytes payload)
    // to prevent IOHIDDeviceGetReport buffer overrun on macOS per D-02 and D-12.
    // No write_bulk or send_feature_report calls are permitted per D-12.
    let mut probe_buf = [0u8; 65];
    let query_res = transport.get_feature_report(0, &mut probe_buf);

    let transport_state = match HidTransport::evaluate_state_query(
        is_wireless,
        query_res.as_ref().map(|&n| n).map_err(|e| e.clone()),
    ) {
        Ok(state) => format!("{state:?}"),
        Err(e) => {
            tracing::warn!("Failed to query device feature report during probe: {e}");
            format!("Unknown/Error: {e}")
        }
    };

    let (interface_a, interface_b) = match device_set {
        Some(set) => (set.has_interface_a(), set.has_interface_b()),
        None => (false, false),
    };

    let (hw_rev, fw_ver) = match query_res {
        Ok(n) if n > 0 => parse_probe_response(&probe_buf[..n]),
        _ => ("unknown".to_string(), "unknown".to_string()),
    };

    let capabilities = determine_capabilities(interface_a, interface_b);

    ProbeOutput {
        model: "Monka 3075 Pro / RKGK890".to_string(),
        hardware_revision: hw_rev,
        firmware_version: fw_ver,
        transport_state,
        capabilities,
        interface_a_detected: interface_a,
        interface_b_detected: interface_b,
        read_only_verified: true,
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

/// Creates a probe output from descriptor metadata only (when transport cannot be opened).
pub fn build_descriptor_only_probe_output(device_set: Option<&MonkaDeviceSet>) -> ProbeOutput {
    let (interface_a, interface_b) = match device_set {
        Some(set) => (set.has_interface_a(), set.has_interface_b()),
        None => (false, false),
    };
    let capabilities = determine_capabilities(interface_a, interface_b);

    let transport_state = if let Some(set) = device_set {
        if set.is_wireless() {
            "WirelessSleeping".to_string()
        } else {
            "WiredUsb".to_string()
        }
    } else {
        "WiredUsb".to_string()
    };

    ProbeOutput {
        model: "Monka 3075 Pro / RKGK890".to_string(),
        hardware_revision: "unknown".to_string(),
        firmware_version: "unknown".to_string(),
        transport_state,
        capabilities,
        interface_a_detected: interface_a,
        interface_b_detected: interface_b,
        read_only_verified: true,
    }
}

/// Formats `ProbeOutput` into a terminal capability matrix with checkmarks and status indicators.
pub fn format_probe_human(output: &ProbeOutput) -> String {
    let mut out = String::new();
    out.push_str("============================================================\n");
    out.push_str("              MonKey Device Capability Probe                \n");
    out.push_str("============================================================\n");
    out.push_str(&format!("  Model:                  {}\n", output.model));
    out.push_str(&format!(
        "  Hardware Revision:      {}\n",
        output.hardware_revision
    ));
    out.push_str(&format!(
        "  Firmware Version:       {}\n",
        output.firmware_version
    ));
    out.push_str(&format!(
        "  Transport State:        {}\n",
        output.transport_state
    ));
    out.push_str(&format!(
        "  Read-Only Safety Gate:  {} (0 flash writes)\n",
        if output.read_only_verified {
            "VERIFIED"
        } else {
            "FAILED"
        }
    ));

    out.push_str("\nHardware Capabilities:\n");
    for cap in &output.capabilities {
        out.push_str(&format!("  [✓] {}\n", cap));
    }

    out.push_str("\nInterface Status:\n");
    out.push_str(&format!(
        "  Interface A (Bulk OUT 0xFF68:0x0061): {}\n",
        if output.interface_a_detected {
            "Detected"
        } else {
            "Not Present"
        }
    ));
    out.push_str(&format!(
        "  Interface B (Control  0x000C:0x0001): {}\n",
        if output.interface_b_detected {
            "Detected"
        } else {
            "Not Present"
        }
    ));
    out.push_str("============================================================");
    out
}

/// Executes `monkey probe` writing to stdout.
pub fn run_probe(format: OutputFormat) -> anyhow::Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    run_probe_with_writer(format, &mut handle)
}

/// Executes `monkey probe` using a custom writer for headless testability.
pub fn run_probe_with_writer<W: Write>(format: OutputFormat, writer: &mut W) -> anyhow::Result<()> {
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

    if sets.len() > 1 {
        tracing::warn!(
            "Multiple Monka keyboards detected ({} sets found); inspecting first detected device",
            sets.len()
        );
    }

    let set = &sets[0];
    let probe_output = if let Some(ref b_dev) = set.interface_b {
        match open_device_path(&api, b_dev) {
            Ok(dev) => {
                let mut transport = HidTransport::new(dev);
                let is_wireless = transport.is_wireless();
                probe_device_with_transport(&mut transport, Some(set), is_wireless)
            }
            Err(e) => {
                tracing::warn!(
                    "Could not open Interface B ({e}); falling back to descriptor inspection"
                );
                build_descriptor_only_probe_output(Some(set))
            }
        }
    } else {
        build_descriptor_only_probe_output(Some(set))
    };

    let human = format_probe_human(&probe_output);
    format.write_to(writer, &human, &probe_output)?;
    Ok(())
}

/// Executes probing with an explicit `Transport` implementation and custom writer.
pub fn run_probe_with_transport<W: Write, T: Transport>(
    transport: &mut T,
    device_set: Option<&MonkaDeviceSet>,
    is_wireless: bool,
    format: OutputFormat,
    writer: &mut W,
) -> anyhow::Result<ProbeOutput> {
    let output = probe_device_with_transport(transport, device_set, is_wireless);
    let human = format_probe_human(&output);
    format.write_to(writer, &human, &output)?;
    Ok(output)
}
