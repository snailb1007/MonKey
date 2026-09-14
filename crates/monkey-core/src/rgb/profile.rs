use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::error::{MonkeyError, Result};
use crate::rgb::mode::LightingConfig;

pub const CURRENT_PROFILE_SCHEMA_VERSION: u32 = 1;

/// Persistent RGB profile structure for saving/restoring keyboard lighting configurations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbProfile {
    pub schema_version: u32,
    pub model: String,
    pub created_at: String,
    pub lighting: LightingConfig,
    pub description: Option<String>,
}

impl RgbProfile {
    pub fn new(model: impl Into<String>, lighting: LightingConfig, description: Option<String>) -> Self {
        Self {
            schema_version: CURRENT_PROFILE_SCHEMA_VERSION,
            model: model.into(),
            created_at: chrono_timestamp(),
            lighting,
            description,
        }
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| MonkeyError::Protocol(format!("Failed to serialize RGB profile: {e}")))
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let profile: Self = serde_json::from_str(json)
            .map_err(|e| MonkeyError::Protocol(format!("Failed to deserialize RGB profile: {e}")))?;

        if profile.schema_version > CURRENT_PROFILE_SCHEMA_VERSION {
            return Err(MonkeyError::Protocol(format!(
                "Unsupported profile schema version {} (max supported: {})",
                profile.schema_version, CURRENT_PROFILE_SCHEMA_VERSION
            )));
        }

        profile.lighting.validate()?;
        Ok(profile)
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let json = self.to_json()?;
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                MonkeyError::Protocol(format!("Failed to create directory {:?}: {e}", parent))
            })?;
        }
        fs::write(p, json).map_err(|e| {
            MonkeyError::Protocol(format!("Failed to write profile to {:?}: {e}", p))
        })?;
        Ok(())
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let p = path.as_ref();
        let content = fs::read_to_string(p).map_err(|e| {
            MonkeyError::Protocol(format!("Failed to read profile file {:?}: {e}", p))
        })?;
        Self::from_json(&content)
    }
}

fn chrono_timestamp() -> String {
    // Simple timestamp representation without external chrono dependency
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}
