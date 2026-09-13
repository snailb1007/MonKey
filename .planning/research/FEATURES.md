# Feature Research

**Domain:** Custom Mechanical Keyboard Driver, Hardware Abstraction Layer & LCD Display CLI (`crates/monkey-core` & `crates/monkey-cli`)  
**Researched:** 2026-09-13  
**Confidence:** HIGH (Derived from verified hardware captures on Monka 3075 Pro, deconstructed OEM driver manifests `device.xml` and `KeyboardLayout.xml`, verified `05AC:024F` / `RKGK890` implementations from `rcsn01/GMK-67-Driver`, and comparative analysis of `wsclx/ak820pro-modder`, `Aiacos/ajazz-control-center`, VIA/Vial, and OpenRGB).

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist in modern mechanical keyboard driver CLIs and display controllers. Missing any of these means the product feels incomplete, unproven, or unusable as a driver foundation.

| Feature | Why Expected | Complexity | Implementation Notes |
|---------|--------------|------------|----------------------|
| **Device Discovery & Enumeration (`monkey info`)** | Users need to confirm the keyboard is connected, recognized, and accessible without permission errors. | LOW | Match VID `0x05AC` (spoofed Apple), PID `0x024F`, and product name `Gaming Keyboard`. Enumerate both HID interfaces: vendor bulk interface (`Usage Page 0xFF68`, `Usage 0x61`) and configuration interface (`Usage Page 0xFFFF`, `Usage 0x01`). |
| **Safe Capability Probing (`monkey probe`)** | Custom keyboards have varied hardware revisions and modes (Wired USB vs 2.4GHz Dongle vs Bluetooth). Probe must detect available features without altering state. | MEDIUM | Query hardware revision, firmware string, and connection mode. Strictly isolate read probes from write paths. Detect whether device is attached via physical wire or phantom wireless dongle (via heartbeat ping). |
| **Static LCD Frame Rendering (`monkey lcd image`)** | The primary physical feature of the Monka 3075 Pro is its 128x128 color LCD. Users expect to display arbitrary custom images. | MEDIUM | Ingest standard formats (PNG, JPG, BMP). Resize/crop to 128x128. Convert to 16-bit RGB565 (32,768 bytes total). Split into 8 discrete 4096-byte OUT chunks over bulk interface `0xFF68`. Target latency: <50ms per frame. |
| **Basic Animation Playback (`monkey lcd anim`)** | Animated GIFs/pixel art on keyboard screens are standard in competing screen-equipped boards (Ajazz AK820 Pro, Zoom75). | MEDIUM | Parse animated GIF/APNG or directory of frames on host. Decode to series of 128x128 RGB565 frames. Stream sequentially at target 10–15 FPS using host timer pacing. Support loop control and graceful interruption (Ctrl+C). |
| **Ambient / Preset RGB Control (`monkey rgb set`)** | RGB lighting configuration is standard for custom mechanical keyboards. | LOW | Implement verified single-packet command `04 13` over 64-byte feature reports on interface `0xFFFF`. Set built-in animation modes (Static, Breathing, Wave, Ripple, etc.), speed (1–5), brightness (0–100%), and RGB hex color. |
| **Diagnostic Benchmark Tool (`monkey bench`)** | Developers and power users need to measure USB throughput, frame delivery latency, and detect report drops before deploying animations. | MEDIUM | Send test pattern bursts to LCD bulk pipe (`0xFF68`) and measure round-trip chunk ACK timing, frame transfer rate (FPS ceiling), and bus jitter. Measure feature report round-trip latency on `0xFFFF`. |
| **RGB Configuration Readback & Restore (`monkey rgb save / restore`)** | Changing lighting temporarily (e.g. for notifications or ambient tests) must not permanently wipe the user's preferred lighting setup. | MEDIUM | Execute `04 F5` readback report to pull current 9-frame lighting table into memory/file. Provide `restore` command to rewrite saved state via `04 18` -> `04 20` / `04 13` -> `04 02` -> `04 F0`. |
| **Structured JSON Output (`--json`)** | CLIs used in scripted workflows or desktop wrappers require machine-readable outputs. | LOW | Provide `--json` flag across all diagnostic commands (`info`, `probe`, `bench`, `rgb get`) emitting consistent, versioned JSON schemas. |
| **Predictable Error Codes & Diagnostics** | Clear error messages when device is unplugged, in sleep mode, or blocked by OS permissions. | LOW | Return standard POSIX exit codes (0 = Success, 1 = Device Not Found, 2 = Permission Denied / TCC Issue, 3 = Protocol Timeout, 4 = Payload Rejected). |

---

### Differentiators (Competitive Advantage)

Features that elevate MonKey beyond typical vendor companion bloatware and fragile reverse-engineered Python scripts. These directly support the core value: safe, verified communication and headless daemon readiness.

| Feature | Value Proposition | Complexity | Implementation Notes |
|---------|-------------------|------------|----------------------|
| **Dual-Interface Transport Multiplexing** | Completely solves macOS TCC Input Monitoring issues by separating display bulk transfers from configuration reports. | HIGH | `0xFF68` bulk pipe (4096-byte OUT) runs without requiring system Input Monitoring permissions. `0xFFFF` feature reports (64 bytes) handle RGB/configuration via native IOKit / `hidapi`. Single unified client routes commands to the correct interface transparently. |
| **macOS App Sandbox-Compliant Core (`monkey-core`)** | Allows direct embedding into future App Store-compliant or notarized desktop applications (Tauri v2) without requiring root permissions, kernel drivers, or unsafe entitlement bypasses. | HIGH | Zero reliance on C-bindings that bypass sandbox boundaries; clean separation of transport trait (`DeviceTransport`) permitting mock injection, native IOHIDManager, or WebHID adapters. |
| **Schema-Driven Protocol Safety Rails & Opcode Whitelist** | Guarantees the driver will never brick a user's keyboard through blind opcodes or corrupted writes. | HIGH | Hardened command dispatch table. Whitelist only capture-verified opcodes (`04 13`, `04 18`, `04 20`, `04 F5`, `04 02`, `04 F0`, and verified LCD chunk headers). Block known bootloader/ISP trigger vectors (e.g. `0x0B`, `0x7140`). Checksum and boundary validation on every packet. |
| **Perceptual Floyd-Steinberg Dithering for RGB565** | Prevents ugly color banding on small 128x128 TFT panels when converting 24-bit TrueColor images to 16-bit RGB565. | MEDIUM | Host-side image processing pipeline applies Floyd-Steinberg error diffusion dithering and gamma correction prior to raw RGB565 byte generation. Yields significantly higher visual quality on gradients and photos. |
| **Adaptive Frame-Pacing & Ping-Pong Transfer Engine** | Guarantees smooth 10–15 FPS LCD animation playback without saturating the USB bus or causing keystroke latency. | HIGH | Stream 4096-byte chunks with double-buffer pacing and backpressure monitoring. Drop frames host-side if the keyboard MCU FIFO signals backpressure, ensuring keystrokes (Interface 0) retain highest NVIC priority. |
| **Two-Tier State Sync (RAM Preview vs Flash Commit)** | Protects onboard SPI NOR Flash from premature wear (10k–100k cycle limit) and eliminates brown-out bricking during wireless battery dips. | MEDIUM | Differentiate between live ambient updates (RAM only, discarded on power cycle) and explicit user persistence (`--save` flag issuing `04 02` commit). Battery-level check inhibits flash writes if battery <20% on wireless modes. |
| **ASCII/ANSI Keyboard Matrix Visualizer** | Instant visual terminal feedback of key matrix layout (81 keys) and per-key RGB state for rapid debugging. | LOW | Render an ANSI-colored 75% layout in terminal stdout displaying active lighting colors and matrix indices (`matrix_key_index`) mapped from `layout_81keys.json`. |
| **Agent Surface Abstraction Hook Preparation** | Architected specifically to receive normalized state events from AI coding agents (Claude Code) in Milestone 2. | MEDIUM | Define a clean display canvas API (e.g., render status badge, progress bar, notification icon) that decouples hardware LCD blitting from the upstream state daemon. |

---

### Anti-Features (Commonly Requested, Often Problematic)

Features that frequently appear in vendor software or community feature requests, but create severe reliability, security, or hardware safety hazards. MonKey explicitly declines to build these in Milestone 1.

| Feature | Why Requested | Why Problematic | Alternative / Better Approach |
|---------|---------------|-----------------|-------------------------------|
| **Monolithic GUI / Desktop App in v1** | Users often prefer a graphical window over command line. | Developing a GUI before the hardware transport and protocol reverse-engineering are 100% verified creates two moving targets and massive tech debt. | Build a rock-solid, fully testable Rust CLI first (`monkey-cli`). Defer GUI to Tauri v2 in Milestone 3 once `monkey-core` is proven. |
| **Speculative Bootloader / DFU / Firmware Flashing** | Users want to flash custom QMK/Sonix firmware. | Guessing undocumented bootloader entry points or flashing corrupt binaries permanently bricks hardware (especially on OEM HFD/Sonix boards with masked bootloaders). | Strictly ban unverified flash write and DFU jump opcodes (`0x0B`, `0x7140`). Firmware flashing is out of scope. |
| **Real-Time 30–60 FPS Video Streaming Over USB HID** | Users want to play high-fps movies or games on the 128x128 LCD screen. | Full-Speed USB HID (12 Mbps) shared endpoints cannot sustain 30+ FPS raw 32KB frames (~1 MB/s) without causing USB FIFO saturation, dropped keystrokes, and MCU watchdog resets. | Target 10–15 FPS for ambient glanceable status, system badges, and pixel animations. Implement frame-drop backpressure. |
| **LCD Bulk Streaming over Bluetooth / BLE** | Users want full LCD animation playback while unplugged. | BLE bandwidth is capped at ~1–2 KB/s in practical conditions. Streaming 32KB frames would take 15–30 seconds per frame and drain the keyboard battery in minutes. | Restrict LCD chunk streaming to wired USB connection (`Usage Page 0xFF68`). Provide static built-in icon indexing over wireless if supported by firmware. |
| **Unthrottled 60Hz Reactive Per-Key RGB Streaming** | Users want reactive audio visualizers or screen-mirroring lighting. | Blasting 60Hz per-key RGB reports over 64-byte feature reports (`0xFFFF`) starves MCU processing cycles, induces typing jitter, and overheats low-cost controller ICs. | Limit host-driven RGB streaming to max 10–15Hz with drop-behind coalescing, or utilize the MCU's built-in hardware lighting modes (`04 13`). |
| **Monolithic In-Driver AI Agent Daemon** | Users want Claude Code integration out-of-the-box in v1. | Coupling LLM hook state parsing and background daemons directly into the hardware driver creates a brittle, unmaintainable architecture. | Defer agent daemon to Milestone 2. Keep `monkey-core` as a pure, stateless driver library and `monkey-cli` as a deterministic CLI tool. |
| **Cloud Account & Online Profile Telemetry** | Typical in proprietary software (Razer Synapse, SteelSeries GG). | Unnecessary privacy invasion, bloat, and internet failure point for a physical hardware input device. | 100% offline, zero-telemetry, local file/JSON-based configuration. |

---

## Feature Dependencies

```
[Device Discovery (monkey info)]
    └──requires──> [OS Transport Layer (hidapi / IOKit)]
                       └──requires──> [Vendor Interface Partition (0xFF68 vs 0xFFFF)]

[Safe Capability Probe (monkey probe)]
    └──requires──> [Device Discovery (monkey info)]
    └──requires──> [Protocol Safety Rails & Opcode Whitelist]

[Static LCD Frame Render (monkey lcd image)]
    └──requires──> [Vendor Interface Partition (0xFF68 bulk pipe)]
    └──requires──> [Image Processing Pipeline (RGB565 & Dithering)]
    └──requires──> [Protocol Safety Rails (Verified Chunk Header)]

[Animation Engine (monkey lcd anim)]
    └──requires──> [Static LCD Frame Render]
    └──requires──> [Adaptive Frame Pacing & Backpressure Engine]

[Ambient RGB Preset Control (monkey rgb set)]
    └──requires──> [Configuration Interface (0xFFFF feature reports)]
    └──requires──> [Verified Opcode Dispatch (04 13 / 04 18 / 04 02)]

[RGB Readback & Restore (monkey rgb save/restore)]
    └──requires──> [Ambient RGB Preset Control]
    └──requires──> [Verified Readback Opcode (04 F5)]

[Diagnostic Benchmark (monkey bench)]
    └──requires──> [Static LCD Frame Render]
    └──requires──> [Ambient RGB Preset Control]
    └──requires──> [High-Precision Timing Harness]

[Two-Tier State Sync (RAM vs Flash)]
    └──enhances──> [Ambient RGB Preset Control]
    └──enhances──> [Animation Engine]

[Floyd-Steinberg Dithering]
    └──enhances──> [Static LCD Frame Render]
    └──enhances──> [Animation Engine]

[Wireless / BLE Transport] ──conflicts──> [Bulk LCD Animation Streaming]
[High-Frequency Streaming] ──conflicts──> [Flash Commit Persistence]
```

### Dependency Notes

- **`Static LCD Frame Render` requires `Vendor Interface Partition (0xFF68)`:** The 128x128 display requires 32,768 bytes per frame. This can only be efficiently delivered via the 4096-byte OUT report on `0xFF68` (8 chunks). It cannot run over the 64-byte configuration endpoint without severe bus contention.
- **`Animation Engine` requires `Adaptive Frame Pacing`:** Streaming frames without backpressure will overflow the keyboard MCU's small RAM FIFO, causing visual tearing or dropped USB packets.
- **`RGB Readback & Restore` requires `04 F5` verification:** Reading the current state requires verified handling of device-rendered RGB responses (which differ slightly from host input bytes due to internal gamma tables).
- **`Floyd-Steinberg Dithering` enhances `Static LCD Frame Render`:** Converting 24-bit color to 16-bit RGB565 reduces color depth to 5 bits Red, 6 bits Green, 5 bits Blue. Error diffusion dithering eliminates noticeable color banding on gradients.
- **`Wireless / BLE Transport` conflicts with `Bulk LCD Animation Streaming`:** 32KB frame payloads over BLE are physically unviable. Bulk LCD streaming must be restricted to wired USB mode.
- **`High-Frequency Streaming` conflicts with `Flash Commit Persistence`:** Frequent flash writes destroy SPI NOR flash memory. Dynamic or streaming updates must only target MCU RAM buffers; flash commits (`04 02`) must be strictly debounced or triggered on explicit user request.

---

## MVP Definition

### Launch With (v1)

The minimum viable product needed to validate safe hardware communication, establish cross-platform transport, and deliver reliable LCD and RGB control.

- [ ] **Multi-Axis Device Identification (`monkey info`)** — Match VID `0x05AC`, PID `0x024F`, OEM identifier `RKGK890`, enumerate dual interfaces (`0xFF68` and `0xFFFF`).
- [ ] **Safe Capability Probe (`monkey probe`)** — Query protocol response, detect wire status vs wireless dongle without mutating device state.
- [ ] **Static LCD Image Upload (`monkey lcd image <file>`)** — Load PNG/JPG/BMP, convert to 128x128 RGB565 with Floyd-Steinberg dithering, transmit 8 x 4096-byte chunks over `0xFF68` in <50ms.
- [ ] **Basic LCD Animation Stream (`monkey lcd anim <file/dir>`)** — Play GIF/APNG or frame sequence at 10–15 FPS with host timer pacing and graceful Ctrl+C interruption.
- [ ] **Ambient RGB Preset Control (`monkey rgb set --mode ... --color ...`)** — Send verified `04 13` feature report to switch hardware effects, speed, brightness, and static color.
- [ ] **Protocol Safety Rails** — Schema-enforced opcode dispatch table blocking unverified opcodes and dangerous bootloader commands.
- [ ] **Hardware Diagnostic & Throughput Benchmark (`monkey bench`)** — Automated throughput and latency test measuring chunk delivery times, frame rate ceiling, and feature report round-trip times.
- [ ] **Machine-Readable CLI Output (`--json`)** — JSON output support on all inspection and benchmark commands for downstream tooling.

### Add After Validation (v1.x)

Features to add once core driver transport and frame stability are verified on real hardware.

- [ ] **RGB Readback and Snapshot (`monkey rgb save / restore`)** — Decode `04 F5` 9-frame readback table to back up and restore custom color configurations.
- [ ] **Custom Per-Key RGB Upload (`monkey rgb custom`)** — Support 8-chunk `04 20` table to set individual switch LED colors mapped to `layout_81keys.json`.
- [ ] **Onboard LCD Built-In Animation Indexing (`monkey lcd preset <id>`)** — Probe if firmware supports index switching (similar to Ajazz `0x51`) to trigger pre-flashed animations without host streaming.
- [ ] **LCD RTC / Clock Sync (`monkey lcd time`)** — Synchronize host system clock to onboard RTC if supported by the HFD display controller.
- [ ] **Text & System Status Banner Renderer (`monkey lcd text "..."`)** — Built-in canvas renderer to format CPU/RAM/Battery or text messages into 128x128 frames.

### Future Consideration (v2+)

Features deferred to subsequent major milestones after the core driver and CLI are battle-tested.

- [ ] **AI Agent Status Daemon (Milestone 2)** — Background process watching Claude Code hook events (`IDLE`, `THINKING`, `EDITING`, `RUNNING`, `WAITING_FOR_YOU`, `ERROR`) and rendering status badges to LCD and RGB.
- [ ] **Tauri v2 Desktop Application (Milestone 3)** — Native macOS/cross-platform UI providing visual keymap remapping, color picker, and drag-and-drop LCD GIF management using `monkey-core`.
- [ ] **Keymap Remap & Macro Engine (`04 11`)** — Full custom keymapping and macro table persistence, adhering to the 4-byte action format and matrix allocation table.
- [ ] **WebHID Browser Companion** — Standalone WebHID web page for zero-install static LCD uploads on Chromium-based browsers.

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| **Device Discovery (`monkey info`)** | HIGH | LOW | **P1** |
| **Protocol Safety Rails & Opcode Whitelist** | HIGH | LOW | **P1** |
| **Static LCD Image Render (`monkey lcd image`)** | HIGH | MEDIUM | **P1** |
| **Basic Animation Playback (`monkey lcd anim`)** | HIGH | MEDIUM | **P1** |
| **Ambient RGB Preset Control (`monkey rgb set`)** | HIGH | LOW | **P1** |
| **Diagnostic Benchmark Tool (`monkey bench`)** | HIGH | MEDIUM | **P1** |
| **Floyd-Steinberg Dithering Pipeline** | MEDIUM | LOW | **P1** |
| **JSON Output Mode (`--json`)** | MEDIUM | LOW | **P1** |
| **Two-Tier State Sync (RAM vs Flash)** | HIGH | MEDIUM | **P2** |
| **RGB Configuration Readback & Restore** | MEDIUM | MEDIUM | **P2** |
| **Custom Per-Key RGB (`04 20`)** | MEDIUM | HIGH | **P2** |
| **Terminal ANSI Matrix Visualizer** | MEDIUM | LOW | **P2** |
| **LCD RTC / Clock Sync** | LOW | MEDIUM | **P2** |
| **AI Agent State Daemon (Claude Code hooks)** | HIGH | HIGH | **P3 (Milestone 2)** |
| **Keymap Remap & Macro Engine** | MEDIUM | HIGH | **P3 (Milestone 2)** |
| **Tauri v2 Desktop Application** | HIGH | HIGH | **P3 (Milestone 3)** |

**Priority key:**
- **P1:** Must have for v1 launch (validates core hardware communication, safety, LCD, and RGB).
- **P2:** Should have, add in v1.x once core transport is rock solid.
- **P3:** Defer to dedicated subsequent milestones (Agent Daemon, GUI, Keymap Engine).

---

## Competitor Feature Analysis

| Feature | `rcsn01/GMK-67-Driver` | `wsclx/ak820pro-modder` | VIA / Vial | MonKey Approach |
|---------|------------------------|-------------------------|------------|-----------------|
| **Target Architecture** | Swift macOS App & CLI (`gmk67`) | Python CLI scripts | WebHID / C++ QMK Raw HID | Modular Rust workspace (`monkey-core` + `monkey-cli`) |
| **Hardware Compatibility** | Same `05AC:024F` / `RKGK890` (67 keys) | Ajazz AK820 Pro / Sonix MCU | QMK/Vial-compliant open MCUs | Monka 3075 Pro (`05AC:024F` / `RKGK890`, 81 keys) |
| **LCD Screen Support** | ❌ None (Board has no screen) | ✅ TFT frame & GIF upload via Python | ❌ None (Protocol has no display concept) | ✅ 128x128 RGB565 over dedicated 4096-byte bulk pipe (`0xFF68`) with Floyd-Steinberg dithering |
| **macOS Permissions Handling** | Relies on macOS IOKit native handles | Linux/Windows oriented, untested on macOS sandbox | WebHID requires TCC Input Monitoring for protected collections | Partitioned transport: bulk LCD runs without TCC prompt; config runs via native IOKit |
| **Protocol Safety Rails** | Basic payload verification | Ad-hoc script execution | Secure unlock keys (Vial) | Strict schema-driven opcode whitelist; blocks ISP/bootloader writes |
| **RGB Lighting Control** | ✅ Reverse-engineered `04 xx` packets | ✅ Custom script packets | ✅ VIA v3 lighting channels | ✅ Verified `04 13` ambient presets + `04 F5` readback snapshot |
| **Animation Frame Pacing** | N/A | Basic sleep delay in Python | N/A | Adaptive host-side double-buffer pacing with backpressure monitoring (10–15 FPS) |
| **Diagnostics & Benchmarking** | Basic `doctor` command | None | Matrix key tester | Comprehensive `monkey bench` throughput & latency suite with `--json` |
| **AI Coding Agent Integration** | ❌ None | ❌ None | ❌ None | Planned Milestone 2 ambient status surface (Claude Code hooks) |

---

## Sources

- **Monka 3075 Pro Hardware Captures & Manifests**:
  - `vendor_driver/device.xml` (VID `0x05AC`, PID `0x024F`, Solution `RKGK890`, Shenzhen HFD Technology).
  - `vendor_driver/KeyboardLayout.xml` & `research/layout_81keys.json` (81-key matrix, scancodes, LED indices).
  - `research/webhid_tester.html` physical measurements (Dual interface: `0xFF68` OUT 4096-byte bulk display vs `0xFFFF` 64-byte config).
- **Reverse-Engineered Prior Art**:
  - `rcsn01/GMK-67-Driver` (Verified `04 13`, `04 18`, `04 20`, `04 F5`, `04 02`, `04 F0` command table on identical `05AC:024F` HFD hardware).
  - `wsclx/ak820pro-modder` & `Aiacos/ajazz-control-center` (TFT 128x128 RGB565 animation structures, chunking, and RTC probe mechanics).
  - OpenRGB (`Controllers/RoyuanKeyboardController`, `Sonix SN32F248B` analysis in `research/reverse_engineering_guide.md`).
  - VIA v2/v3 & Vial Protocol Specifications (`research/via_vial_protocol_deepdive.md`).
- **Operating System Guidelines**:
  - Apple Developer Documentation: IOKit Human Interface Device (HID) Class & App Sandbox Entitlements.
  - USB HID Specification v1.11: Report Descriptors, Vendor Usage Pages (`0xFF00-0xFFFF`), Interrupt/Control transfers.

---
*Feature research for: MonKey keyboard driver & CLI*  
*Researched: 2026-09-13*
