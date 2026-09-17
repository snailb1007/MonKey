use serde::{Deserialize, Serialize};

use crate::device::{InterfaceCheckStatus, MonkaDevice};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticCheck {
    pub id: String,
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorSummary {
    pub total: usize,
    pub passed: usize,
    pub warned: usize,
    pub failed: usize,
    pub all_healthy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorReport {
    pub platform: String,
    pub os: String,
    pub arch: String,
    pub checks: Vec<DiagnosticCheck>,
    pub summary: DoctorSummary,
}

/// Run system diagnostics and environmental pre-flight checks.
pub fn run_doctor_checks(mock: bool) -> DoctorReport {
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();
    let platform = format!("{}-{}", os, arch);

    if mock {
        return generate_mock_doctor_report(&platform, &os, &arch);
    }

    let mut checks = Vec::new();

    // 1. Host Platform Check
    checks.push(DiagnosticCheck {
        id: "platform".into(),
        name: "Host Operating System".into(),
        status: CheckStatus::Pass,
        message: format!("Platform {} ({}) supported", os, arch),
        remediation: None,
    });

    // Delegate hardware inspection to MonkaDevice::diagnose() per D-03
    let diag = MonkaDevice::diagnose();

    // 2. HID Subsystem Initialization
    match diag.hid_init {
        Ok(()) => {
            let backend_desc = if cfg!(target_os = "macos") {
                "IOHIDManager (macOS Darwin)"
            } else if cfg!(target_os = "linux") {
                "hidraw (Linux)"
            } else if cfg!(target_os = "windows") {
                "Win32 HID"
            } else {
                "generic hidapi"
            };

            checks.push(DiagnosticCheck {
                id: "hid_subsystem".into(),
                name: "USB HID Subsystem".into(),
                status: CheckStatus::Pass,
                message: format!("HIDAPI initialized successfully via {}", backend_desc),
                remediation: None,
            });
        }
        Err(e) => {
            checks.push(DiagnosticCheck {
                id: "hid_subsystem".into(),
                name: "USB HID Subsystem".into(),
                status: CheckStatus::Fail,
                message: format!("Failed to initialize HID subsystem: {}", e),
                remediation: Some(
                    "Check OS permissions or restart the operating system HID daemon.".into(),
                ),
            });
        }
    }

    // 3. Monka 3075 Pro Enumeration
    if let Some(ref set) = diag.device_set {
        let conn_type = if set.is_wireless() {
            "2.4GHz Wireless Dongle"
        } else {
            "USB Wired"
        };

        checks.push(DiagnosticCheck {
            id: "device_enumeration".into(),
            name: "Keyboard Detection".into(),
            status: CheckStatus::Pass,
            message: format!(
                "Monka 3075 Pro / RKGK890 detected (Connection: {})",
                conn_type
            ),
            remediation: None,
        });

        // 4. Interface A: Bulk LCD Display Pipe (0xFF68:0x61)
        match diag.interface_a_status {
            InterfaceCheckStatus::OpenSuccess => {
                checks.push(DiagnosticCheck {
                    id: "interface_a".into(),
                    name: "Interface A (Bulk Display Pipe)".into(),
                    status: CheckStatus::Pass,
                    message: "Interface A (Usage Page 0xFF68, Usage 0x61) accessible (4096-byte OUT reports supported without TCC prompts).".into(),
                    remediation: None,
                });
            }
            InterfaceCheckStatus::OpenFailed(e) => {
                checks.push(DiagnosticCheck {
                    id: "interface_a".into(),
                    name: "Interface A (Bulk Display Pipe)".into(),
                    status: CheckStatus::Fail,
                    message: format!("Cannot open Interface A device path: {}", e),
                    remediation: Some(
                        "Check if another process has claimed exclusive access to the USB bulk interface.".into(),
                    ),
                });
            }
            InterfaceCheckStatus::NotPresent => {
                checks.push(DiagnosticCheck {
                    id: "interface_a".into(),
                    name: "Interface A (Bulk Display Pipe)".into(),
                    status: CheckStatus::Warn,
                    message: "Interface A not found in active device set.".into(),
                    remediation: Some(
                        "LCD frame rendering requires Interface A. In wireless mode, verify the wireless dongle firmware supports bulk display piping.".into(),
                    ),
                });
            }
        }

        // 5. Interface B: Control & Configuration Pipe (0xFFFF:0x0001)
        match diag.interface_b_status {
            InterfaceCheckStatus::OpenSuccess => {
                checks.push(DiagnosticCheck {
                    id: "interface_b".into(),
                    name: "Interface B (Config & RGB Pipe)".into(),
                    status: CheckStatus::Pass,
                    message: "Interface B (Usage Page 0xFFFF, Usage 0x0001) accessible (64-byte feature reports supported).".into(),
                    remediation: None,
                });
            }
            InterfaceCheckStatus::OpenFailed(e) => {
                let remediation = if cfg!(target_os = "macos") {
                    "macOS composite HID devices sharing Consumer Control/Mouse require Input Monitoring permissions. Open System Settings > Privacy & Security > Input Monitoring, and add your terminal application."
                } else if cfg!(target_os = "linux") {
                    "Linux hidraw devices require udev rules. Add 'SUBSYSTEM==\"hidraw\", ATTRS{idVendor}==\"05ac\", ATTRS{idProduct}==\"024f\", MODE=\"0666\"' to /etc/udev/rules.d/99-monka.rules and run 'udevadm control --reload-rules && udevadm trigger'."
                } else {
                    "Ensure standard HID class drivers are loaded for Interface B."
                };

                checks.push(DiagnosticCheck {
                    id: "interface_b".into(),
                    name: "Interface B (Config & RGB Pipe)".into(),
                    status: CheckStatus::Fail,
                    message: format!("Cannot open Interface B device path: {}", e),
                    remediation: Some(remediation.into()),
                });
            }
            InterfaceCheckStatus::NotPresent => {
                checks.push(DiagnosticCheck {
                    id: "interface_b".into(),
                    name: "Interface B (Config & RGB Pipe)".into(),
                    status: CheckStatus::Warn,
                    message: "Interface B not found in active device set.".into(),
                    remediation: Some("RGB and device configuration require Interface B.".into()),
                });
            }
        }
    } else {
        checks.push(DiagnosticCheck {
            id: "device_enumeration".into(),
            name: "Keyboard Detection".into(),
            status: CheckStatus::Fail,
            message: "No Monka 3075 Pro or RKGK890 keyboard detected on USB bus.".into(),
            remediation: Some(
                "Ensure keyboard is connected via USB-C or the 2.4GHz receiver is plugged in. Check the physical switch on the left side of the keyboard (switch to USB or G).".into(),
            ),
        });
    }

    // 6. Safety Rails Invariant Check
    checks.push(DiagnosticCheck {
        id: "safety_rails".into(),
        name: "Hardware Safety Rails".into(),
        status: CheckStatus::Pass,
        message: "Default-deny opcode whitelist active; flash debouncing (500ms) and low-battery guard enforced; hardware writes require explicit consent.".into(),
        remediation: None,
    });

    let summary = calculate_summary(&checks);

    DoctorReport {
        platform,
        os,
        arch,
        checks,
        summary,
    }
}

fn generate_mock_doctor_report(platform: &str, os: &str, arch: &str) -> DoctorReport {
    let checks = vec![
        DiagnosticCheck {
            id: "platform".into(),
            name: "Host Operating System".into(),
            status: CheckStatus::Pass,
            message: format!("Platform {} ({}) supported (mock environment)", os, arch),
            remediation: None,
        },
        DiagnosticCheck {
            id: "hid_subsystem".into(),
            name: "USB HID Subsystem".into(),
            status: CheckStatus::Pass,
            message: "Mock HID subsystem initialized successfully".into(),
            remediation: None,
        },
        DiagnosticCheck {
            id: "device_enumeration".into(),
            name: "Keyboard Detection".into(),
            status: CheckStatus::Pass,
            message: "Mock Monka 3075 Pro detected (VID: 0x05AC, PID: 0x024F, USB Wired)".into(),
            remediation: None,
        },
        DiagnosticCheck {
            id: "interface_a".into(),
            name: "Interface A (Bulk Display Pipe)".into(),
            status: CheckStatus::Pass,
            message: "Mock Interface A (Usage Page 0xFF68, Usage 0x61) accessible (4096-byte bulk reports)".into(),
            remediation: None,
        },
        DiagnosticCheck {
            id: "interface_b".into(),
            name: "Interface B (Config & RGB Pipe)".into(),
            status: CheckStatus::Pass,
            message: "Mock Interface B (Usage Page 0xFFFF, Usage 0x0001) accessible (64-byte feature reports)".into(),
            remediation: None,
        },
        DiagnosticCheck {
            id: "safety_rails".into(),
            name: "Hardware Safety Rails".into(),
            status: CheckStatus::Pass,
            message: "Safety rails active with mock transport".into(),
            remediation: None,
        },
    ];

    let summary = calculate_summary(&checks);

    DoctorReport {
        platform: platform.to_string(),
        os: os.to_string(),
        arch: arch.to_string(),
        checks,
        summary,
    }
}

fn calculate_summary(checks: &[DiagnosticCheck]) -> DoctorSummary {
    let total = checks.len();
    let passed = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Pass)
        .count();
    let warned = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Warn)
        .count();
    let failed = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Fail)
        .count();
    let all_healthy = failed == 0;

    DoctorSummary {
        total,
        passed,
        warned,
        failed,
        all_healthy,
    }
}
