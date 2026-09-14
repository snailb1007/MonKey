# MonKey (MonkaKeyboard)

## What This Is

MonKey is an open-source, cross-platform driver and tooling ecosystem written in Rust for the Monka 3075 Pro mechanical keyboard (and related HFD/RKGK890 OEM boards). It decouples the core hardware driver from the UI by delivering a robust CLI first, providing safe hardware probing, verified protocol execution for LCD frame rendering and RGB lighting, with an architecture designed to guarantee macOS App Sandbox compatibility and eventual Tauri v2 UI integration.

## Core Value

Safe, verified hardware communication and reliable device capability negotiation without risky OEM protocol assumptions, delivering predictable performance for LCD display and RGB controls.

## Requirements

### Validated

- [x] Multi-axis device identification model (`model` + `hardware revision` + `firmware version` + `transport` + `capabilities`) (Phase 1)
- [x] Safe transport abstraction layer (macOS hidapi / IOKit) with strict sandbox awareness, cleanly separating the vendor bulk LCD interface (`0xFF68`) from configuration feature reports (`0xFFFF`) (Phase 1)
- [x] Protocol safety rails and schema-driven capability matrix: execute only verified opcodes/payloads, blocking unverified or high-risk flash/bootloader writes (Phase 2)
- [x] High-performance 128x128 RGB565 LCD rendering engine: static frame transfers (<100ms total delivery: <15ms host + <75ms transport) and animation playback (10-15 FPS) without HID bus contention (Phase 3)
- [x] RGB lighting configuration and effect controls via validated feature reports with two-tier RAM preview and debounced flash commits (Phase 4)
- [x] Modular Rust workspace layout: `crates/monkey-core` (driver, protocol, transport) and `crates/monkey-cli` (diagnostics, testing, benchmarking, controls) (Phase 1)
- [x] Diagnostic CLI commands: `info` (probe & capability dump), `lcd` (display test, image/frame rendering), `rgb` (backlight control), `bench` (transport throughput & latency benchmarks), `doctor` (pre-flight diagnostics), and `completions` (shell completion scripts) (Phases 1-5)

### Active

(Milestone v1.0 complete; Milestone v2.0 planning next)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Decouple core driver from UI; build CLI first | Isolates hardware communication, simplifies protocol reverse-engineering and benchmarking before GUI complexity | Validated (v1.0) |
| Device abstraction: `model + hw rev + fw ver + transport + capabilities` | Prevents fragile hardcoded assumptions across different hardware batches and OEM firmware variations | Validated (v1.0) |
| Schema-driven capability matrix with safety gate | Eliminates bricking risks by whitelisting only capture-verified packets and isolating dangerous commands | Validated (v1.0) |
| macOS App Sandbox readiness in core driver | Ensures seamless transition to Tauri v2 and compliant macOS distribution | Validated (v1.0) |
| Target 10-15 FPS for LCD animations | Perfectly balances fluid visual status/GIFs with USB HID bandwidth and system resource usage | Validated (v1.0) |
| Two-tier volatile RAM preview vs debounced flash commits | Protects onboard SPI NOR flash from premature wear during interactive slider adjustments | Validated (v1.0) |
| Standardized POSIX exit codes (0-5) | Enables robust shell scripting and automated deployment diagnostics | Validated (v1.0) |

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
