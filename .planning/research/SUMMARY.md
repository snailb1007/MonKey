# Project Research Summary

**Project:** MonKey (MonkaKeyboard)  
**Domain:** Custom Keyboard Hardware Driver & CLI Tooling (Rust / macOS USB HID / 128x128 RGB565 LCD Streaming / Feature Report RGB)  
**Researched:** 2026-09-13  
**Confidence:** MEDIUM overall; evidence is mixed and protocol claims are not capture-verified on Monka 3075 Pro

> **Evidence boundary:** The repository contains OEM manifests and research notes, but no raw USB capture (`.pcap`/`.pcapng`). Claims below therefore distinguish directly inspectable OEM metadata from WebHID observations recorded in prose, prior art, and hardware-family inference. Q1–Q4 in `research/capture_plan.md` remain open until captures and persistence tests are added.

---

## Executive Summary

MonKey is an open-source, cross-platform userspace driver and diagnostic CLI ecosystem written in Rust for the Monka 3075 Pro mechanical keyboard. The local OEM metadata identifies Shenzhen HFD Technology `RKGK890` and the spoofed Apple VID/PID `0x05AC:0x024F`; see `vendor_driver/device.xml`. Research notes report two macOS HID entries—a standalone vendor collection (`Usage Page 0xFF68`, `Usage 0x61`) and a composite configuration collection (`Usage Page 0xFFFF`, `Usage 0x01`)—but the repository has no raw USB capture to independently reproduce those observations. The 4096-byte LCD path and 64-byte configuration path are consequently implementation hypotheses until a capture confirms them on the target board.

The recommended architectural approach is a modular Rust workspace (`crates/monkey-core` and `crates/monkey-cli`) operating directly through Apple's `IOHIDManager` via `hidapi` (compiled with `macos-shared-device`). Rather than binding the driver to an asynchronous runtime like Tokio, `monkey-core` proposes a synchronous core driver API with a dedicated OS hardware worker thread communicating across `crossbeam-channel` queues. The 2–5ms LCD, 15ms query, and 80ms Flash intervals are design targets, not measurements. The LCD rendering pipeline couples `image` for asset ingestion with `embedded-graphics` for procedural badge drawing and Floyd-Steinberg error diffusion to eliminate 16-bit color banding.

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
- **Multi-Axis Device Discovery (`monkey info`)** — Enumerate the OEM-confirmed VID `0x05AC`, PID `0x024F`, and identifier `RKGK890`; interface discovery for `0xFF68`/`0xFFFF` is a reported observation pending raw descriptor/capture archival.
- **Safe Capability Probing (`monkey probe`)** — Non-destructive query of firmware version, hardware revision, and connection mode (wired vs 2.4GHz dongle) without modifying state.
- **Static LCD Frame Rendering (`monkey lcd image`)** — Ingest standard image formats and convert to 128x128 RGB565; the reported 8 × 4096-byte transfer is a candidate design pending capture validation on Monka 3075 Pro.
- **Basic LCD Animation Playback (`monkey lcd anim`)** — Stream GIF/APNG animations at a proposed 10–15 FPS, pending confirmation of the target board's LCD transport, buffering, and persistence behavior.
- **Ambient RGB Preset Control (`monkey rgb set`)** — Candidate implementation based on the GMK-67 prior-art command `04 13`; not verified on Monka 3075 Pro.
- **Protocol Safety Rails & Opcode Whitelist** — Compile-time and runtime validation blocking unverified opcodes and known ISP/bootloader triggers (`0x7140`).
- **Diagnostic Benchmark Tool (`monkey bench`)** — Automated throughput and latency test suite measuring chunk ACK latency, frame rate ceilings, and feature report round-trips.
- **Machine-Readable CLI Output (`--json`)** — Consistent, versioned JSON output across all inspection and benchmark commands for downstream tooling.

**Should have (competitive differentiators):**
- **Dual-Interface Transport Multiplexing** — Separates the two paths reported by `research/protocol_notes.md`; the `0xFF68` bulk and `0xFFFF` configuration details remain pending raw-capture confirmation.
- **Two-Tier State Sync (RAM Preview vs Flash Commit)** — Proposed model for live updates and explicit persistence; whether `04 02` commits Flash on Monka 3075 Pro is unverified.
- **Perceptual Floyd-Steinberg Dithering** — Eliminates severe color banding on the 128x128 panel when converting 24-bit TrueColor images to 16-bit RGB565.
- **RGB Configuration Readback & Restore (`monkey rgb save / restore`)** — Candidate design based on prior-art `04 F5`; response shape and persistence behavior are unverified on Monka 3075 Pro.
- **ASCII/ANSI Keyboard Matrix Visualizer** — Terminal stdout visualizer mapping active lighting states across the 81-key matrix using `layout_81keys.json`.

**Defer (v2+ roadmap):**
- **AI Agent Ambient Status Daemon (Milestone 2)** — Background process listening to Claude Code hook events (`IDLE`, `THINKING`, `WAITING_FOR_YOU`) and rendering glanceable badges to the LCD and RGB.
- **Keymap Remap & Macro Engine (`04 11`, Milestone 2)** — Full custom keymapping and macro table persistence adhering to 4-byte action formats.
- **Tauri v2 Desktop Application (Milestone 3)** — Native macOS/cross-platform graphical application with interactive visual keymap and drag-and-drop LCD GIF management.
- **Speculative Bootloader / Firmware Flashing** — Anti-feature; permanently excluded to prevent hardware bricking.
- **High-FPS Video (>20 FPS) / Wireless LCD Streaming** — Anti-features; Full-Speed USB HID and BLE bandwidth constraints make these physically unviable.

---

### Architecture Approach

MonKey proposes a layered, UI-agnostic architecture centered in `crates/monkey-core`, ensuring that protocol logic, safety gates, and transport abstractions remain reusable across the `monkey-cli` binary today and the Tauri v2 GUI tomorrow. High-level commands flow from the application layer into `KeyboardDriver`, which serializes all transactions through a dedicated single-flight `CommandQueue`. The queue will use configurable pacing targets once hardware measurements exist. Every packet must pass through a typestate `SafetyGate` that verifies magic bytes, an evidence-backed opcode whitelist, and parameter boundaries before conversion into a `ValidatedPacket`. The proposed `DualInterfaceTransport` will route traffic according to interfaces confirmed by target-board captures; the currently reported `0xFF68`/`0xFFFF`, 4096-byte, and 64-byte values are not yet established facts.

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
5. **`ProtocolCodec`:** Proposed zero-copy packet encoder/decoder for candidate 64-byte configuration packets (`0x04` magic, `AA 55` framing) and candidate 4096-byte bulk display chunks; packet sizes and framing require captures.
6. **`DualInterfaceTransport`:** Proposed transport coordinator managing distinct `hidapi::HidDevice` handles after Interface A/B identities and capabilities are confirmed, with a pluggable `MockTransport` for deterministic CI tests.
7. **`LcdRenderingEngine`:** Image-processing and streaming pipeline converting RGBA inputs to 128x128 RGB565; chunk count, chunk size, endianness, and 10–15 FPS pacing remain target-board validation items.
8. **`RgbLightingEngine`:** Proposed lighting controller using `layout_81keys.json`; built-in presets (`04 13`), per-key tables (`04 20`), and RAM/Flash behavior are prior-art hypotheses until Monka captures and persistence tests confirm them.

---

### Critical Pitfalls

1. **Blind Opcodes & Accidental Bootloader/DFU Invocation (Permanent Hardware Bricking)**  
   *Risk:* Low-cost HFD/Sonix Cortex-M0 microcontrollers share the vendor HID dispatch table with internal ISP bootloader routines (e.g., PID `0x7140`) and flash erase registers. Sending unverified or fuzzed byte sequences can trigger mass erase or corrupt the boot vector, permanently bricking the board.  
   *How to avoid:* Enforce an immutable schema-driven opcode whitelist in `SafetyGate`. Never permit blind probing. Dispatch target-board opcodes only after differential USB captures validate them (`04 18`, `04 13`, `04 20`, `04 02`, `04 F0`). Quarantined bootloader vectors must be blocked at compile time.

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
**Rationale:** Establishing the dual-interface transport layer and reported enumeration model must precede any packet writes. Validating whether `0xFF68` (LCD) and `0xFFFF` (Config) can be opened non-exclusively on macOS without triggering TCC permission denials is a Phase 1 hardware test, not an established research fact.
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
- **Phase 2 (Protocol Codecs & Safety Rails):** Candidate command structures (`04 18`, `04 13`, `04 20`, `04 02`, `04 F0`) are reported by prior art, not verified on Monka 3075 Pro. The exact checksum algorithm variant (One's Complement vs Additive 16-bit vs CRC16) requires byte-level validation after captures are archived.
- **Phase 4 (RGB Engine & State Persistence):** Prior art reports a `04 F5` readback with a 9-frame payload, but the response shape, byte mapping, and persistence behavior remain unverified on Monka. Capture and persistence testing against the OEM driver are required.

**Phases with standard patterns (skip research-phase):**
- **Phase 1 (Workspace & Transport Foundations):** Cargo workspace layout, `clap` CLI boilerplate, `hidapi` multi-interface opening with `macos-shared-device`, and `MockTransport` mocking patterns are standard, well-documented Rust practices.
- **Phase 3 (LCD Rendering & Streaming):** Image downscaling, Floyd-Steinberg error diffusion algorithms, RGB565 byte packing, and timer-paced chunk iteration follow established graphics and embedded engineering patterns.
- **Phase 5 (Hardening & Release Readiness):** Shell completion generation, `cargo-deny` configuration, and POSIX exit code standardization are standard Rust release engineering tasks.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|:----------:|-------|
| **Stack** | **HIGH** | Package versions and feature flags are checked against the documented registries; the worker-thread recommendation is a standard architecture choice, not physical-device evidence. |
| **Features** | **MEDIUM** | Scope is grounded in OEM identity/layout files and comparative research, but LCD transport, RGB opcodes, ACKs, and persistence are not supported by raw Monka captures. |
| **Architecture** | **MEDIUM** | Layering and safety boundaries are design recommendations; interface assignments, packet sizes, and timing parameters remain reported or INFERRED until target-board captures validate them. |
| **Pitfalls** | **MEDIUM** | Bricking, Flash wear, TCC, bus saturation, and byte-order risks are credible engineering risks, but target-board-specific causes and thresholds require validation. |

**Overall confidence: MEDIUM**

### Gaps to Address

- **Checksum Algorithm Exact Formula:** HFD feature report headers use 2 checksum bytes at offset 62..63. Different OEM firmwares alternate between an additive 16-bit sum, 1's complement sum, or CRC16-CCITT.  
  *Resolution:* Implement an algorithmic checksum detector in Phase 2 unit tests that validates candidate packets only after raw target-board captures (`.pcapng`) are archived; prior-art packets are not target-board evidence.
- **Firmware RTC / Time Sync Opcode:** Whether the Monka 3075 Pro MCU supports setting the onboard LCD clock via USB feature report (as seen on Ajazz AK820 Pro `0x51` packets) remains unverified for this specific firmware.  
  *Resolution:* Marked as optional P2 feature; probe non-destructively in Phase 4 without blocking LCD image/animation release.
- **App Sandbox IOKit Matching Strings:** Exact `com.apple.security.device.usb` entitlement configuration for Mac App Store-compliant Tauri v2 distribution.  
  *Resolution:* Fully addressed in Phase 5 hardening tests using clean sandbox container builds.

---

## Sources and Evidence Boundary

> **Reproducibility status:** The paths below are present as repository-local working-tree artifacts, but this project contains no raw USB capture (`.pcap`/`.pcapng`). A clean Git checkout can reproduce the planning text, not the uncommitted/ignored vendor and research inputs, unless those artifacts are separately supplied. No source below is evidence that Q1–Q4 in `research/capture_plan.md` have been answered.

### Directly inspectable OEM metadata (HIGH for identity/layout only)
- `vendor_driver/device.xml`: `VID=05AC`, `PID=024F`, product name `Gaming Keyboard`, OEM identifier `RKGK890`.
- `vendor_driver/KeyboardLayout.xml`: OEM key definitions and indices; `research/layout_81keys.json`: normalized 81-key matrix data.
- `research/device_info.json`: locally recorded device metadata; use as a cross-check, not as a substitute for a raw capture.

### Target-board observations recorded without raw capture (MEDIUM / reported)
- `research/protocol_notes.md`: prose notes of WebHID/interface observations. These are not independently replayable until the underlying descriptor dump or capture is archived.
- `research/webhid_tester.html`: measurement tool used for the notes; the tool itself is not a capture artifact.
- `research/capture_plan.md`: explicitly leaves Q1 (LCD RAM vs Flash), Q2 (feature-report interface), Q3 (RGB persistence), and Q4 (ACK behavior) open.

### Prior art and cross-family inference (INFERRED; not verified on Monka)
- `research/prior_art_protocol.md` and `rcsn01/GMK-67-Driver`: `04 18`, `04 13`, `04 20`, `04 02`, `04 F0`, and `04 F5` are prior-art commands observed on GMK-67/related hardware, not Monka 3075 Pro captures.
- `wsclx/ak820pro-modder` and `Aiacos/ajazz-control-center`: 128x128 RGB565, 4096-byte LCD chunking, ST7789-related behavior, and RTC `0x51` are Ajazz/Sonix-family references only. They remain hypotheses for Monka.
- `research/reverse_engineering_guide.md`, `research/via_vial_protocol_deepdive.md`, and OpenRGB references provide design context, not target-board protocol proof.

### Dependency and platform references (MEDIUM)
- `crates.io` package registries: library versions and feature flags.
- Apple IOKit/App Sandbox and USB HID specifications: platform constraints and API guidance.

**Overall confidence:** MEDIUM for the combined research; HIGH only for the directly inspectable OEM identity/layout metadata. Protocol implementation must remain gated until raw captures and the four capture-plan questions are resolved.

---
*Research completed: 2026-09-13*  
*Ready for roadmap: yes*
