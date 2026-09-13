# Walking Skeleton — MonKey (Monka 3075 Pro Tooling)

**Phase:** 1
**Generated:** 2026-09-13

## Capability Proven End-to-End

A user can execute `cargo run -p monkey-cli -- probe --json` on macOS to inspect their connected Monka 3075 Pro keyboard's composite HID interfaces, hardware identifiers, and connection health without administrative privileges or destructive flash writes.

## Architectural Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Workspace Architecture | Virtual Cargo workspace (`crates/monkey-core`, `crates/monkey-cli`) | Decouples reusable hardware driver library from CLI presentation, preparing for future Tauri v2 GUI integration without code duplication. |
| USB HID Transport | `hidapi 2.6.7` with `macos-shared-device` feature flag | Apple kernel automatically claims HID devices (`AppleUserHIDDevice`). Raw USB (`rusb`/`nusb`) cannot claim them on macOS without DriverKit. `hidapi` communicates through `IOHIDManager`, and `macos-shared-device` (`hid_darwin_set_open_exclusive(0)`) prevents composite interface lockouts against OS keyboard drivers. |
| Interface Disambiguation | Primary `(UsagePage, Usage)` matching with interface fallback | macOS IOHIDManager uniquely disambiguates Interface A (`0xFF68:0x0061` bulk pipe) from Interface B (`0x000C:0x0001`/`0xFFFF` control pipe). Linux/Windows falls back to interface numbers. |
| Transport Abstraction | Synchronous `Transport: Send` trait | Models blocking hardware I/O with explicit timeout boundaries. Avoids dragging Tokio into `monkey-core`, ensuring deterministic inter-packet pacing. |
| Test Double Architecture | Deterministic `MockTransport` | Provides in-memory recording, canned feature report responses, and FIFO input queues, enabling 100% automated CI and headless unit/integration testing without physical hardware. |
| Packet Serialization | `zerocopy 0.8.57` with explicit little-endian types | Zero-heap-allocation transmutes for 64-byte feature reports and 4096-byte bulk chunks. Guarantees memory alignment safety on Apple Silicon ARM64 and x86_64. |
| Hardware Safety Rails | Zero-flash-commit invariant in Phase 1 | Restricts commands strictly to read-only descriptor inspection and benign queries. No sector erases, no speculative writes, and rejection of ISP bootloader devices (`0x7140`). |

## Stack Touched in Phase 1

- [ ] Project scaffold (virtual Cargo workspace, `crates/monkey-core`, `crates/monkey-cli`, dependencies, clippy/test runners)
- [ ] Hardware Transport Seam (`Transport` trait, typed `TransportError`, deterministic `MockTransport`)
- [ ] Userspace HID Driver (`HidTransport` wrapping `hidapi::HidDevice`, `macos-shared-device`, Report ID 0 userspace prefix handling)
- [ ] Dual-Interface Discovery (VID/PID `0x05AC:0x024F`, UsagePage `0xFF68` vs `0x000C`/`0xFFFF`, path-based device opening)
- [ ] Protocol Codec Core (zero-copy fixed structures, little-endian types, header validation)
- [ ] CLI User Interface (`monkey info` and `monkey probe` commands with human terminal formatting and `--json` structured output)
- [ ] Automated Test Suite (headless integration tests executing CLI commands against `MockTransport`)

## Out of Scope (Deferred to Later Slices)

- 128x128 RGB565 LCD image processing, dithering, and 8-chunk packetization (Phase 3)
- 10–15 FPS animated GIF streaming and frame pacing (Phase 3)
- Ambient RGB lighting adjustments, RAM preview updates, and SPI flash wear protection (Phase 4)
- 81-key matrix remapping and macro scripting (Phase 4 / v2)
- Compile-time and runtime `SafetyGate` opcode firewall and RAII `TransactionGuard` (Phase 2)
- Micro-benchmarking harness `monkey bench` (Phase 2)
- macOS App Sandbox packaging and `monkey doctor` environment diagnostic (Phase 5)
- Tauri v2 desktop GUI and background agent status daemon (v2)

## Subsequent Slice Plan

Each later phase adds one vertical slice on top of this skeleton without altering its architectural decisions:

- **Phase 2: Protocol Codecs, Transaction Safety Rails & Benchmark Harness** — Zero-copy packet encoders/decoders, CRC engine, compile-time `SafetyGate` opcode whitelist, RAII `TransactionGuard`, serialized `CommandQueue`, and `monkey bench`.
- **Phase 3: High-Performance LCD Rendering & Streaming Engine** — Image ingestion pipeline with Floyd-Steinberg dithering to 128x128 RGB565, 8-chunk packetizer over Interface A bulk OUT (`0xFF68`), static frame rendering, and 10–15 FPS animation playback.
- **Phase 4: Ambient RGB Engine, Matrix Mapping & Two-Tier State Persistence** — Lighting control via `04 13` feature reports, two-tier RAM (30Hz) vs Flash (500ms debounced) persistence, battery safety lockouts (<20%), and profile backup/restore.
- **Phase 5: Production Hardening, Packaging & Release Readiness** — `monkey doctor` macOS USB permission diagnostics, shell auto-completions, standardized POSIX exit codes, and `cargo-deny` security audit.
