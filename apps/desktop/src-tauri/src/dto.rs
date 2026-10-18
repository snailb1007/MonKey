use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopAction {
    ListDevices,
    Probe,
    Diagnostics,
    PickAsset,
    StartLcd,
    StopLcd,
    JobStatus,
    RgbStatus,
    RgbPreview,
    RgbCommit,
    ExportProfile,
    ImportProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NotConnected,
    PermissionDenied,
    Unsupported,
    Busy,
    UnsafeOperation,
    LowBattery,
    InvalidInput,
    IoError,
    Cancelled,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopRequestEnvelope {
    pub version: u32,
    pub request_id: String,
    pub device_id: Option<String>,
    pub action: DesktopAction,
    pub payload: serde_json::Value,
}

impl DesktopRequestEnvelope {
    pub fn validate_envelope(&self) -> Result<(), crate::error::DesktopError> {
        if self.version != 1 {
            return Err(crate::error::DesktopError::InvalidInput(format!(
                "Unsupported request version: {}. Only version 1 is supported",
                self.version
            )));
        }
        let req_id = self.request_id.trim();
        if req_id.is_empty() {
            return Err(crate::error::DesktopError::InvalidInput(
                "Request ID cannot be empty".to_string(),
            ));
        }
        if req_id.len() > 128 {
            return Err(crate::error::DesktopError::InvalidInput(
                "Request ID exceeds maximum length of 128 characters".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSuccessResponse<T> {
    pub version: u32,
    pub request_id: String,
    pub ok: bool,
    pub data: T,
}

impl<T> DesktopSuccessResponse<T> {
    pub fn new(request_id: String, data: T) -> Self {
        Self {
            version: 1,
            request_id,
            ok: true,
            data,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopErrorResponse {
    pub version: u32,
    pub request_id: String,
    pub ok: bool,
    pub error: DesktopErrorPayload,
}

impl DesktopErrorResponse {
    pub fn new(request_id: String, error: DesktopErrorPayload) -> Self {
        Self {
            version: 1,
            request_id,
            ok: false,
            error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopResponseEnvelope {
    pub version: u32,
    pub request_id: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<DesktopErrorPayload>,
}

impl DesktopResponseEnvelope {
    pub fn ok(request_id: &str, data: serde_json::Value) -> Self {
        Self {
            version: 1,
            request_id: request_id.to_string(),
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(request_id: &str, error: crate::error::DesktopError) -> Self {
        Self {
            version: 1,
            request_id: request_id.to_string(),
            ok: false,
            data: None,
            error: Some(error.to_payload()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopErrorPayload {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
}

// --- Device DTOs ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionType {
    Wired,
    Wireless,
    Asleep,
    Disconnected,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterfacesDto {
    pub lcd: bool,
    pub config: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilitiesDto {
    pub lcd: bool,
    pub rgb: bool,
    pub persist: bool,
    pub remap: bool,
    pub macro_: bool,
    pub firmware: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceSnapshot {
    pub id: String,
    pub label: String,
    pub connection: ConnectionType,
    pub interfaces: InterfacesDto,
    pub capabilities: CapabilitiesDto,
    pub battery_percent: Option<u8>,
    pub firmware: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDevicesResult {
    pub devices: Vec<DeviceSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub device: DeviceSnapshot,
}

// --- Diagnostics DTOs ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStatusDto {
    Pass,
    Warn,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticCheckDto {
    pub id: String,
    pub label: String,
    pub status: DiagnosticStatusDto,
    pub detail: String,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsResult {
    pub checks: Vec<DiagnosticCheckDto>,
}

// --- Asset DTOs ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Image,
    Animation,
    Sequence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PickAssetPayload {
    pub kind: AssetKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetPreview {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub frame_count: usize,
    pub preview_data_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PickAssetResult {
    pub asset: Option<AssetPreview>,
}

// --- LCD DTOs ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticPattern {
    Red,
    Green,
    Blue,
    White,
    Black,
    Grayscale,
    ColorBars,
    Geometry,
}

impl DiagnosticPattern {
    pub fn to_core_pattern(self) -> monkey_core::lcd::TestPatternType {
        match self {
            Self::Red => monkey_core::lcd::TestPatternType::Red,
            Self::Green => monkey_core::lcd::TestPatternType::Green,
            Self::Blue => monkey_core::lcd::TestPatternType::Blue,
            Self::White => monkey_core::lcd::TestPatternType::White,
            Self::Black => monkey_core::lcd::TestPatternType::Black,
            Self::Grayscale => monkey_core::lcd::TestPatternType::Grayscale,
            Self::ColorBars => monkey_core::lcd::TestPatternType::RgbBars,
            Self::Geometry => monkey_core::lcd::TestPatternType::Geometry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartLcdPayload {
    pub asset_id: Option<String>,
    pub pattern: Option<DiagnosticPattern>,
}

impl StartLcdPayload {
    pub fn validate(&self) -> Result<(), crate::error::DesktopError> {
        match (&self.asset_id, &self.pattern) {
            (Some(_), None) | (None, Some(_)) => Ok(()),
            _ => Err(crate::error::DesktopError::InvalidInput(
                "Exactly one of 'assetId' or 'pattern' must be provided".to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartLcdResult {
    pub job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StopLcdPayload {
    pub job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopLcdResult {
    pub stopped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobSnapshot {
    pub id: String,
    pub device_id: String,
    pub state: JobState,
    pub frames_sent: usize,
    pub error: Option<DesktopErrorPayload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobStatusPayload {
    pub job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStatusResult {
    pub job: JobSnapshot,
}

// --- RGB Lighting DTOs ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ColorDto {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LightingConfigDto {
    pub mode: String,
    pub color: ColorDto,
    pub brightness: u8,
    pub speed: u8,
    pub direction: Option<String>,
}

impl LightingConfigDto {
    pub const SUPPORTED_MODES: &'static [&'static str] = &[
        "off",
        "static",
        "breathing",
        "wave",
        "rainbow",
        "ripple",
        "reactive",
    ];

    pub fn validate_for_preview(&self) -> Result<(), crate::error::DesktopError> {
        if !Self::SUPPORTED_MODES.contains(&self.mode.as_str()) {
            return Err(crate::error::DesktopError::InvalidInput(format!(
                "Unsupported lighting mode '{}'. Supported modes: {:?}",
                self.mode,
                Self::SUPPORTED_MODES
            )));
        }
        if self.brightness > 100 {
            return Err(crate::error::DesktopError::InvalidInput(format!(
                "Brightness {} exceeds maximum 100",
                self.brightness
            )));
        }
        if self.speed > 100 {
            return Err(crate::error::DesktopError::InvalidInput(format!(
                "Speed {} exceeds maximum 100",
                self.speed
            )));
        }
        match self.direction.as_deref() {
            Some("left_to_right") | Some("right_to_left") => Ok(()),
            Some(other) => Err(crate::error::DesktopError::InvalidInput(format!(
                "Invalid direction '{}'. Supported: left_to_right, right_to_left",
                other
            ))),
            None if matches!(self.mode.as_str(), "off" | "static") => Ok(()),
            None => Err(crate::error::DesktopError::InvalidInput(
                "Direction is required for RGB preview and cannot be null".to_string(),
            )),
        }
    }

    pub fn to_core_config(
        &self,
    ) -> Result<monkey_core::rgb::LightingConfig, crate::error::DesktopError> {
        use std::str::FromStr;
        let mode = monkey_core::rgb::LightingMode::from_str(&self.mode)
            .map_err(|e| crate::error::DesktopError::InvalidInput(e.to_string()))?;
        let color = monkey_core::rgb::RgbColor::new(self.color.r, self.color.g, self.color.b);
        let direction = match self.direction.as_deref() {
            Some("right_to_left") => monkey_core::rgb::FlowDirection::RightToLeft,
            _ => monkey_core::rgb::FlowDirection::LeftToRight,
        };
        Ok(monkey_core::rgb::LightingConfig {
            mode,
            color,
            brightness: self.brightness,
            speed: self.speed,
            direction,
        })
    }

    pub fn from_core_config(config: &monkey_core::rgb::LightingConfig) -> Self {
        let mode_str = match config.mode {
            monkey_core::rgb::LightingMode::Off => "off",
            monkey_core::rgb::LightingMode::Static => "static",
            monkey_core::rgb::LightingMode::Breathing => "breathing",
            monkey_core::rgb::LightingMode::Wave => "wave",
            monkey_core::rgb::LightingMode::Rainbow => "rainbow",
            monkey_core::rgb::LightingMode::Ripple => "ripple",
            monkey_core::rgb::LightingMode::Reactive => "reactive",
        };
        let dir_str = match config.direction {
            monkey_core::rgb::FlowDirection::LeftToRight => "left_to_right",
            monkey_core::rgb::FlowDirection::RightToLeft => "right_to_left",
        };
        Self {
            mode: mode_str.to_string(),
            color: ColorDto {
                r: config.color.r,
                g: config.color.g,
                b: config.color.b,
            },
            brightness: config.brightness,
            speed: config.speed,
            direction: Some(dir_str.to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceState {
    Unknown,
    Ram,
    Verified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RgbStatusResult {
    pub config: Option<LightingConfigDto>,
    pub persistence: PersistenceState,
    pub supported_modes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RgbPreviewPayload {
    pub config: LightingConfigDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RgbPreviewResult {
    pub config: LightingConfigDto,
    pub persistence: PersistenceState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RgbCommitResult {
    pub persistence: PersistenceState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportProfilePayload {
    pub config: LightingConfigDto,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProfileResult {
    pub profile_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportProfilePayload {
    pub profile_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProfileResult {
    pub config: LightingConfigDto,
}
