# MonKey (MonkaKeyboard)

## What This Is

MonKey is an open-source, cross-platform driver and tooling ecosystem written in Rust for the Monka 3075 Pro mechanical keyboard (and related HFD/RKGK890 OEM boards). It decouples the core hardware driver from the UI by delivering a robust CLI first, providing safe hardware probing, verified protocol execution for LCD frame rendering and RGB lighting, with an architecture designed to guarantee macOS App Sandbox compatibility and eventual Tauri v2 UI integration.

## Core Value

Safe, verified hardware communication and reliable device capability negotiation without risky OEM protocol assumptions, delivering predictable performance for LCD display and RGB controls.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Multi-axis device identification model (`model` + `hardware revision` + `firmware version` + `transport` + `capabilities`)
- [ ] Safe transport abstraction layer (macOS hidapi / IOKit) with strict sandbox awareness, cleanly separating the vendor bulk LCD interface (`0xFF68`) from configuration feature reports (`0xFFFF`)
- [ ] Protocol safety rails and schema-driven capability matrix: execute only verified opcodes/payloads, blocking unverified or high-risk flash/bootloader writes
- [ ] High-performance 128x128 RGB565 LCD rendering engine: static frame transfers (<100ms total delivery: <15ms host + <75ms transport) and animation playback (10-15 FPS) without HID bus contention
- [ ] RGB lighting configuration and effect controls via validated feature reports
- [ ] Modular Rust workspace layout: `crates/monkey-core` (driver, protocol, transport) and `crates/monkey-cli` (diagnostics, testing, benchmarking, controls)
- [ ] Diagnostic CLI commands: `info` (probe & capability dump), `lcd` (display test, image/frame rendering), `rgb` (backlight control), and `bench` (transport throughput & latency benchmarks)

### Out of Scope

- Direct GUI / Tauri application in v1 — deferred to a subsequent milestone after core driver and CLI stability are proven
- Guessing unverified bootloader / DFU / flash-write opcodes — strictly prohibited to prevent hardware bricking
- Real-time 30+ FPS video streaming over USB HID — LCD is intended for ambient glanceable status and lightweight animations (10-15 FPS), avoiding USB bus saturation
- AI Agent Daemon (Claude Code hook state machine) — prioritized for Milestone 2 once the core driver and CLI display transport are fully validated

## Context

- **Hardware Target**: Monka 3075 Pro (75% layout, 81 keys), Shenzhen HFD Technology (`RKGK890`), spoofed Apple VID/PID `0x05AC:0x024F`, 128x128 RGB565 TFT LCD screen.
- **Dual HID Interfaces**:
  - Interface A (`Usage Page 0xFF68`, `Usage 0x61`): Dedicated bulk/vendor collection (4096-byte OUT report, 64-byte IN report) used for display frame chunks (`128*128*2 = 32768` bytes = 8 chunks of 4096 bytes). Does not collide with protected HID collections, enabling unprompted access.
  - Interface B (`Usage Page 0xFFFF` + `Consumer/Mouse`): Shares endpoint with protected collections. Feature reports (64 bytes) handle RGB and configuration, requiring proper entitlements/permissions on macOS.
- **Protocol Corrections**: Previous community research contains unchecked assumptions about Ajazz/GMK-67 packet formats. The core driver must replace speculative packet structures with strictly verified captures and runtime capability negotiation.
- **Sandbox Architecture**: While the standalone CLI runs natively in terminal, `monkey-core` must adhere to macOS App Sandbox rules and permission models so it can be embedded directly in a future Tauri v2 desktop app.

## Constraints

- **Language & Runtime**: 100% Rust for core driver and CLI for memory safety, concurrency, and performance.
- **OS Compatibility**: macOS first (primary development target), designed with cross-platform abstractions (Linux/Windows via `hidapi`).
- **Hardware Safety**: Zero blind/speculative writes to flash. Read before write; RAM buffers before flash commits.
- **Performance**: Static LCD frame transfer under 100ms total latency (<15ms host + <75ms transport); animation throughput stable at 10-15 FPS without dropped reports.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Decouple core driver from UI; build CLI first | Isolates hardware communication, simplifies protocol reverse-engineering and benchmarking before GUI complexity | — Pending |
| Device abstraction: `model + hw rev + fw ver + transport + capabilities` | Prevents fragile hardcoded assumptions across different hardware batches and OEM firmware variations | — Pending |
| Schema-driven capability matrix with safety gate | Eliminates bricking risks by whitelisting only capture-verified packets and isolating dangerous commands | — Pending |
| macOS App Sandbox readiness in core driver | Ensures seamless transition to Tauri v2 and compliant macOS distribution | — Pending |
| Target 10-15 FPS for LCD animations | Perfectly balances fluid visual status/GIFs with USB HID bandwidth and system resource usage | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-09-13 after initialization*
