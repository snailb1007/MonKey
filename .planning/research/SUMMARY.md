# Project Research Summary

**Project:** MonKey (MonkaKeyboard)  
**Domain:** Custom Keyboard Hardware Driver & CLI Tooling (Rust / macOS USB HID / 128x128 RGB565 LCD Streaming / Feature Report RGB)  
**Researched:** 2026-09-13  
**Confidence:** HIGH  

---

## Executive Summary

MonKey is an open-source, cross-platform userspace driver and diagnostic CLI ecosystem written in Rust for the Monka 3075 Pro mechanical keyboard (Shenzhen HFD Technology `RKGK890`, spoofed Apple VID/PID `0x05AC:0x024F`). Commercial companion software for this hardware is Windows-only, bloated (Electron/MFC), closed-source, and prone to hardware corruption. Building an open-source driver requires precise USB HID orchestration: the keyboard exposes two distinct endpoints on macOS—a standalone vendor collection (`Usage Page 0xFF68`, `Usage 0x61`) accepting 4096-byte OUT bulk reports for its 128x128 color LCD, and a composite configuration interface (`Usage Page 0xFFFF`, `Usage 0x01`) co-resident with Consumer Control and Mouse usages using 64-byte feature reports.

The recommended architectural approach is a modular Rust workspace (`crates/monkey-core` and `crates/monkey-cli`) operating directly through Apple's `IOHIDManager` via `hidapi` (compiled with `macos-shared-device`). Rather than binding the driver to an asynchronous runtime like Tokio, `monkey-core` employs a synchronous core driver API with a dedicated OS hardware worker thread communicating across `crossbeam-channel` queues. This guarantees microsecond-accurate inter-packet pacing (2–5ms for LCD chunks, 15ms for queries, 80ms for flash writes) without runtime bloat. The LCD rendering pipeline couples `image` for asset ingestion with `embedded-graphics` for procedural badge drawing and Floyd-Steinberg error diffusion to eliminate 16-bit color banding.

The primary engineering risks are hardware bricking and system permission lockouts. Low-cost HFD/Sonix Cortex-M0 microcontrollers share their vendor command dispatch space with ISP bootloader vectors (`PID 0x7140`) and NOR flash mass-erase routines; blind opcode guessing will permanently brick the board. Furthermore, blasting live color updates or high-FPS animations burns out SPI NOR flash write cycles (rated for 10k–100k cycles) or saturates the Full-Speed USB bus, dropping matrix keystrokes. MonKey mitigates these hazards through a compile-time typestate `SafetyGate` that blocks unverified opcodes, an RAII `TransactionGuard` ensuring clean protocol cleanup, a two-tier RAM Preview vs Debounced Flash Commit state model, and strict 10–15 FPS rate limiting for display transfers.

---

## Key Findings

### Recommended Stack

MonKey utilizes a 100% Rust workspace targeting Rust `1.80+` (Edition 2021/2024-ready). Rust guarantees memory safety, zero garbage collection jitter during continuous 10–15 FPS LCD streaming, and seamless C FFI interop with Apple's `IOKit` and `IOHIDManager`. To prevent OS keyboard seizure conflicts and ensure compatibility with future sandboxed macOS distribution (Tauri v2), the driver operates via `hidapi` (2.6.7) with the `macos-shared-device` flag (`hid_darwin_set_open_exclusive(0)`). Raw USB libraries (`nusb`/`rusb`) are explicitly rejected because Apple's `AppleUserHIDDevice` kernel driver claims all HID-class devices, returning `kIOReturnExclusiveAccess` to userspace claims.

Packet processing relies on `zerocopy` (0.8.57) for zero-allocation packet slicing and endian-safe numeric conversions (`U16<LittleEndian>`, `U32<BigEndian>`), ensuring safe layout transmutation on Apple Silicon ARM64 without copy overhead. Graphics processing uses a dual pipeline: `image` (0.25.10) for external asset ingestion and multi-frame GIF decoding, paired with `embedded-graphics` (0.8.2) for zero-allocation procedural status badges. Command-line interactions are powered by `clap` (4.6.6) with derive macros, while diagnostics leverage `tracing` (0.1.44) and `indicatif` (0.18.6).

**Core technologies:**
- **Rust `1.80+` & Cargo:** Core programming language and workspace manager — provides memory safety, predictable microsecond timing, zero GC pauses during LCD streaming, and clean C ABI interop with IOKit.
- **`hidapi` `2.6.7` (`macos-shared-device`):** Userspace USB HID communication across macOS, Linux, and Windows — operates within Apple's supported IOHIDManager stack without needing kernel driver detachment; non-exclusive flag avoids locking composite keyboard endpoints.
- **`zerocopy` `0.8.57`:** Endian-aware, zero-copy packet transmutation and chunk slicing — slices 32,768-byte frame buffers into 8x 4096-byte OUT chunks with zero heap allocations and verified alignment safety on Apple Silicon.
- **Dedicated Worker Thread + `crossbeam-channel` `0.5.17`:** Hardware I/O serialization and concurrency model — enforces single-flight hardware constraints and microsecond inter-packet pacing without tying the core driver to Tokio.
- **`image` `0.25.10` & `embedded-graphics` `0.8.2`:** Dual graphics ingestion and procedural rendering engines — `image` decodes animated GIFs/PNGs and extracts frame delay metadata; `embedded-graphics` generates glanceable status badges directly in RGB565.
- **`clap` `4.6.6` (derive, env):** Command-line parser for `monkey-cli` — provides typed command trees (`info`, `probe`, `lcd`, `rgb`, `bench`), shell auto-completions, and structured machine-readable `--json` output.

---

### Expected Features

The feature landscape balances essential hardware validation with long-term headless agent readiness. MonKey explicitly structures features to eliminate moving targets: a bulletproof CLI tool is delivered in Milestone 1, laying the foundation for an AI Agent Status Daemon in Milestone 2 and a Tauri v2 Desktop GUI in Milestone 3.

**Must have (table stakes for v1):**
- **Multi-Axis Device Discovery (`monkey info`)** — Enumerate VID `0x05AC`, PID `0x024F`, OEM string `RKGK890`, and identify both Interface A (`0xFF68`) and Interface B (`0xFFFF`).
- **Safe Capability Probing (`monkey probe`)** — Non-destructive query of firmware version, hardware revision, and connection mode (wired vs 2.4GHz dongle) without modifying state.
- **Static LCD Frame Rendering (`monkey lcd image`)** — Ingest standard image formats, resize to 128x128, convert to RGB565 with Floyd-Steinberg dithering, and deliver 8x 4096-byte chunks in <50ms.
- **Basic LCD Animation Playback (`monkey lcd anim`)** — Stream GIF/APNG animations at paced 10–15 FPS with double-buffering, backpressure regulation, and graceful Ctrl+C interruption.
- **Ambient RGB Preset Control (`monkey rgb set`)** — Verified single-packet command (`04 13`) to configure hardware animation modes, speed, brightness, and static hex colors.
- **Protocol Safety Rails & Opcode Whitelist** — Compile-time and runtime validation blocking unverified opcodes and known ISP/bootloader triggers (`0x7140`).
- **Diagnostic Benchmark Tool (`monkey bench`)** — Automated throughput and latency test suite measuring chunk ACK latency, frame rate ceilings, and feature report round-trips.
- **Machine-Readable CLI Output (`--json`)** — Consistent, versioned JSON output across all inspection and benchmark commands for downstream tooling.

**Should have (competitive differentiators):**
- **Dual-Interface Transport Multiplexing** — Isolates bulk LCD transfers (`0xFF68`, requires no macOS Input Monitoring TCC prompt) from configuration feature reports (`0xFFFF`).
- **Two-Tier State Sync (RAM Preview vs Flash Commit)** — Routes live slider and agent updates to volatile RAM (30Hz), debouncing flash commits (`04 02`) by 500ms to preserve SPI NOR flash endurance.
- **Perceptual Floyd-Steinberg Dithering** — Eliminates severe color banding on the 128x128 panel when converting 24-bit TrueColor images to 16-bit RGB565.
- **RGB Configuration Readback & Restore (`monkey rgb save / restore`)** — Decodes `04 F5` 9-frame readback table to back up and restore customized color setups.
- **ASCII/ANSI Keyboard Matrix Visualizer** — Terminal stdout visualizer mapping active lighting states across the 81-key matrix using `layout_81keys.json`.

**Defer (v2+ roadmap):**
- **AI Agent Ambient Status Daemon (Milestone 2)** — Background process listening to Claude Code hook events (`IDLE`, `THINKING`, `WAITING_FOR_YOU`) and rendering glanceable badges to the LCD and RGB.
- **Keymap Remap & Macro Engine (`04 11`, Milestone 2)** — Full custom keymapping and macro table persistence adhering to 4-byte action formats.
- **Tauri v2 Desktop Application (Milestone 3)** — Native macOS/cross-platform graphical application with interactive visual keymap and drag-and-drop LCD GIF management.
- **Speculative Bootloader / Firmware Flashing** — Anti-feature; permanently excluded to prevent hardware bricking.
- **High-FPS Video (>20 FPS) / Wireless LCD Streaming** — Anti-features; Full-Speed USB HID and BLE bandwidth constraints make these physically unviable.

---

### Architecture Approach

MonKey adopts a layered, UI-agnostic architecture centered in `crates/monkey-core`, ensuring that all protocol logic, safety gates, and transport abstractions remain 100% reusable across the `monkey-cli` binary today and the Tauri v2 GUI tomorrow. High-level commands flow from the application layer into `KeyboardDriver`, which serializes all transactions through a dedicated single-flight `CommandQueue`. This queue enforces hardware-mandated inter-packet delays (2–5ms for LCD chunks, 15ms for queries, 80ms for flash writes). Every packet must pass through a typestate `SafetyGate` that verifies magic bytes, whitelisted opcodes, and parameter boundaries before conversion into a `ValidatedPacket`. Outbound traffic is routed across a `DualInterfaceTransport` coordinator, dispatching 4096-byte OUT chunks to Interface A (`0xFF68`) and 64-byte feature reports to Interface B (`0xFFFF`).

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                   monkey-cli / Future Tauri v2 Desktop UI                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                 KeyboardDriver Session (Lifecycle, Reconnect)               │
│          ┌───────────────────────────────────────────────────────┐          │
│          │  CommandQueue (Single-Flight, Inter-Packet Delay)     │          │
│          └──────────────────────────┬────────────────────────────┘          │
├─────────────────────────────────────┼───────────────────────────────────────┤
│            SafetyGate & CapabilityMatrix (Opcode Whitelist)                 │
├─────────────────────────────────────┼───────────────────────────────────────┤
│    ProtocolCodec (64B Feature / 4096B Bulk / CRC16 / AA 55 Framing)        │
├─────────────────────────────────────┼───────────────────────────────────────┤
│       DualInterfaceTransport (hidapi / IOKit / MockTransport for CI)       │
│          ├── Interface A (UP 0xFF68) ──► 4096B OUT Report (LCD Chunks)      │
│          └── Interface B (UP 0xFFFF) ──► 64B Feature Report (Config / RGB)  │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Major components:**
1. **`monkey-cli`:** Command-line executable (`clap` v4 derive, `indicatif`, `tracing`) exposing `info`, `probe`, `lcd`, `rgb`, and `bench` subcommands with human and JSON output modes.
2. **`KeyboardDriver`:** High-level session coordinator managing device lifecycle, connection state machines (detecting phantom dongle sleep), and coordinating subsystem engines.
3. **`CommandQueue`:** Dedicated actor loop serializing all hardware operations into single-flight execution, enforcing hardware cooldown intervals to prevent MCU FIFO drops.
4. **`CapabilityMatrix & SafetyGate`:** Strongly-typed registry evaluating hardware profiles (`model + hw_rev + fw_ver + transport`) and typestate validator converting `UncheckedPacket` to `ValidatedPacket`.
5. **`ProtocolCodec`:** Zero-copy packet encoder/decoder handling 64-byte configuration packets (`0x04` magic, `AA 55` framing, checksums) and 4096-byte bulk display chunks.
6. **`DualInterfaceTransport`:** Unified transport coordinator managing distinct `hidapi::HidDevice` handles for Interface A and Interface B, with a pluggable `MockTransport` for deterministic CI tests.
7. **`LcdRenderingEngine`:** Image processing and streaming pipeline converting RGBA inputs to 128x128 RGB565, applying Floyd-Steinberg dithering, slicing into 8x 4096B chunks, and regulating 10–15 FPS pacing.
8. **`RgbLightingEngine`:** Lighting controller managing built-in presets (`04 13`), per-key matrix mapping from `layout_81keys.json` (`04 20`), and coordinating volatile RAM previews with debounced Flash commits.

---

### Critical Pitfalls

1. **Blind Opcodes & Accidental Bootloader/DFU Invocation (Permanent Hardware Bricking)**  
   *Risk:* Low-cost HFD/Sonix Cortex-M0 microcontrollers share the vendor HID dispatch table with internal ISP bootloader routines (e.g., PID `0x7140`) and flash erase registers. Sending unverified or fuzzed byte sequences can trigger mass erase or corrupt the boot vector, permanently bricking the board.  
   *How to avoid:* Enforce an immutable schema-driven opcode whitelist in `SafetyGate`. Never permit blind probing. Only dispatch opcodes verified through differential USB captures (`04 18`, `04 13`, `04 20`, `04 02`, `04 F0`). Quarantined bootloader vectors must be blocked at compile time.

2. **High-Frequency Flash Commit Exhaustion & Low-Battery Brownout**  
   *Risk:* Onboard SPI NOR flash supports only 10,000 to 100,000 write cycles per sector. Committing live RGB slider tweaks or continuous status updates directly to flash burns out storage within days. Executing flash writes on battery power below 20% induces voltage sag and MCU brownout, corrupting partition tables.  
   *How to avoid:* Implement a two-tier state model: Tier 1 (RAM Preview) streams volatile mode updates (`04 13`) throttled at 30Hz without flash writes; Tier 2 (Flash Commit) debounces save commands (`04 02`) by 500ms after user interaction settles. Enforce an automatic flash write lockout if battery is below 20% on wireless modes.

3. **macOS TCC Input Monitoring Collisions & App Sandbox Lockdown**  
   *Risk:* Opening the composite keyboard endpoint via standard APIs fails with `kIOReturnNotPermitted` (`0xE00002E2`) or triggers intrusive macOS "Input Monitoring" keylogger warnings. Furthermore, App Sandbox blocks unentitled USB access.  
   *How to avoid:* Exploit dual-interface physical separation. Target Interface A (`Usage Page 0xFF68`) for all high-throughput LCD transfers; vendor pages are exempt from TCC prompts. For Interface B (`Usage Page 0xFFFF`), compile `hidapi` with `macos-shared-device` (`hid_darwin_set_open_exclusive(0)`) to avoid locking Apple's kernel keyboard driver. Configure sandbox USB entitlements (`com.apple.security.device.usb`).

4. **USB Full-Speed Bus Saturation & MCU FIFO Overrun during Bulk LCD Streaming**  
   *Risk:* A 128x128 RGB565 frame requires 32,768 bytes. The hardware uses 4096-byte OUT reports (8 chunks per frame). Flooding these chunks unthrottled over Full-Speed USB (12 Mbps) saturates the bus, starves the 1ms keyboard scanning endpoint (causing dropped/laggy keystrokes), and overflows the MCU's internal SPI DMA buffer, producing screen tearing or USB disconnects.  
   *How to avoid:* Cap animation playback at 10–15 FPS (66ms–100ms per frame). Enforce strict 2–5ms inter-chunk pacing delays. Maintain a half-duplex single-flight queue preventing RGB feature reports from interleaving with LCD chunk bursts.

5. **Incomplete Protocol Transactions & Firmware State Locking (`04 18` $\rightarrow$ `04 02` $\rightarrow$ `04 F0`)**  
   *Risk:* The HFD firmware implements a multi-packet transaction lifecycle: Start (`04 18`) $\rightarrow$ Data (`04 13 / 04 20`) $\rightarrow$ Commit (`04 02`) $\rightarrow$ End (`04 F0`). If a client panics, crashes, or drops connection mid-transfer without sending `04 F0`, the MCU remains trapped in its programming loop, freezing physical key scanning until physically replugged.  
   *How to avoid:* Wrap all transactions in an RAII `TransactionGuard` struct implementing Rust's `Drop` trait. `TransactionGuard` guarantees transmission of `04 F0` even on thread panics, I/O errors, or Ctrl+C cancellations.

---

## Implications for Roadmap

Based on research dependencies, risk reduction priorities, and architectural boundaries, the implementation roadmap is structured into 5 cohesive phases:

### Phase 1: Workspace Architecture, Transport Foundation & Device Probing
**Rationale:** Establishing the dual-interface transport layer and verified enumeration models must precede any packet writes. Proving that `0xFF68` (LCD) and `0xFFFF` (Config) can be opened non-exclusively on macOS without triggering TCC permission denials validates the core platform thesis.  
**Delivers:**
- Cargo workspace with `crates/monkey-core` and `crates/monkey-cli`.
- `HidTransport` trait abstraction with production `HidapiTransport` (`macos-shared-device`) and deterministic `MockTransport`.
- Multi-axis device identification model (`model`, `hw_rev`, `fw_ver`, `transport`, `capabilities`).
- Connection state machine with active heartbeat ping to detect phantom 2.4GHz wireless dongle sleep states.
- `monkey info` and `monkey probe` CLI commands with human and `--json` outputs.  
**Addresses:** Table stakes device discovery, capability probing, dual-interface transport multiplexing.  
**Avoids:** Pitfall 3 (macOS TCC collisions), Pitfall 6 (Phantom dongle deception), Pitfall 7 (Report ID byte shift).

### Phase 2: Protocol Codecs, Transaction Safety Rails & Benchmark Harness
**Rationale:** Before sending write commands to physical hardware, the safety gate and transaction lifecycles must be mathematically enforced in code. Building the benchmark harness immediately afterward allows measuring real-world USB packet timings and establishing inter-packet delay baselines on physical hardware.  
**Delivers:**
- Zero-copy packet encoders/decoders for 64-byte configuration packets and 4096-byte bulk chunks (`zerocopy`).
- Checksum algorithms (One's Complement, Additive 16-bit, CRC16) and framing markers (`0x04`, `AA 55`).
- `CapabilityMatrix` registry and typestate `SafetyGate` validator (`UncheckedPacket` $\rightarrow$ `ValidatedPacket`).
- Immutable opcode whitelist blocking ISP bootloader vectors (`0x7140`) and unmapped write commands.
- RAII `TransactionGuard` ensuring atomic transaction completion (`04 18` $\rightarrow$ `04 F0`) across error paths.
- Single-flight serialized `CommandQueue` with configurable delay profiles.
- `monkey bench` diagnostic subcommand measuring chunk ACK latency, frame rate ceilings, and bus jitter.  
**Addresses:** Protocol safety rails, schema-driven opcode whitelist, diagnostic benchmarking.  
**Avoids:** Pitfall 1 (Bootloader invocation / bricking), Pitfall 5 (Incomplete transaction state locking), Pitfall 8 (Passive feature scan deadlock).

### Phase 3: High-Performance LCD Rendering & Streaming Engine
**Rationale:** The 128x128 color display is MonKey's primary visual feature and carries the heaviest I/O load (32,768 bytes per frame). Developing the image processing and chunking pipeline after the benchmark harness ensures pacing parameters (2–5ms inter-chunk delay, 10–15 FPS) are tuned against measured hardware capabilities.  
**Delivers:**
- RGB888 to RGB565 endian-safe converter with perceptual Floyd-Steinberg error diffusion dithering.
- 32,768-byte frame chunker generating 8x 4096-byte OUT reports for Interface A (`0xFF68`).
- `LcdStreamer` rate regulator maintaining 10–15 FPS with double-buffering and frame-drop backpressure.
- Diagnostic test pattern generator (`monkey lcd test-pattern`) rendering pure R/G/B and grayscale ramps to verify pixel endianness.
- `monkey lcd image <file>` for static image rendering (<50ms delivery target).
- `monkey lcd anim <file/dir>` for fluid GIF/APNG playback with graceful interruption (Ctrl+C).  
**Addresses:** Static LCD rendering, basic animation playback, Floyd-Steinberg dithering.  
**Avoids:** Pitfall 4 (USB bus saturation & keystroke lag), Pitfall 7 (RGB565 endianness inversion & color channel swap).

### Phase 4: Ambient RGB Engine, Matrix Mapping & Two-Tier State Persistence
**Rationale:** RGB configuration requires both single-packet mode changes (`04 13`) and 81-key matrix mappings (`04 20`), as well as protecting SPI NOR flash endurance. Implementing this after the core driver session and command queue are proven ensures safe flash wear throttling.  
**Delivers:**
- Ambient lighting controller supporting built-in modes, speed, brightness, and static hex colors (`04 13`).
- Per-key matrix lighting mapper using `layout_81keys.json` to translate physical keys to hardware LED indices (`04 20`).
- Two-tier state synchronization: ephemeral RAM Preview (throttled at 30Hz) vs Debounced Flash Commit (500ms settle after last interaction).
- Low-battery safety lockout blocking flash writes if battery is below 20% on wireless modes.
- Non-destructive configuration snapshot and restore (`monkey rgb save / restore`) decoding the `04 F5` 9-frame readback table.
- Terminal ANSI 75% matrix visualizer displaying active lighting states in stdout.
- `monkey rgb set`, `monkey rgb save`, and `monkey rgb restore` CLI subcommands.  
**Addresses:** Ambient RGB preset control, RGB configuration readback/restore, per-key RGB, terminal ANSI matrix visualizer.  
**Avoids:** Pitfall 2 (Flash commit wearout & brownout), Pitfall 8 (Readback verification false failures).

### Phase 5: Production Hardening, Packaging & Release Readiness
**Rationale:** Final polish guarantees that MonKey meets production distribution standards, provides clear troubleshooting diagnostics for end users, and validates App Sandbox readiness for future Tauri v2 integration.  
**Delivers:**
- Pre-flight diagnostic command (`monkey doctor`) checking USB connectivity, permissions, and OS driver states.
- Standardized POSIX exit codes and actionable terminal error messages with remediation steps.
- Shell auto-completions (bash, zsh, fish) generated via `clap_complete`.
- macOS App Sandbox entitlement verification and end-to-end integration test suite on `MockTransport`.
- `cargo-deny` audit configuration ensuring MIT/Apache-2.0 license compliance and zero security advisories.
- Fully documented CLI manual, README usage guides, and release build workflows.  
**Addresses:** Predictable error codes, sandbox readiness, developer/user documentation.  
**Avoids:** Pitfall 3 (App Sandbox lockdown), technical debt shortcuts.

---

### Phase Ordering Rationale

- **Transport First (Phase 1):** You cannot safely test protocols or reverse-engineer hardware without reliable, non-exclusive OS handles. Establishing `HidTransport` and `MockTransport` first unlocks 100% automated CI testability for all subsequent phases.
- **Safety Rails Before Writes (Phase 2):** Hardware safety cannot be an afterthought. Implementing `SafetyGate`, the opcode whitelist, and the RAII `TransactionGuard` before implementing write commands prevents accidental bricking during live development.
- **LCD Pipeline Before RGB Config (Phase 3 before Phase 4):** Interface A (`0xFF68` bulk display) is simpler from a protocol state-machine perspective (pure OUT data chunks, no multi-packet transaction sequences) but has the highest bandwidth demands. Validating bulk data delivery first proves USB bus stability before tackling complex multi-packet feature reports on Interface B.
- **State Persistence Last (Phase 4):** Two-tier RAM/Flash synchronization requires a fully functioning command queue, session lifecycle, and battery probe logic. Building it after the core engines are functional prevents premature flash wear during debugging.
- **Hardening & Sandbox Polish (Phase 5):** Packaging, shell completions, and sandbox entitlement validation build upon a frozen, stable core API.

---

### Research Flags

**Phases likely needing deeper research during planning:**
- **Phase 2 (Protocol Codecs & Safety Rails):** While the command structures (`04 18`, `04 13`, `04 20`, `04 02`, `04 F0`) are verified from prior art, the exact checksum algorithm variant (One's Complement vs Additive 16-bit vs CRC16) across different firmware revisions of the HFD/RKGK890 requires byte-level validation during codec test construction.
- **Phase 4 (RGB Engine & State Persistence):** The `04 F5` readback report returns a 9-frame payload. The exact byte mapping of internal PWM gamma curves and analog duty cycles requires capture verification against `MK3075ProDriver V1.0.exe` to ensure write-verification routines do not produce false rollback errors.

**Phases with standard patterns (skip research-phase):**
- **Phase 1 (Workspace & Transport Foundations):** Cargo workspace layout, `clap` CLI boilerplate, `hidapi` multi-interface opening with `macos-shared-device`, and `MockTransport` mocking patterns are standard, well-documented Rust practices.
- **Phase 3 (LCD Rendering & Streaming):** Image downscaling, Floyd-Steinberg error diffusion algorithms, RGB565 byte packing, and timer-paced chunk iteration follow established graphics and embedded engineering patterns.
- **Phase 5 (Hardening & Release Readiness):** Shell completion generation, `cargo-deny` configuration, and POSIX exit code standardization are standard Rust release engineering tasks.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|:----------:|-------|
| **Stack** | **HIGH** | `hidapi` 2.6.7, `zerocopy` 0.8.57, and `embedded-graphics` 0.8.2 are verified through crates.io releases and physical IOKit tests. Dedicated worker thread architecture cleanly solves async/sync impedance without Tokio bloat. |
| **Features** | **HIGH** | Feature scope is grounded in verified hardware captures (`device.xml`, `layout_81keys.json`), physical WebHID measurements on macOS, and comparative analysis of working implementations (`rcsn01/GMK-67-Driver`). Clear milestone boundaries prevent scope creep. |
| **Architecture** | **HIGH** | Layered separation (`monkey-core` vs `monkey-cli`), typestate `SafetyGate`, `DualInterfaceTransport`, single-flight command queue, and two-tier RAM/Flash sync directly address the physical hardware constraints of the Monka 3075 Pro. |
| **Pitfalls** | **HIGH** | All major pitfalls (bootloader bricking, flash exhaustion, macOS TCC permissions, USB bus saturation, transaction locking, RGB565 endianness) are documented with exact technical root causes and concrete avoidance architectures. |

**Overall confidence: HIGH**

### Gaps to Address

- **Checksum Algorithm Exact Formula:** HFD feature report headers use 2 checksum bytes at offset 62..63. Different OEM firmwares alternate between an additive 16-bit sum, 1's complement sum, or CRC16-CCITT.  
  *Resolution:* Implement an algorithmic checksum detector in Phase 2 unit tests that validates against known valid captured packets (`04 18`, `04 13`, `04 02`).
- **Firmware RTC / Time Sync Opcode:** Whether the Monka 3075 Pro MCU supports setting the onboard LCD clock via USB feature report (as seen on Ajazz AK820 Pro `0x51` packets) remains unverified for this specific firmware.  
  *Resolution:* Marked as optional P2 feature; probe non-destructively in Phase 4 without blocking LCD image/animation release.
- **App Sandbox IOKit Matching Strings:** Exact `com.apple.security.device.usb` entitlement configuration for Mac App Store-compliant Tauri v2 distribution.  
  *Resolution:* Fully addressed in Phase 5 hardening tests using clean sandbox container builds.

---

## Sources

### Primary (HIGH confidence)
- **Monka 3075 Pro Physical Hardware Measurements (macOS Sequoia 15.x):** Direct WebHID and IOKit descriptor dumps verifying Interface A (`0xFF68`, OUT 4096 / IN 64) and Interface B (`0xFFFF`, co-resident Consumer/Mouse).
- **Vendor Driver Binaries & Manifests:** Extracted `device.xml` (`RKGK890`, `05AC:024F`, Shenzhen HFD Technology), `KeyboardLayout.xml`, and disassembled MFC command dispatchers from `MK3075ProDriver V1.0.exe`.
- **`rcsn01/GMK-67-Driver`:** Production reverse-engineering on identical `05AC:024F / RKGK890` hardware verifying `04 18` (start), `04 13` (ambient mode), `04 20` (per-key RGB table), `04 02` (save), `04 F0` (end), and `04 F5` (readback).
- **`crates.io` Package Registries:** Verified releases and feature flags for `hidapi` (2.6.7), `zerocopy` (0.8.57), `clap` (4.6.6), `image` (0.25.10), `embedded-graphics` (0.8.2), and `crossbeam-channel` (0.5.17).

### Secondary (MEDIUM confidence)
- **`wsclx/ak820pro-modder` & `Aiacos/ajazz-control-center`:** Analysis of 128x128 TFT RGB565 animation structures, chunking mechanics, and host-side pacing considerations on related OEM keyboard families.
- **OpenRGB & Sonix-QMK Projects:** Technical documentation on Sonix/HFD SN32F248B Cortex-M0 microcontrollers, ISP bootloader PID `0x7140` collision vectors, and SPI NOR flash endurance limits.
- **Apple Developer Documentation:** `IOHIDManager` C API specifications, macOS TCC Input Monitoring security policies (`kTCCServiceListenEvent`), and App Sandbox hardware entitlements.

### Tertiary (LOW confidence)
- **Community Forum Reverse-Engineering Notes:** Speculative Ajazz `0x51` RTC clock sync opcodes (requires differential hardware validation before adoption).

---
*Research completed: 2026-09-13*  
*Ready for roadmap: yes*
