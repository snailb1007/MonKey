# Plan 04-01 Summary: Ambient RGB Lighting Data Types, Packet Codecs & 81-Key Matrix Model

## Completed Work
1. **Lighting Primitives (`crates/monkey-core/src/rgb/mode.rs`)**:
   - Implemented `LightingMode` enum covering `Static`, `Breathing`, `Wave`, `Rainbow`, `Ripple`, `Reactive`, `Off` with byte conversion and case-insensitive string parsing.
   - Implemented `RgbColor` with constructors, hex parser (`#RRGGBB` and `RRGGBB`), named color aliases (`red`, `green`, `blue`, `yellow`, `cyan`, `magenta`, `white`, `black`/`off`, `orange`, `purple`), and hex serializer.
   - Implemented `LightingConfig` with range validation for brightness (0..=100) and speed (0..=100).
2. **RGB Packet Codec (`crates/monkey-core/src/rgb/codec.rs`)**:
   - Implemented `encode_rgb_control_packet` generating 64-byte `FeatureReportPacket` with opcode `0x13` (`CommandId::RgbControl`), hardware scale normalization (1..=5), and `[0xAA, 0x55]` marker.
   - Implemented `decode_rgb_control_packet` parsing 64-byte wire reports into `LightingConfig`.
3. **81-Key Matrix Model (`crates/monkey-core/src/rgb/matrix.rs`)**:
   - Implemented `KeyMatrix` embedding `research/layout_81keys.json`.
   - Provided lookup methods by key name, HID scancode, and physical RGB LED index (0..80).
4. **Testing (`crates/monkey-core/tests/rgb_codecs_test.rs`)**:
   - 4 tests passing validating color parsing, mode parsing, packet wire layout, and matrix key lookups.

## Verification
- `cargo test -p monkey-core --test rgb_codecs_test` passes (4/4 tests).
