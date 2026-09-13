# Phase 3: High-Performance LCD Rendering & Streaming Engine - Research

**Date:** 2026-09-13
**Domain:** RGB565 conversion, Floyd-Steinberg dithering, USB bulk chunking (Interface A 0xFF68), pacing regulation, GIF decoding.

## 1. Executive Summary & Constraints

Phase 3 implements the LCD streaming pipeline for the Monka 3075 Pro:
- Display resolution: 128x128 pixels, 16-bit RGB565 format ($128 \times 128 \times 2 = 32,768$ bytes per frame).
- Transport interface: Interface A (`0xFF68`), 4096-byte vendor HID OUT bulk reports.
- Frame chunking: Exactly 8 chunks of 4096 bytes per frame ($8 \times 4096 = 32,768$ bytes).
- Performance targets:
  - Dithering + conversion latency: <15ms per frame.
  - Inter-chunk pacing delay: 3ms–8ms (configurable).
  - Target frame rate: 10–15 FPS (default 12 FPS, ~83.3ms per frame).
  - Static frame total wire transfer time: <50ms.
- Memory safety: Zero allocations in tight streaming loops, pre-allocated frame buffers.

## 2. Technical Findings

### 2.1 RGB565 Format & Endianness
- RGB565 packs color into 16 bits:
  - Red: bits 11..15 (5 bits)
  - Green: bits 5..10 (6 bits)
  - Blue: bits 0..4 (5 bits)
- Value representation: `(r >> 3) << 11 | (g >> 2) << 5 | (b >> 3)`
- Endianness: Little-Endian byte order `[val as u8, (val >> 8) as u8]` based on prior captures of RKGK890 / Monka firmware. Big-Endian option provided for diagnostic toggling.
- Diagnostic test patterns:
  - Pure Red (`#FF0000`): RGB565 `0xF800` -> Little-Endian `[0x00, 0xF8]`
  - Pure Green (`#00FF00`): RGB565 `0x07E0` -> Little-Endian `[0xE0, 0x07]`
  - Pure Blue (`#0000FF`): RGB565 `0x001F` -> Little-Endian `[0x1F, 0x00]`

### 2.2 Floyd-Steinberg Error Diffusion Dithering
- Floyd-Steinberg distributes quantization error to neighboring unprocessed pixels:
  - Right: $7/16$
  - Down-left: $3/16$
  - Down: $5/16$
  - Down-right: $1/16$
- Serpentine scanning (alternating left-to-right and right-to-left) reduces directional worm artifacts.
- Fixed-point arithmetic (i16/i32 scaling) ensures execution finishes in <5ms for a 128x128 buffer on modern CPUs.

### 2.3 Frame Chunking & Interface A
- Interface A (`0xFF68`, report ID 0) accepts raw 4096-byte payloads without sub-packet protocol headers.
- When calling `hid_write` via `hidapi` on macOS, prepending `0x00` (report ID 0) is handled by the `Transport::write_bulk` abstraction.
- The 32KB frame is sliced into 8 contiguous slices:
  `frame[i * 4096 .. (i + 1) * 4096]` for $i \in [0..7]$.

### 2.4 Streaming Pacing & Backpressure
- USB Full-Speed endpoints have limited buffer depth; flooding 32KB back-to-back causes MCU SPI DMA drops.
- Pacing requirement: 3ms–8ms sleep between chunk writes.
- Frame rate regulation: An active loop pacing with `std::time::Instant` targeting e.g. 12 FPS. If frame rendering or transfer takes longer than the target interval, the loop drops/skips frames rather than queuing lag.

### 2.5 GIF Animation & Image Loading
- The `image` crate provides `image::load_from_memory` for static images and `image::codecs::gif::GifDecoder` for multi-frame animations.
- Animation decoder reads frame delay and image dimensions, center-crops/downscales to 128x128 using `CatmullRom` or `Lanczos3`.
