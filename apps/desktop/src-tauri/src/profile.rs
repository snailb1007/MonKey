use monkey_core::rgb::{RgbProfile, CURRENT_PROFILE_SCHEMA_VERSION};

use crate::dto::LightingConfigDto;
use crate::error::{DesktopError, DesktopResult};

pub fn export_profile_json(
    model: &str,
    config: &LightingConfigDto,
    description: Option<&str>,
) -> DesktopResult<String> {
    config.validate_for_preview()?;
    let core_config = config.to_core_config()?;
    let profile = RgbProfile::new(model, core_config, description.map(|s| s.to_string()));
    profile
        .to_json()
        .map_err(|e| DesktopError::InvalidInput(format!("Failed to serialize profile: {e}")))
}

pub fn import_profile_json(json_str: &str) -> DesktopResult<LightingConfigDto> {
    // Parse using core RgbProfile to validate schema version and structure
    let profile = RgbProfile::from_json(json_str)
        .map_err(|e| DesktopError::InvalidInput(format!("Invalid RGB profile JSON: {e}")))?;

    if profile.schema_version > CURRENT_PROFILE_SCHEMA_VERSION {
        return Err(DesktopError::InvalidInput(format!(
            "Unsupported profile schema version {} (max supported: {})",
            profile.schema_version, CURRENT_PROFILE_SCHEMA_VERSION
        )));
    }

    let config_dto = LightingConfigDto::from_core_config(&profile.lighting);
    config_dto.validate_for_preview()?;
    Ok(config_dto)
}
