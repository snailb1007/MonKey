use monkey_core::rgb::{
    decode_rgb_control_packet, encode_rgb_control_packet, FlowDirection, KeyMatrix, LightingConfig,
    LightingMode, RgbColor,
};
use monkey_core::protocol::types::{
    CommandId, FEATURE_REPORT_MAGIC, FEATURE_REPORT_MARKER, FEATURE_REPORT_SIZE,
};

#[test]
fn test_rgb_color_parsing() {
    // Hex with hash
    let red = RgbColor::parse("#FF0000").expect("valid hex");
    assert_eq!(red, RgbColor::new(255, 0, 0));

    // Hex without hash
    let green = RgbColor::parse("00FF00").expect("valid hex");
    assert_eq!(green, RgbColor::new(0, 255, 0));

    // Named colors
    assert_eq!(RgbColor::parse("blue").unwrap(), RgbColor::BLUE);
    assert_eq!(RgbColor::parse("YELLOW").unwrap(), RgbColor::YELLOW);
    assert_eq!(RgbColor::parse("white").unwrap(), RgbColor::WHITE);
    assert_eq!(RgbColor::parse("black").unwrap(), RgbColor::BLACK);
    assert_eq!(RgbColor::parse("off").unwrap(), RgbColor::BLACK);

    // Invalid hex
    assert!(RgbColor::parse("#XYZ").is_err());
    assert!(RgbColor::parse("#12345").is_err());
    assert!(RgbColor::parse("invalid_color_name").is_err());
}

#[test]
fn test_lighting_mode_parsing() {
    assert_eq!("static".parse::<LightingMode>().unwrap(), LightingMode::Static);
    assert_eq!("breathing".parse::<LightingMode>().unwrap(), LightingMode::Breathing);
    assert_eq!("WAVE".parse::<LightingMode>().unwrap(), LightingMode::Wave);
    assert_eq!("rainbow".parse::<LightingMode>().unwrap(), LightingMode::Rainbow);
    assert_eq!("ripple".parse::<LightingMode>().unwrap(), LightingMode::Ripple);
    assert_eq!("reactive".parse::<LightingMode>().unwrap(), LightingMode::Reactive);
    assert_eq!("off".parse::<LightingMode>().unwrap(), LightingMode::Off);

    assert!("unknown_mode".parse::<LightingMode>().is_err());
}

#[test]
fn test_rgb_packet_encoding_and_marker() {
    let config = LightingConfig {
        mode: LightingMode::Static,
        color: RgbColor::new(0x12, 0x34, 0x56),
        brightness: 80,
        speed: 60,
        direction: FlowDirection::LeftToRight,
    };

    let packet = encode_rgb_control_packet(&config).expect("encoding succeeds");
    let bytes = packet.as_bytes();

    assert_eq!(bytes.len(), FEATURE_REPORT_SIZE);
    assert_eq!(bytes[0], FEATURE_REPORT_MAGIC);
    assert_eq!(bytes[1], CommandId::RgbControl as u8);
    assert_eq!(bytes[2], LightingMode::Static.to_byte());
    assert_eq!(bytes[3], 0x12);
    assert_eq!(bytes[4], 0x34);
    assert_eq!(bytes[5], 0x56);
    assert_eq!(bytes[14], FEATURE_REPORT_MARKER[0]);
    assert_eq!(bytes[15], FEATURE_REPORT_MARKER[1]);

    // Test round trip decode
    let decoded = decode_rgb_control_packet(bytes).expect("decoding succeeds");
    assert_eq!(decoded.mode, LightingMode::Static);
    assert_eq!(decoded.color, RgbColor::new(0x12, 0x34, 0x56));
    assert_eq!(decoded.direction, FlowDirection::LeftToRight);
}

#[test]
fn test_matrix_layout_loading_and_lookups() {
    let matrix = KeyMatrix::load_default().expect("loads embedded layout");
    assert_eq!(matrix.total_keys, 81);
    assert_eq!(matrix.keys.len(), 81);

    // Lookup by name
    let esc = matrix.lookup_by_name("esc").expect("esc exists");
    assert_eq!(esc.name.to_lowercase(), "esc");
    assert_eq!(esc.rgb_light_index, 1);
    assert_eq!(esc.hid_scancode_hex, "0x29");

    let space = matrix.lookup_by_name("Space").expect("space exists");
    assert_eq!(space.name.to_lowercase(), "space");

    // Lookup by LED index
    let led_1 = matrix.lookup_by_led_index(1).expect("led 1 exists");
    assert_eq!(led_1.name.to_lowercase(), "esc");

    // Lookup non-existent
    assert!(matrix.lookup_by_name("non_existent_key").is_none());
    assert!(matrix.lookup_by_led_index(999).is_none());
}
