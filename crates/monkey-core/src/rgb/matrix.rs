use crate::error::{MonkeyError, Result};
use serde::{Deserialize, Serialize};

const DEFAULT_LAYOUT_JSON: &str = include_str!("../../data/layout_81keys.json");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyPosition {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyDefinition {
    pub name: String,
    pub desc: String,
    pub hid_scancode_hex: String,
    pub hid_scancode_dec: u16,
    pub matrix_key_index: u16,
    pub rgb_light_index: u16,
    pub fn_layer_disable: u8,
    pub position: Option<KeyPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LayoutContainer {
    pub layouts: Vec<LayoutItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LayoutItem {
    pub name: String,
    pub identify: String,
    pub total_keys: usize,
    pub keys: Vec<KeyDefinition>,
}

/// 81-Key Keyboard Layout & LED Matrix representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyMatrix {
    pub name: String,
    pub identify: String,
    pub total_keys: usize,
    pub keys: Vec<KeyDefinition>,
}

impl KeyMatrix {
    /// Loads the default 81-key matrix from embedded `layout_81keys.json`.
    pub fn load_default() -> Result<Self> {
        Self::from_json_str(DEFAULT_LAYOUT_JSON)
    }

    /// Parses a KeyMatrix from a JSON string.
    pub fn from_json_str(json: &str) -> Result<Self> {
        let container: LayoutContainer = serde_json::from_str(json)
            .map_err(|e| MonkeyError::Protocol(format!("Failed to parse layout JSON: {e}")))?;

        let layout =
            container.layouts.into_iter().next().ok_or_else(|| {
                MonkeyError::Protocol("Layout JSON contains no layouts".to_string())
            })?;

        Ok(Self {
            name: layout.name,
            identify: layout.identify,
            total_keys: layout.total_keys,
            keys: layout.keys,
        })
    }

    /// Finds a key definition by its case-insensitive name/label.
    pub fn lookup_by_name(&self, name: &str) -> Option<&KeyDefinition> {
        let target = name.trim().to_lowercase();
        self.keys
            .iter()
            .find(|k| k.name.trim().to_lowercase() == target)
    }

    /// Finds a key definition by its physical RGB LED index (0..80).
    pub fn lookup_by_led_index(&self, index: u16) -> Option<&KeyDefinition> {
        self.keys.iter().find(|k| k.rgb_light_index == index)
    }

    /// Finds a key definition by its HID scancode.
    pub fn lookup_by_scancode(&self, scancode: u16) -> Option<&KeyDefinition> {
        self.keys.iter().find(|k| k.hid_scancode_dec == scancode)
    }
}
