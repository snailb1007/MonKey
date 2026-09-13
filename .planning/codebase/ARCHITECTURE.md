# MonKey Architecture

## System Overview

- **Layered Decoupling**: UI/CLI (`monkey-cli`) -> Hardware Driver (`monkey-core`) -> Protocol & Safety Gate (`protocol`) -> Transport Abstraction (`Transport`) -> OS HID (`hidapi`).
- **Target Hardware**: Monka 3075 Pro (Shenzhen HFD Technology `RKGK890`, VID `0x05AC`, PID `0x024F`).

## USB Interface Model

- **Interface A (`Usage Page 0xFF68`, `Usage 0x61`)**: Standalone vendor collection for bulk LCD streaming via 4096-byte OUT reports (Report ID 0). Works out-of-the-box on macOS without permission prompts.
- **Interface B (`Usage Page 0xFFFF`, `Usage 0x0001`)**: Composite configuration interface sharing HID with Consumer Control and Mouse. Uses 64-byte feature reports for RGB and device settings. Requires `macos-shared-device` feature in `hidapi`.

## Concurrency & Threading Model

- **Single-Flight Hardware Access**: USB HID hardware is stateful and cannot accept interleaved concurrent writes.
- **Dedicated Worker Thread**: `HardwareChannel` owns `HidDevice` handles on an isolated OS background thread, communicating with caller code via `crossbeam-channel`.
- **Strict Pacing**: 10-25ms inter-chunk delays prevent MCU buffer drops during 10-15 FPS LCD streaming.

## Display & Graphics Pipeline

- **Resolution & Format**: 128x128 pixels in RGB565 endian-safe format (32,768 bytes total).
- **Chunk Slicing**: Frames are segmented into 8 x 4096-byte chunks (`LCD_CHUNK_SIZE = 4096`).
- **Asset Ingestion**: Pure Rust `image` crate (0.25.8) handles PNG, JPEG, GIF, and BMP decoding and resizing.
- **Planned Milestone 2 Daemon**: `embedded-graphics` is planned for Milestone 2 procedural ambient status UI widgets.
