# Stack Research

**Domain:** Keyboard Hardware Driver & CLI Tooling (macOS App Sandbox / Cross-Platform HID, 128x128 RGB565 LCD Streaming, Feature Report RGB)  
**Researched:** 2026-09-13  
**Confidence:** HIGH  

---

## Executive Summary

The MonKey driver stack requires high-performance, predictable I/O with strict hardware safety rails. The hardware target (Monka 3075 Pro / Shenzhen HFD Technology `RKGK890`, VID/PID `0x05AC:0x024F`) exposes two distinct USB HID interfaces:
1. **Interface A (`Usage Page 0xFF68`, `Usage 0x61`)**: Standalone vendor collection with 4096-byte OUT report (8 chunks of 4096 bytes = 32,768 bytes for one 128x128 RGB565 frame). It has no collision with protected OS collections and works out-of-the-box on macOS without permission prompts.
2. **Interface B (`Usage Page 0xFFFF`, `Usage 0x0001`)**: Shared composite interface with Consumer Control (`0x0C`) and Mouse (`0x01`). Uses 64-byte feature reports (`HidD_SetFeature` / `IOHIDDeviceSetReport`) for RGB and configuration, requiring proper entitlement and non-exclusive device access on macOS.

The recommended stack is **100% Rust**, leveraging `hidapi` (2.6.7) with `macos-shared-device`, `zerocopy` (0.8.57) for zero-allocation packet slicing, a dual graphics pipeline combining `image` (0.25.10) for asset decoding and `embedded-graphics` (0.8.2) for procedural status UI rendering, and a **synchronous driver core with dedicated hardware worker thread** communicating via `crossbeam-channel` (0.5.17) rather than tying the core driver to an async runtime like Tokio.

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| **Rust Language & Cargo** | `1.80+` (Edition 2021/2024 ready) | Core programming language & package manager | Guarantees memory safety, data-race prevention, zero garbage collection pauses (critical during 10–15 FPS LCD streaming), predictable millisecond pacing for inter-packet delays, and seamless C ABI interop with Apple's IOKit. |
| **`hidapi`** | `2.6.7` (with `macos-shared-device`) | Userspace USB HID communication across macOS, Linux, and Windows | Battle-tested C/Rust wrapper around Apple's `IOHIDManager`, Win32 HID, and Linux `hidraw`. Unlike raw USB libraries (`nusb`/`rusb`), `hidapi` operates through the OS HID stack without needing to detach Apple's default kernel keyboard driver (`AppleUserHIDDevice`). The `macos-shared-device` feature flag invokes `hid_darwin_set_open_exclusive(0)`, preventing exclusive lockouts between composite interfaces. |
| **`zerocopy`** | `0.8.57` | Zero-copy packet transmutation, chunk slicing, and endian-safe conversions | Monka 3075 Pro streams 32,768 bytes per frame (8 × 4096 bytes) at 10–15 FPS (~327–491 KB/s). `zerocopy` provides `FromBytes`, `IntoBytes`, and `KnownLayout` derive macros with endian-aware primitives (`U16<LittleEndian>`, `U32<BigEndian>`), ensuring alignment safety on ARM64 Apple Silicon without memory allocations or copy overhead. |
| **`clap`** | `4.6.6` (features `derive`, `env`, `cargo`) | Command-line interface parser for `monkey-cli` | Industry standard declarative CLI framework in Rust. Provides strongly typed subcommand trees (`info`, `lcd`, `rgb`, `bench`), automatic shell completion generation, environment variable parsing, and clear help documentation. |
| **`image`** | `0.25.10` (features `png`, `jpeg`, `gif`, `webp`) | External asset ingestion, GIF animation decoding, downscaling | Pure-Rust image decoding with built-in multi-frame GIF decoding (`GifDecoder`) and high-quality image resizing (`Lanczos3` / `CatmullRom`). Extracts per-frame animation delays to drive hardware animation playback. |
| **`embedded-graphics`** | `0.8.2` | Procedural status rendering (text, badges, shapes, icons) directly to RGB565 | Lightweight, `no_std`-capable 2D graphics engine. Directly renders text, AI agent status indicators ("IDLE", "THINKING", "WAITING FOR YOU"), progress bars, and shapes into a 128x128 `Rgb565` off-screen framebuffer without OS display server dependencies. |
| **Dedicated Worker Thread + `crossbeam-channel`** | `0.5.17` | Hardware I/O serialization & concurrency model | USB HID hardware is physically single-flight and stateful; concurrent interleaved writes cause bus collisions and corrupted frames. A dedicated background OS worker thread owns the `HidDevice` handles and enforces strict 10–25ms inter-chunk delays. Channels decouple the CLI and future Tauri async IPC from blocking hardware operations without dragging Tokio into `monkey-core`. |

---

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| **`crc`** | `3.4.0` | Algorithmic checksum generation (CRC16-CCITT, CRC16-MODBUS, CRC16-ARC) | Calculating and verifying checksum bytes required by Shenzhen HFD protocol headers on configuration packets and bulk chunks. |
| **`thiserror`** | `2.0.20` | Structured domain-specific error types | In `crates/monkey-core` to construct clear, typed errors (`TransportError`, `ProtocolError`, `FrameError`, `SafetyViolationError`) with context and source chaining. |
| **`anyhow`** | `1.0.104` | Application error reporting | In `crates/monkey-cli` for high-level error handling, CLI user-facing diagnostics, and backtraces. |
| **`tracing`** | `0.1.44` | Structured diagnostics and telemetry | Instrumenting protocol operations, timing measurements, and packet dumps across `monkey-core` and `monkey-cli`. |
| **`tracing-subscriber`** | `0.3.23` (feature `env-filter`, `fmt`) | Log formatting and runtime filtering | Configuring human-readable terminal output or structured JSON traces via `RUST_LOG=monkey=trace`. |
| **`serde` & `serde_json`** | `1.0.229` / `1.0.151` (feature `derive`) | Serialization for layout files and CLI output | Loading `research/layout_81keys.json` matrix mappings, capability manifests, and emitting machine-readable output for `monkey info --json`. |
| **`indicatif`** | `0.18.6` | Progress bars and transfer indicators | Rendering upload progress, chunk status, and transfer throughput metrics in `monkey lcd` and `monkey bench`. |
| **`ctrlc`** | `3.4.5` (features `termination`) | Active SIGINT/Ctrl+C signal interception | Catches terminal interrupts to trigger cooperative cleanup frames (`04 F0`) before process exit, preventing locked MCU states. |

---

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| **`cargo-nextest`** | High-performance test runner | Runs unit tests (protocol codecs, dithering algorithms, capability matrix safety gates) in parallel with clean failure isolation. |
| **`cargo-deny`** | Dependency auditing | Enforces strict license compliance (MIT/Apache-2.0, preventing accidental GPL contamination) and flags vulnerable crate versions. |
| **`cargo-clippy`** | Static analysis and idiomatic lints | Run with `-D warnings -D clippy::all -D clippy::pedantic` to prevent unaligned memory access, accidental integer overflows, and redundant clones. |
| **`ioreg` & macOS Console** | Hardware enumeration inspection | Command `ioreg -p IOUSB -l -w0` inspects IOKit USB properties; `log stream --predicate 'subsystem == "com.apple.TCC"'` tracks macOS permission events. |
| **`Wireshark` + `USBPcap`** | USB protocol capture against OEM driver | Used on Windows/VM for verifying vendor packet captures against `capture_plan.md`. |

---

## Installation

### Workspace Configuration (`Cargo.toml`)

```toml
[workspace]
members = [
    "crates/monkey-core",
    "crates/monkey-cli",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["MonKey Contributors"]
license = "MIT"
repository = "https://github.com/monkakeyboard/monkey"

[workspace.dependencies]
# Hardware & Transport
hidapi = { version = "2.6.7", features = ["macos-shared-device"] }
zerocopy = { version = "0.8.57", features = ["derive"] }
crossbeam-channel = "0.5.17"
crc = "3.4.0"

# Graphics & Image Processing
image = { version = "0.25.10", default-features = false, features = ["png", "jpeg", "gif", "webp", "bmp"] }
embedded-graphics = "0.8.2"

# CLI & Diagnostics
clap = { version = "4.6.6", features = ["derive", "env", "cargo"] }
indicatif = "0.18.6"
ctrlc = { version = "3.4.5", features = ["termination"] }
tracing = "0.1.44"
tracing-subscriber = { version = "0.3.23", features = ["env-filter", "fmt"] }

# Serialization & Error Handling
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.151"
thiserror = "2.0.20"
anyhow = "1.0.104"
```

### Core Crate (`crates/monkey-core/Cargo.toml`)

```toml
[package]
name = "monkey-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
hidapi.workspace = true
zerocopy.workspace = true
crossbeam-channel.workspace = true
crc.workspace = true
image.workspace = true
embedded-graphics.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true

[dev-dependencies]
tempfile = "3.17.1"
```

### CLI Crate (`crates/monkey-cli/Cargo.toml`)

```toml
[package]
name = "monkey-cli"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "monkey"
path = "src/main.rs"

[dependencies]
monkey-core = { path = "../monkey-core" }
clap.workspace = true
indicatif.workspace = true
ctrlc.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
anyhow.workspace = true
serde_json.workspace = true
```

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| **`hidapi` (2.6.7)** | `nusb` (0.2.7) / `rusb` (0.9.4) / `libusb` | **Never for standard HID on macOS.** Apple kernel extensions claim HID devices automatically. Userspace raw USB APIs cannot claim claimed interfaces without custom DriverKit system extensions. Use `nusb` only for custom USB vendor devices lacking standard HID descriptors. |
| **`hidapi` (2.6.7)** | `async-hid` (0.5.3) | If building a 100% async-native application from day one where pure Rust `objc2-io-kit` bindings are desired. However, `async-hid` is still early-stage (v0.5), has smaller community validation, and lacks the mature battle-tested edge-case handling of `hidapi` across mixed macOS versions. |
| **Dedicated OS Thread + Channels** | Full `tokio` Runtime in `monkey-core` | If `monkey-core` had to handle hundreds of concurrent network connections (e.g., HTTP server or WebSocket server). For USB HID with serialized single-flight hardware constraints, Tokio introduces runtime bloat, context switching jitter, and complicates synchronous CLI commands. |
| **`zerocopy` (0.8.57)** | `bytemuck` (1.25.2) | When simple plain-old-data (POD) casting without endianness abstraction is sufficient. `zerocopy` is superior here because it provides built-in endian-aware numeric types (`U16<LittleEndian>`, etc.) essential for parsing multi-byte hardware packet fields. |
| **`zerocopy` (0.8.57)** | `deku` (0.18) / `binrw` (0.14) | For complex, dynamic, bit-level protocol schemas with variable-length nested fields. For MonKey's fixed 64-byte and 4096-byte hardware chunks, `zerocopy` avoids runtime heap allocations and dynamic parsing overhead. |
| **`embedded-graphics` (0.8.2)** | `tiny-skia` (0.11) | If rendering complex anti-aliased vector paths or SVG artwork at high resolutions. For a 128x128 pixel display showing glanceable text, status badges, and simple icons, `embedded-graphics` is vastly lighter and outputs native RGB565 directly. |

---

## What NOT to Use and Why

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **`nusb` / `rusb` (Raw USB Claiming)** | On macOS, Apple's `AppleUserHIDDevice` kernel driver automatically claims any device declaring HID class (`0x03`). Calling `claim_interface` via raw USB fails with `kIOReturnExclusiveAccess`. macOS App Sandbox also strictly forbids claiming standard HID devices under `com.apple.security.device.usb`. | **`hidapi` with `macos-shared-device`**, using `IOHIDManager` which communicates directly through Apple's supported HID driver stack. |
| **Embedding Tokio directly in `monkey-core`** | Wrapping blocking C `hidapi` functions (`hid_write`, `hid_read_timeout`) in Tokio async functions either blocks worker threads or requires `spawn_blocking` wrappers on every packet, introducing scheduling jitter into tight 10–25ms chunk transfers. It also forces Tokio onto all downstream library users. | **Synchronous core driver API with a dedicated worker thread** and `crossbeam-channel` message passing. Tauri v2 async commands can bridge cleanly via oneshot channels. |
| **Guessing Bootloader / DFU Opcodes** | OEM MCU solutions (HFD/RKGK) share command address spaces between normal configuration and bootloader firmware flash triggers. Sending unverified guessed opcodes can trigger sector erases or enter an unrecoverable ISP mode, permanently bricking the keyboard. | **Schema-driven capability matrix with strict whitelist validation.** Only execute captures verified on real hardware. |
| **High-Frequency Direct Flash Commits** | Low-cost onboard SPI NOR flash typically supports only 10,000–100,000 write cycles per sector. Committing RGB slider changes or live status frames directly to flash wears out the memory within days. | **Two-tier state model:** RAM preview for live status/color changes (throttled at ~30Hz, zero flash writes); debounced Flash commit (500ms after user stops editing). |
| **Electron + `node-hid`** | 150MB+ bundle size, 200MB+ background RAM usage, Node ABI compilation headaches on Apple Silicon, and V8 garbage collection pauses that cause stutter in continuous LCD transfers. | **Rust CLI first, moving to Tauri v2** (<15MB RAM, native WKWebView, zero GC pauses on the hardware thread). |
| **Assuming Report ID byte is transmitted on wire when Report ID is 0** | MonKey's vendor HID pipe (`0xFF68`) uses unnumbered reports (Report ID 0). While `hidapi` requires buffer `[0x00, ...payload]` so its internal C layer recognizes it as unnumbered, macOS `IOHIDDeviceSetReport` strips this leading byte, transmitting only the raw 4096 payload bytes. Writing packet codecs that expect an on-wire Report ID causes 1-byte framing offsets. | **Explicit transport abstraction:** codec produces raw on-wire bytes; the `hidapi` transport wrapper handles prepending `0x00` when calling `hid_write`. |

---

## Stack Patterns by Variant

### Variant 1: Standalone CLI Tooling (`crates/monkey-cli`)
- **Use:** Synchronous execution on the main thread for one-shot commands (`info`, `rgb static FF0000`).
- **Use:** Spawns a dedicated streaming thread with `indicatif` progress bar when transferring multi-frame animations (`monkey lcd animate dog.gif`).
- **Why:** Maximum simplicity, zero background daemon overhead, instant startup (<5ms).

### Variant 2: Ambient AI Status Display Engine (Milestone 2 Daemon)
- **Use:** Dedicated background worker thread holding persistent `HidDevice` handles with a 3–5 second heartbeat ping.
- **Use:** `embedded-graphics` renders dynamic agent status widgets (e.g. Claude Code "THINKING", "WAITING FOR USER", progress bars) directly into a 128x128 RGB565 memory buffer.
- **Use:** Pushes frames over Interface A vendor HID OUT report (`0xFF68`) directly into display RAM, with zero flash writes.
- **Why:** Delivers glanceable status at 10–15 FPS while completely eliminating SPI flash wear.

### Variant 3: Future Tauri v2 Desktop GUI (Post-M1)
- **Use:** Tauri frontend (Svelte 5 / React 19) invokes asynchronous Tauri IPC commands (`#[tauri::command]`).
- **Use:** Tauri commands send typed action messages across `crossbeam-channel` to the `monkey-core` dedicated hardware thread, awaiting response via `tokio::sync::oneshot`.
- **Why:** Prevents hardware I/O from blocking the 60 FPS desktop UI event loop while maintaining deterministic inter-packet pacing on the hardware thread.

---

## macOS App Sandbox & Entitlements Details

To ensure `monkey-core` is sandbox-ready for future Tauri v2 distribution, the stack must respect macOS security boundaries:

```xml
<!-- Entitlements for Developer ID Signed & Notarized macOS App -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- App Sandbox -->
    <key>com.apple.security.app-sandbox</key>
    <true/>
    
    <!-- USB Device Access (for raw vendor interfaces) -->
    <key>com.apple.security.device.usb</key>
    <true/>
    
    <!-- Network client (for fetching updates / AI hooks if needed) -->
    <key>com.apple.security.network.client</key>
    <true/>
</dict>
</plist>
```

### Critical macOS Permission Findings:
1. **Vendor Usage Page `0xFF68` (Interface A — Bulk Display Pipe)**:
   - Standalone collection without co-resident keyboard or mouse usages.
   - Accessible in macOS App Sandbox and WebHID without user-facing TCC prompts or Input Monitoring permissions.
2. **Usage Page `0xFFFF` (Interface B — Configuration & Feature Reports)**:
   - Co-resides on the composite device with Consumer Control (`0x0C`) and Mouse (`0x01`).
   - `hidapi` must be compiled with `features = ["macos-shared-device"]` so it calls `hid_darwin_set_open_exclusive(0)`. This allows `IOHIDDeviceOpen` to succeed without seizing the device from Apple's system keyboard driver.
   - For reading input reports on this interface, macOS requires **Input Monitoring** (`kTCCServiceListenEvent`). The driver must isolate feature report writes from input listening to function gracefully when input monitoring is ungranted.

---

## Version Compatibility Matrix

| Package | Version | Compatible With | Notes |
|---------|---------|-----------------|-------|
| `hidapi` | `2.6.7` | macOS 12+ (Monterey through Sequoia), Linux kernel 5.4+, Windows 10/11 | Uses system `IOKit`, `CoreFoundation`, and `AppKit` on macOS. |
| `zerocopy` | `0.8.57` | Rust `1.70+` | Uses modern `IntoBytes` (replaces deprecated `AsBytes` from 0.7). |
| `clap` | `4.6.6` | Rust `1.74+` | Derive macros generate compile-time checked argument trees. |
| `image` | `0.25.10` | Rust `1.75+` | Decodes animated GIFs and extracts frame delay metadata. |
| `embedded-graphics` | `0.8.2` | `embedded-graphics-core 0.4.1`, Rust `1.70+` | Zero-allocation drawing pipelines. |
| `thiserror` | `2.0.20` | Rust `1.70+` | Modernized 2.0 release with enhanced diagnostic attributes. |

---

## Sources

- `crates.io/api/v1/crates/*` — Verified current package releases and feature flags (`hidapi` 2.6.7, `zerocopy` 0.8.57, `clap` 4.6.6, `image` 0.25.10, `embedded-graphics` 0.8.2, `crossbeam-channel` 0.5.17).
- `github.com/libusb/hidapi` (`mac/hid.c`) — Verified macOS report ID 0 handling (`IOHIDDeviceSetReport` strips report ID 0; caller must prefix `0x00`) and `macos-shared-device` (`hid_darwin_set_open_exclusive(0)`).
- `research/prior_art_protocol.md` & `research/capture_plan.md` — Verified real hardware dual-interface map (`0xFF68` 4096-byte vendor HID OUT pipe vs `0xFFFF` 64-byte feature reports).
- Apple Developer Documentation — macOS App Sandbox & IOKit Human Interface Device Access (`IOHIDManager`).

---
*Stack research for: MonKey (Monka 3075 Pro Keyboard Driver & CLI)*  
*Researched: 2026-09-13*
