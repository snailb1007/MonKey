use std::fmt;

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_GENERAL: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_NO_DEVICE: i32 = 3;
pub const EXIT_PERMISSION: i32 = 4;
pub const EXIT_BLOCKED: i32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success,
    General,
    Usage,
    NoDevice,
    Permission,
    Blocked,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        match self {
            ExitCode::Success => EXIT_SUCCESS,
            ExitCode::General => EXIT_GENERAL,
            ExitCode::Usage => EXIT_USAGE,
            ExitCode::NoDevice => EXIT_NO_DEVICE,
            ExitCode::Permission => EXIT_PERMISSION,
            ExitCode::Blocked => EXIT_BLOCKED,
        }
    }

    pub fn category_name(self) -> &'static str {
        match self {
            ExitCode::Success => "Success",
            ExitCode::General => "Internal / General Error",
            ExitCode::Usage => "Command Usage / Invalid Arguments",
            ExitCode::NoDevice => "Hardware Device Not Found",
            ExitCode::Permission => "OS Permission Denied",
            ExitCode::Blocked => "Hardware Safety Gate Blocked",
        }
    }
}

impl fmt::Display for ExitCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (exit code {})", self.category_name(), self.as_i32())
    }
}

/// Classifies any `anyhow::Error` into a standardized POSIX ExitCode by inspecting the error chain.
pub fn classify_error(err: &anyhow::Error) -> ExitCode {
    for cause in err.chain() {
        let msg = cause.to_string();

        // 1. Device missing / disconnected checks
        if msg.contains("No Monka 3075 Pro")
            || msg.contains("No valid HID interface detected")
            || msg.contains("DeviceNotFound")
            || msg.contains("device not found")
        {
            return ExitCode::NoDevice;
        }

        // 2. Hardware safety / write consent checks
        if msg.contains("Hardware writes require explicit consent")
            || msg.contains("Flash write throttled")
            || msg.contains("Battery level is below 20%")
            || msg.contains("Safety violation")
            || msg.contains("OpcodeNotPermitted")
        {
            return ExitCode::Blocked;
        }

        // 3. Permission denied / OS security checks
        if msg.contains("Permission denied")
            || msg.contains("Input Monitoring")
            || msg.contains("exclusive access")
            || msg.contains("kIOReturnExclusiveAccess")
        {
            return ExitCode::Permission;
        }

        // 4. Invalid arguments / usage errors
        if msg.contains("Invalid lighting mode")
            || msg.contains("Invalid RGB hex color")
            || msg.contains("Invalid flow direction")
            || msg.contains("Invalid image format")
            || msg.contains("Invalid test pattern")
            || msg.contains("validation")
        {
            return ExitCode::Usage;
        }
    }

    ExitCode::General
}
