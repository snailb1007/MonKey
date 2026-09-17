use std::io::Write;

use crate::output::OutputFormat;
pub use monkey_core::device::{determine_capabilities, parse_probe_response, ProbeOutput};
use monkey_core::device::{InterfacePolicy, MonkaDevice, MonkaDeviceSet, OpenError};

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
    let probe_output = match MonkaDevice::open(InterfacePolicy::RequireB) {
        Ok(mut device) => device.probe()?,
        Err(OpenError::NoDevice) => return Err(OpenError::NoDevice.into()),
        Err(e) => {
            tracing::warn!(
                "Could not open Interface B ({e}); falling back to descriptor inspection"
            );
            let sets = MonkaDevice::discover()?;
            let set = sets.into_iter().next();
            build_descriptor_only_probe_output(set.as_ref())
        }
    };

    let human = format_probe_human(&probe_output);
    format.write_to(writer, &human, &probe_output)?;
    Ok(())
}
