use std::str::FromStr;
use serde::{Deserialize, Serialize};
use crate::error::{MonkeyError, Result};

/// Lighting modes supported by the Monka 3075 Pro controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LightingMode {
    Off = 0x00,
    Static = 0x01,
    Breathing = 0x02,
    Wave = 0x03,
    Rainbow = 0x04,
    Ripple = 0x05,
    Reactive = 0x06,
}

impl LightingMode {
    pub fn to_byte(self) -> u8 {
        self as u8
    }

    pub fn from_byte(b: u8) -> Result<Self> {
        match b {
            0x00 => Ok(Self::Off),
            0x01 => Ok(Self::Static),
            0x02 => Ok(Self::Breathing),
            0x03 => Ok(Self::Wave),
            0x04 => Ok(Self::Rainbow),
            0x05 => Ok(Self::Ripple),
            0x06 => Ok(Self::Reactive),
            other => Err(MonkeyError::Protocol(format!(
                "Unknown lighting mode byte: 0x{other:02X}"
            ))),
        }
    }
}

impl FromStr for LightingMode {
    type Err = MonkeyError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "off" | "none" => Ok(Self::Off),
            "static" | "solid" => Ok(Self::Static),
            "breathing" | "breath" => Ok(Self::Breathing),
            "wave" => Ok(Self::Wave),
            "rainbow" | "spectrum" => Ok(Self::Rainbow),
            "ripple" => Ok(Self::Ripple),
            "reactive" => Ok(Self::Reactive),
            other => Err(MonkeyError::Protocol(format!(
                "Invalid lighting mode '{other}'. Supported: static, breathing, wave, rainbow, ripple, reactive, off"
            ))),
        }
    }
}

/// 24-bit RGB Color with hex and named color alias support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const RED: Self = Self { r: 255, g: 0, b: 0 };
    pub const GREEN: Self = Self { r: 0, g: 255, b: 0 };
    pub const BLUE: Self = Self { r: 0, g: 0, b: 255 };
    pub const YELLOW: Self = Self { r: 255, g: 255, b: 0 };
    pub const CYAN: Self = Self { r: 0, g: 255, b: 255 };
    pub const MAGENTA: Self = Self { r: 255, g: 0, b: 255 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255 };
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const ORANGE: Self = Self { r: 255, g: 128, b: 0 };
    pub const PURPLE: Self = Self { r: 128, g: 0, b: 128 };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses color from hex string (`#RRGGBB` or `RRGGBB`) or named aliases.
    pub fn parse(s: &str) -> Result<Self> {
        let trimmed = s.trim().to_lowercase();
        match trimmed.as_str() {
            "red" => return Ok(Self::RED),
            "green" => return Ok(Self::GREEN),
            "blue" => return Ok(Self::BLUE),
            "yellow" => return Ok(Self::YELLOW),
            "cyan" => return Ok(Self::CYAN),
            "magenta" => return Ok(Self::MAGENTA),
            "white" => return Ok(Self::WHITE),
            "off" | "black" => return Ok(Self::BLACK),
            "orange" => return Ok(Self::ORANGE),
            "purple" => return Ok(Self::PURPLE),
            _ => {}
        }

        let hex_str = trimmed.strip_prefix('#').unwrap_or(&trimmed);
        if hex_str.len() != 6 {
            return Err(MonkeyError::Protocol(format!(
                "Invalid color hex '{s}'. Expected 6 hex characters (#RRGGBB) or named color (red, green, blue, etc.)"
            )));
        }

        let r = u8::from_str_radix(&hex_str[0..2], 16).map_err(|_| {
            MonkeyError::Protocol(format!("Invalid red channel in hex '{s}'"))
        })?;
        let g = u8::from_str_radix(&hex_str[2..4], 16).map_err(|_| {
            MonkeyError::Protocol(format!("Invalid green channel in hex '{s}'"))
        })?;
        let b = u8::from_str_radix(&hex_str[4..6], 16).map_err(|_| {
            MonkeyError::Protocol(format!("Invalid blue channel in hex '{s}'"))
        })?;

        Ok(Self { r, g, b })
    }

    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

impl FromStr for RgbColor {
    type Err = MonkeyError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Dynamic wave/effect flow direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowDirection {
    LeftToRight = 0x00,
    RightToLeft = 0x01,
}

/// Complete ambient lighting configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightingConfig {
    pub mode: LightingMode,
    pub color: RgbColor,
    /// Brightness level (0..=100)
    pub brightness: u8,
    /// Speed level (0..=100)
    pub speed: u8,
    pub direction: FlowDirection,
}

impl LightingConfig {
    pub fn new(mode: LightingMode, color: RgbColor, brightness: u8, speed: u8) -> Result<Self> {
        let config = Self {
            mode,
            color,
            brightness,
            speed,
            direction: FlowDirection::LeftToRight,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.brightness > 100 {
            return Err(MonkeyError::Protocol(format!(
                "Brightness {} exceeds maximum 100",
                self.brightness
            )));
        }
        if self.speed > 100 {
            return Err(MonkeyError::Protocol(format!(
                "Speed {} exceeds maximum 100",
                self.speed
            )));
        }
        Ok(())
    }

    pub fn default_static(color: RgbColor) -> Self {
        Self {
            mode: LightingMode::Static,
            color,
            brightness: 100,
            speed: 50,
            direction: FlowDirection::LeftToRight,
        }
    }
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            mode: LightingMode::Wave,
            color: RgbColor::WHITE,
            brightness: 100,
            speed: 50,
            direction: FlowDirection::LeftToRight,
        }
    }
}
