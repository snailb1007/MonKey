pub mod codec;
pub mod manager;
pub mod matrix;
pub mod mode;
pub mod profile;

pub use codec::{decode_rgb_control_packet, encode_rgb_control_packet};
pub use manager::RgbManager;
pub use matrix::{KeyDefinition, KeyMatrix, KeyPosition};
pub use mode::{FlowDirection, LightingConfig, LightingMode, RgbColor};
pub use profile::{RgbProfile, CURRENT_PROFILE_SCHEMA_VERSION};
