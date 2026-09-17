# Requirements: MonKey (Monka 3075 Pro Tooling)

**Defined:** 2026-09-13
**Core Value:** Safe, verified hardware communication and reliable device capability negotiation without risky OEM protocol assumptions, delivering predictable performance for LCD display and RGB controls.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Discovery & Transport

- [x] **DISC-01**: System enumerates and isolates dual HID interfaces (Interface A `0xFF68`/bulk OUT 4096B vs Interface B `0xFFFF`/feature 64B) via `hidapi` with `macos-shared-device`.
- [x] **DISC-02**: User can inspect connected device identity via CLI (`monkey info` displaying VID, PID, Product Name, serial, interface mapping).
- [x] **DISC-03**: System probes device capability tuple (`model`, `hardware revision`, `firmware version`, `transport`, `capabilities`) via safe read-only queries (`monkey probe`).
- [x] **DISC-04**: System provides deterministic `MockTransport` simulating Monka 3075 Pro hardware for headless automated testing and CI.
- [x] **DISC-05**: System detects active connection transport state (wired USB vs 2.4GHz wireless dongle sleep/awake) via heartbeat query.

### Protocol Codecs & Safety Rails

- [x] **PROT-01**: Zero-allocation packet encoders/decoders (`zerocopy`) for 64-byte configuration feature reports and 4096-byte vendor HID OUT frame reports.
- [x] **PROT-02**: Checksum engine supporting 16-bit additive/one's complement and CRC verification for incoming and outgoing packets.
- [x] **PROT-03**: Strict `SafetyGate` typestate firewall enforcing a default-deny opcode whitelist (permits only verified opcodes e.g. `04 18`, `04 13`, `04 20`, `04 02`, `04 F0`, `04 F5`) and device enumeration isolation for dangerous ISP bootloader PIDs (`0x7140`).
- [x] **PROT-04**: Best-Effort RAII `TransactionGuard` with `ctrlc` signal interception ensuring session cleanup frame (`04 F0`) dispatch, backed by a standalone recovery command (`monkey reset`).
- [x] **PROT-05**: Serialized `CommandQueue` with inter-packet delay profiles (2ms–10ms) preventing USB controller lockups.

### LCD Display & Streaming

- [x] **LCD-01**: Image processing pipeline converts external image files (PNG/JPEG/BMP) to 128x128 16-bit RGB565 format with Floyd-Steinberg error diffusion dithering.
- [x] **LCD-02**: 8-chunk packetizer slices 32KB RGB565 frames into 8x 4096-byte OUT chunks delivered sequentially over Interface A (`0xFF68`).
- [x] **LCD-03**: User can render a static frame onto the LCD within <100ms total latency (<15ms host + <75ms transport) using `monkey lcd image <path>`.
- [x] **LCD-04**: User can stream animated GIF files or image sequences onto the LCD with host-regulated frame pacing at 10–15 FPS (`monkey lcd anim <path>`) (APNG deferred to v2).
- [x] **LCD-05**: User can run diagnostic LCD test patterns (RGB color bars, geometry alignment) to verify display endianness and wiring (`monkey lcd test-pattern`).

### Ambient RGB & State Management

- [x] **RGB-01**: User can configure ambient lighting mode, animation speed, brightness, and primary RGB color via verified `04 13` feature report (`monkey rgb set`).
- [x] **RGB-02**: Two-tier state persistence separating volatile 30Hz RAM previews from debounced (500ms) SPI Flash commits (`04 02`) to protect NOR flash endurance.
- [x] **RGB-03**: Safety lockout prevents persistent flash writes if battery level is below 20% to prevent brownout firmware corruption.
- [x] **RGB-04**: User can snapshot active lighting state via `04 F5` readback and restore previous profile (`monkey rgb save` / `monkey rgb restore`).

### Diagnostics & Benchmarking

- [x] **DIAG-01**: User can run transport throughput, chunk transfer latency, and feature report round-trip benchmarks with machine-readable JSON output (`monkey bench`).
- [x] **DIAG-02**: User can verify macOS USB permissions, sandboxing entitlements, and transport health via environment diagnostic check (`monkey doctor`).

### Architecture & Refactoring

- [x] **ARCH-01**: Clean architectural abstractions, pure transport trait with safe guard façade, unified MonkaDevice lifecycle with interface policy, elimination of env-var hacks, and zero test regressions.

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### AI Agent Status & Integrations

- **V2-DAEM**: Background agent status daemon consuming Claude Code / LLM hook events to drive ambient LCD animations and RGB status indications.
- **V2-TAURI**: Tauri v2 desktop application packaging `monkey-core` with native macOS App Sandbox entitlements.
- **V2-MACRO**: Onboard keymap remap and macro engine (`04 20` matrix writes).
- **V2-RTC**: Real-Time Clock (RTC) synchronization protocol uploading host timestamp to keyboard LCD.

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Direct GUI / Tauri application in v1 | Premature complexity; driver core and CLI protocol verification must stabilize first. |
| Guessing unverified bootloader / DFU opcodes | Extreme risk of permanently bricking keyboard hardware; only capture-verified packets are permitted. |
| 30–60 FPS video streaming over USB HID | Full-Speed USB (12 Mbps) cannot sustain >1 MB/s without dropping keystrokes and overflowing MCU buffers. |
| Bluetooth / BLE bulk LCD streaming | BLE bandwidth (< 2 KB/s) is physically incapable of streaming 32KB frames. |
| Unthrottled 60Hz reactive RGB | Overheats controller and induces typing jitter. |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| DISC-01 | Phase 1 | Complete |
| DISC-02 | Phase 1 | Complete |
| DISC-03 | Phase 1 | Complete |
| DISC-04 | Phase 1 | Complete |
| DISC-05 | Phase 1 | Complete |
| PROT-01 | Phase 2 | Complete |
| PROT-02 | Phase 2 | Complete |
| PROT-03 | Phase 2 | Complete |
| PROT-04 | Phase 2 | Complete |
| PROT-05 | Phase 2 | Complete |
| LCD-01 | Phase 3 | Complete |
| LCD-02 | Phase 3 | Complete |
| LCD-03 | Phase 3 | Complete |
| LCD-04 | Phase 3 | Complete |
| LCD-05 | Phase 3 | Complete |
| RGB-01 | Phase 4 | Complete |
| RGB-02 | Phase 4 | Complete |
| RGB-03 | Phase 4 | Complete |
| RGB-04 | Phase 4 | Complete |
| DIAG-01 | Phase 2 | Complete |
| DIAG-02 | Phase 5 | Complete |
| ARCH-01 | Phase 6 | Complete |

**Coverage:**

- v1 requirements: 22 total
- Mapped to phases: 22
- Unmapped: 0 ✓

---
*Requirements defined: 2026-09-13*
*Last updated: 2026-09-13 after roadmap creation*
