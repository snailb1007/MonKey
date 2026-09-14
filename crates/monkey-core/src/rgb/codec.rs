use crate::error::{MonkeyError, Result};
use crate::protocol::types::{
    CommandId, FeatureReportPacket, FEATURE_REPORT_MAGIC, FEATURE_REPORT_MARKER,
    FEATURE_REPORT_SIZE,
};
use crate::rgb::mode::{FlowDirection, LightingConfig, LightingMode, RgbColor};
use zerocopy::FromBytes;

/// Normalizes 0..=100 scale to 1..=5 hardware scale.
pub fn normalize_to_hardware_scale(val: u8) -> u8 {
    if val == 0 {
        1
    } else {
        ((val as u16 * 4).div_ceil(100)).clamp(1, 5) as u8
    }
}

/// Denormalizes 1..=5 hardware scale to 0..=100 scale.
pub fn denormalize_from_hardware_scale(val: u8) -> u8 {
    match val {
        0 | 1 => 20,
        2 => 40,
        3 => 60,
        4 => 80,
        5.. => 100,
    }
}

/// Encodes `LightingConfig` into a 64-byte `FeatureReportPacket`.
pub fn encode_rgb_control_packet(config: &LightingConfig) -> Result<FeatureReportPacket> {
    config.validate()?;

    let mut args = [0u8; 6];
    args[0] = config.mode.to_byte();
    args[1] = config.color.r;
    args[2] = config.color.g;
    args[3] = config.color.b;
    args[4] = 0;
    args[5] = 0;

    let colortype = if config.mode == LightingMode::Rainbow || config.mode == LightingMode::Wave {
        1u8
    } else {
        0u8
    };

    let mut reserved = [0u8; 5];
    reserved[0] = normalize_to_hardware_scale(config.speed);
    reserved[1] = normalize_to_hardware_scale(config.brightness);
    reserved[2] = match config.direction {
        FlowDirection::LeftToRight => 0,
        FlowDirection::RightToLeft => 1,
    };
    reserved[3] = 0;
    reserved[4] = 0;

    Ok(FeatureReportPacket {
        magic: FEATURE_REPORT_MAGIC,
        command: CommandId::RgbControl as u8,
        args,
        chunk_index: colortype,
        reserved,
        marker: FEATURE_REPORT_MARKER,
        payload: [0u8; 48],
    })
}

/// Decodes a 64-byte slice into `LightingConfig`.
pub fn decode_rgb_control_packet(data: &[u8]) -> Result<LightingConfig> {
    if data.len() < FEATURE_REPORT_SIZE {
        return Err(MonkeyError::Protocol(format!(
            "Buffer too short for RGB control packet: {} < {}",
            data.len(),
            FEATURE_REPORT_SIZE
        )));
    }

    let packet = FeatureReportPacket::ref_from_bytes(&data[..FEATURE_REPORT_SIZE])
        .map_err(|e| MonkeyError::Protocol(format!("Failed to parse FeatureReportPacket: {e}")))?;

    if packet.magic != FEATURE_REPORT_MAGIC {
        return Err(MonkeyError::Protocol(format!(
            "Invalid magic byte 0x{:02X}, expected 0x{:02X}",
            packet.magic, FEATURE_REPORT_MAGIC
        )));
    }

    if packet.command != CommandId::RgbControl as u8 {
        return Err(MonkeyError::Protocol(format!(
            "Invalid command byte 0x{:02X}, expected 0x{:02X}",
            packet.command, CommandId::RgbControl as u8
        )));
    }

    if packet.marker != FEATURE_REPORT_MARKER {
        return Err(MonkeyError::Protocol(format!(
            "Invalid marker {:?}, expected {:?}",
            packet.marker, FEATURE_REPORT_MARKER
        )));
    }

    let mode = LightingMode::from_byte(packet.args[0])?;
    let color = RgbColor::new(packet.args[1], packet.args[2], packet.args[3]);
    let speed = denormalize_from_hardware_scale(packet.reserved[0]);
    let brightness = denormalize_from_hardware_scale(packet.reserved[1]);
    let direction = if packet.reserved[2] == 1 {
        FlowDirection::RightToLeft
    } else {
        FlowDirection::LeftToRight
    };

    Ok(LightingConfig {
        mode,
        color,
        brightness,
        speed,
        direction,
    })
}
