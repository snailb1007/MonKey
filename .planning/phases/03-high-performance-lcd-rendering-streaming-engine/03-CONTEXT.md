# Phase 3: High-Performance LCD Rendering & Streaming Engine - Context

**Gathered:** 2026-09-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 3 delivers the high-performance LCD rendering and streaming pipeline for MonKey:
1. Color conversion engine with RGB565 encoding (configurable endianness with default target Little-Endian, verified via test pattern) and Floyd-Steinberg error diffusion dithering (<15ms per frame processing budget).
2. Frame chunking engine slicing exactly 128x128 pixels (32,768 bytes) into 8 contiguous 4096-byte OUT chunks targeting Interface A (`0xFF68`).
3. Frame pacing and streaming regulator with configurable inter-chunk delays (3ms–8ms default) and frame rate regulation (10–15 FPS, default 12 FPS) to avoid USB Full-Speed bus saturation and MCU SPI DMA buffer overflows.
4. Static image loader and preprocessor resizing/cropping arbitrary images to 128x128, including pure Red/Green/Blue/Grayscale test patterns (`monkey lcd test-pattern`).
5. GIF animation decoder and streaming loop (`monkey lcd anim`) handling multi-frame playback, delay timings, frame drops under backpressure, and graceful cancellation on SIGINT/Ctrl-C.
6. CLI subcommands (`monkey lcd image`, `monkey lcd anim`, `monkey lcd test-pattern`) supporting progress reporting with `indicatif`.

Ambient RGB matrix lighting and two-tier flash persistence (Phase 4) and production packaging/hardening (Phase 5) are deferred.
</domain>

<decisions>
## Implementation Decisions

### 1. RGB565 Color Conversion & Floyd-Steinberg Dithering (LCD-01)
- Color pipeline converts source image to 128x128 RGB888, then quantizes to RGB565 (5 bits red, 6 bits green, 5 bits blue).
- Support Floyd-Steinberg error diffusion dithering with serpentine scanning to eliminate color banding across 16-bit color depth while keeping conversion latency strictly under 15ms.
- Endianness handling: Defaults to Little-Endian `[lo, hi]` as observed in target RKGK890 protocol research, with Big-Endian option for calibration.
- Dedicated diagnostic test pattern generator (`monkey lcd test-pattern`) rendering pure Red (`#FF0000` -> `0xF800`), Green (`#00FF00` -> `0x07E0`), Blue (`#0000FF` -> `0x001F`), and grayscale ramp to physically detect byte-swapping issues immediately.

### 2. Zero-Copy 32KB Frame Chunker (LCD-02)
- Frame size is strictly $128 \times 128 \times 2 = 32,768$ bytes.
- Chunker slices the 32KB buffer into exactly 8 equal chunks of 4096 bytes each:
  - Chunk 0: 0..4096
  - Chunk 1: 4096..8192
  - Chunk 2: 8192..12288
  - Chunk 3: 12288..16384
  - Chunk 4: 16384..20480
  - Chunk 5: 20480..24576
  - Chunk 6: 24576..28672
  - Chunk 7: 28672..32768
- Transmitted as unnumbered OUT reports on Interface A (`0xFF68`, report ID 0, prepending `0x00` in `hidapi` transport wrapper as needed). Zero protocol framing overhead per chunk.

### 3. Pacing & Flow Control Regulator (LCD-03)
- Microsecond-accurate inter-chunk pacing delay (default 3ms–8ms, configurable via CLI flag or driver config) allowing the MCU SPI DMA controller to drain without dropping packets.
- Frame rate regulator holding playback strictly within 10–15 FPS (default 12 FPS, ~83.3ms period).
- Non-blocking backpressure / frame drop mechanism: If frame processing or bus transfer takes longer than the frame interval, subsequent frames are dropped or coalesced to prevent keystroke lag or USB bus starvation.
- Total wire transfer latency for static frames kept under 50ms.

### 4. Static Image CLI (`monkey lcd image` / `monkey lcd test-pattern`) (LCD-04)
- Loads PNG, JPEG, WEBP, BMP via `image` crate.
- Center-crops and downscales to 128x128 using high-quality sampling (CatmullRom / Lanczos3).
- Dispatches 8 chunks to hardware via `Transport::write_bulk` with an `indicatif` progress bar.

### 5. Animated GIF Streaming Loop (`monkey lcd anim`) (LCD-05)
- Decodes multi-frame GIF via `image::codecs::gif::GifDecoder`.
- Extracts individual frame delays; resizes and dithers frames ahead of time or in a streaming pipeline.
- Continuous loop with `--loop` flag (or default infinite loop until Ctrl-C).
- Signal handling via `ctrlc` or cooperative cancellation token ensuring clean exit and transport release.

### Claude's Discretion
- Organization of `lcd` module in `crates/monkey-core/src/lcd/` (`color.rs`, `dither.rs`, `chunker.rs`, `streamer.rs`, `mod.rs`).
- Exact progress bar styling and terminal formatting in `crates/monkey-cli/src/commands/lcd.rs`.
- Unit test fixtures and synthetic benchmarking for dithering/chunking speed.
</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/monkey-core/src/transport/`: `Transport` trait with `write_bulk(&self, chunk: &[u8]) -> Result<(), TransportError>`, `HidTransport`, `MockTransport`.
- `crates/monkey-core/src/protocol/channel.rs`: Hardware channel and worker thread patterns.
- `crates/monkey-core/src/protocol/types.rs`: Basic LCD types and constants (`LCD_WIDTH`, `LCD_HEIGHT`, `LCD_FRAME_BYTES`, `LCD_CHUNK_SIZE`, `LCD_CHUNK_COUNT`).
- `crates/monkey-cli/src/commands/`: CLI command structure (`bench.rs`, `info.rs`, `probe.rs`).
- `crates/monkey-cli/src/output.rs`: Terminal output and status helpers.

### Established Patterns
- High performance, zero unnecessary allocations.
- Strongly typed errors (`DriverError`, `TransportError`, `ProtocolError`).
- Standalone CLI subcommands with clap derive macros.
- Comprehensive unit tests using `MockTransport`.

### Integration Points
- `crates/monkey-core/src/lcd/`: LCD color, dithering, chunking, and streaming pipeline.
- `crates/monkey-cli/src/commands/lcd.rs`: `monkey lcd image`, `monkey lcd anim`, `monkey lcd test-pattern`.
- `crates/monkey-cli/src/args.rs`: Clap command tree expansion.
</code_context>

<specifics>
## Specific Ideas
- Provide unit tests verifying exact byte outputs for red (`0xF800` little-endian `[0x00, 0xF8]`), green (`0x07E0` little-endian `[0xE0, 0x07]`), blue (`0x001F` little-endian `[0x1F, 0x00]`).
- Validate Floyd-Steinberg processing latency in a unit benchmark ensuring it meets <15ms constraint.
- Ensure `MockTransport` can record `write_bulk` calls to assert 8 chunks of 4096 bytes each.
</specifics>

<deferred>
## Deferred Ideas
- Phase 4: Ambient RGB Engine, Matrix Mapping & Two-Tier State Persistence.
- Phase 5: Diagnostic doctor command, cargo-deny, release packaging.
</deferred>
