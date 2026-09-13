use std::process::Command;

#[test]
fn lcd_test_pattern_mock_outputs_eight_chunks() {
    let output = Command::new(env!("CARGO_BIN_EXE_monkey"))
        .args([
            "lcd",
            "test-pattern",
            "red",
            "--mock",
            "--json",
            "--inter-chunk-delay-ms",
            "0",
        ])
        .output()
        .expect("failed to invoke monkey lcd test-pattern");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["operation"], "test-pattern");
    assert_eq!(json["target"], "mock");
    assert_eq!(json["chunks_per_frame"], 8);
    assert_eq!(json["bytes_sent"], 32768);
}

#[test]
fn lcd_hardware_write_requires_explicit_consent() {
    let output = Command::new(env!("CARGO_BIN_EXE_monkey"))
        .args(["lcd", "test-pattern", "geometry"])
        .output()
        .expect("failed to invoke monkey lcd test-pattern");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--allow-hardware-writes"));
}

#[test]
fn lcd_image_mock_accepts_bmp_and_reports_frame() {
    let path = std::env::temp_dir().join(format!("monkey-phase3-{}.bmp", std::process::id()));
    write_2x1_bmp(&path);
    let output = Command::new(env!("CARGO_BIN_EXE_monkey"))
        .args([
            "lcd",
            "image",
            path.to_str().unwrap(),
            "--mock",
            "--json",
            "--inter-chunk-delay-ms",
            "0",
        ])
        .output()
        .expect("failed to invoke monkey lcd image");
    let _ = std::fs::remove_file(path);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["operation"], "image");
    assert_eq!(json["chunks_per_frame"], 8);
    assert_eq!(json["bytes_sent"], 32768);
}

fn write_2x1_bmp(path: &std::path::Path) {
    // 24-bit, bottom-up BMP: 2 pixels plus two bytes of row padding.
    let mut bytes = vec![0u8; 62];
    bytes[0..2].copy_from_slice(b"BM");
    bytes[2..6].copy_from_slice(&(62u32).to_le_bytes());
    bytes[10..14].copy_from_slice(&(54u32).to_le_bytes());
    bytes[14..18].copy_from_slice(&(40u32).to_le_bytes());
    bytes[18..22].copy_from_slice(&(2i32).to_le_bytes());
    bytes[22..26].copy_from_slice(&(1i32).to_le_bytes());
    bytes[26..28].copy_from_slice(&(1u16).to_le_bytes());
    bytes[28..30].copy_from_slice(&(24u16).to_le_bytes());
    bytes[34..38].copy_from_slice(&(8u32).to_le_bytes());
    // BGR red then blue.
    bytes[54..57].copy_from_slice(&[0, 0, 255]);
    bytes[57..60].copy_from_slice(&[255, 0, 0]);
    std::fs::write(path, bytes).unwrap();
}
