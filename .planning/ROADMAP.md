# Roadmap: MonKey (Monka 3075 Pro Tooling)

## Overview

MonKey provides an open-source, safe, cross-platform driver and diagnostic CLI ecosystem for the Monka 3075 Pro mechanical keyboard (and related HFD/RKGK890 OEM hardware). This roadmap delivers a vertical MVP progression across five focused phases: establishing the core workspace and safe dual-interface USB HID transport, implementing zero-copy protocol codecs with strict hardware safety rails and benchmarking, delivering the 128x128 RGB565 LCD rendering and streaming engine, providing ambient RGB controls with two-tier flash wear protection, and hardening the entire CLI suite with environment diagnostics and release packaging.

## Phases

- [x] **Phase 1: Workspace Architecture, Transport Foundation & Device Probing** - Establish modular Rust workspace, safe dual-interface HID transport (Interface A `0xFF68` / Interface B `0xFFFF`), MockTransport, and read-only device probing. (completed 2026-09-13)
- [x] **Phase 2: Protocol Codecs, Transaction Safety Rails & Benchmark Harness** - Build zero-copy packet codecs, checksum engine, SafetyGate opcode whitelist, RAII TransactionGuard, CommandQueue, and benchmark tooling. (completed 2026-09-13)
- [x] **Phase 3: High-Performance LCD Rendering & Streaming Engine** - Ingest and dither images to 128x128 RGB565, slice into 8x 4096B chunks, and stream static images and 10–15 FPS animations over Interface A. (completed 2026-09-13)
- [x] **Phase 4: Ambient RGB Engine, Matrix Mapping & Two-Tier State Persistence** - Control lighting modes via verified feature reports, map 81-key matrix, protect NOR flash with two-tier RAM/Flash sync, and support profile backup/restore. (completed 2026-09-14)
- [x] **Phase 5: Production Hardening, Packaging & Release Readiness** - Deliver `monkey doctor` environment diagnostics, shell completions, standardized exit codes, sandbox verification, and release documentation. (completed 2026-09-14)
- [ ] **Phase 6: Improve Architecture v1.0** - Refactor and improve the architectural structure, modularity, and decoupling across the v1.0 driver and CLI codebase.

## Phase Details

### Phase 1: Workspace Architecture, Transport Foundation & Device Probing

**Goal:** As a keyboard user or developer, I want to discover and probe my Monka 3075 Pro over USB HID, so that I can verify device identity and connection health without triggering OS security prompts.
**Mode:** mvp
**Depends on**: Nothing (first phase)
**Requirements**: DISC-01, DISC-02, DISC-03, DISC-04, DISC-05
**Success Criteria** (what must be TRUE):

  1. User can run `monkey info` to view connected keyboard hardware identifiers (VID 0x05AC, PID 0x024F, Product Name "RKGK890", serial, and identified HID interfaces A and B) in formatted terminal text and `--json` format.
  2. User can run `monkey probe` to inspect the keyboard's device capability tuple (model, hardware revision, firmware version, and transport state) via non-destructive read-only queries.
  3. System provides a deterministic `MockTransport` simulating Monka 3075 Pro hardware responses, allowing CI and automated unit/integration tests to execute headlessly without physical hardware.
  4. System detects active transport connection mode (wired USB vs 2.4GHz wireless dongle) via heartbeat ping, reporting sleep and awake states accurately.

**Plans:** 3/3 plans complete

Plans:

- [x] 01-01-PLAN.md — Cargo virtual workspace setup, Transport trait, TransportError, and MockTransport
- [x] 01-02-PLAN.md — Dual-interface HID discovery, HidTransport with macos-shared-device, and heartbeat connection detection
- [x] 01-03-PLAN.md — monkey-cli integration with info and probe commands, JSON formatting, and automated headless integration tests

### Phase 2: Protocol Codecs, Transaction Safety Rails & Benchmark Harness

**Goal:** As a developer, I want to execute commands through a verified zero-copy protocol codec with strict safety rails and benchmarking, so that I can prevent hardware bricking and measure USB communication performance.
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: PROT-01, PROT-02, PROT-03, PROT-04, PROT-05, DIAG-01
**Success Criteria** (what must be TRUE):

  1. System enforces default-deny opcode whitelist via `SafetyGate`, allowing only capture-verified packets (`04 18`, `04 13`, `04 20`, `04 02`, `04 F0`, `04 F5`), while isolating dangerous ISP bootloader PIDs (`0x7140`) at the enumeration layer.
  2. In-flight transactions aborted by errors, panics, or Ctrl+C signals reliably emit the `04 F0` session cleanup frame via RAII `TransactionGuard`, preventing keyboard state lockup.
  3. User can execute `monkey bench` to measure chunk ACK latency, throughput, and feature report round-trip timing with machine-readable JSON output.
  4. Serialized `CommandQueue` paces packet delivery with enforced inter-packet delays (2ms–10ms), preventing USB controller lockups during back-to-back command bursts.

**Plans:** 3/3 plans complete

Plans:

- [x] 02-01-PLAN.md — Protocol codecs, CRC16 checksum engine, and zero-copy chunk framing
- [x] 02-02-PLAN.md — Transaction safety rails, opcode whitelist, flash debouncing, and worker channel
- [x] 02-03-PLAN.md — Synthetic benchmark harness, latency/throughput reports, and CLI bench command

### Phase 3: High-Performance LCD Rendering & Streaming Engine

**Goal:** As a keyboard user, I want to render static images and stream animations to the 128x128 LCD screen, so that I can personalize my keyboard display with smooth and paced visuals.
**Mode:** mvp
**Depends on**: Phase 2
**Requirements**: LCD-01, LCD-02, LCD-03, LCD-04, LCD-05
**Success Criteria** (what must be TRUE):

  1. User can render an external PNG/JPEG/BMP image onto the 128x128 LCD screen within <100ms total latency (<15ms host + <75ms transport) using `monkey lcd image <path>`.
  2. User can stream animated GIF files or image sequence directories onto the LCD screen at a steady 10–15 FPS with smooth visual playback and immediate graceful exit on Ctrl+C using `monkey lcd anim <path>` (APNG deferred to v2).
  3. User can display RGB color bars and geometry alignment patterns via `monkey lcd test-pattern` to visually verify display endianness and color channel mapping.
  4. Images converted by the pipeline display smooth color gradients without severe 16-bit banding due to Floyd-Steinberg error diffusion dithering.

**Plans**: TBD

Plans:

- [x] 03-01: TBD

### Phase 4: Ambient RGB Engine, Matrix Mapping & Two-Tier State Persistence

**Goal:** As a keyboard user, I want to configure ambient lighting and save profiles with two-tier flash wear protection, so that I can customize RGB effects without wearing out onboard SPI flash memory.
**Mode:** mvp
**Depends on**: Phase 3
**Requirements**: RGB-01, RGB-02, RGB-03, RGB-04
**Success Criteria** (what must be TRUE):

  1. User can change ambient lighting mode, speed, brightness, and static RGB color using `monkey rgb set`.
  2. Rapid interactive lighting adjustments update volatile RAM at up to 30Hz without triggering SPI NOR flash wear, debouncing flash commits to a single write after 500ms of inactivity.
  3. Flash commit operations are automatically blocked when the keyboard reports battery level below 20% on wireless connections, displaying a clear safety warning.
  4. User can back up the active lighting configuration with `monkey rgb save` and restore it with `monkey rgb restore`, verified via `04 F5` readback.

**Plans**: 3/3 plans completed

Plans:

- [x] 04-01-PLAN.md: RGB lighting mode, color primitives, packet codecs, and 81-key matrix definition
- [x] 04-02-PLAN.md: Two-tier RAM preview and flash commit engine with debouncing, battery safety, and readback
- [x] 04-03-PLAN.md: CLI `monkey rgb` subcommand tree with profile save/restore, status inspection, and human/JSON formatting

### Phase 5: Production Hardening, Packaging & Release Readiness

**Goal:** As a user or packager, I want to run pre-flight diagnostics and use shell completions with predictable CLI ergonomics, so that I can troubleshoot USB permissions and reliably distribute the tool.
**Mode:** mvp
**Depends on**: Phase 4
**Requirements**: DIAG-02
**Success Criteria** (what must be TRUE):

  1. User can run `monkey doctor` to diagnose macOS USB permissions, sandboxing entitlements, and transport health, receiving actionable remediation guidance if permissions are missing.
  2. User can generate and use tab-completion scripts for bash, zsh, and fish shells via `monkey completions <shell>`.
  3. CLI commands return standardized POSIX exit codes and actionable, user-friendly error diagnostics across all failure modes.
  4. Workspace passes automated license audits and security vulnerability scans (`cargo-deny`) with zero warnings or errors.

**Plans**: 3/3 plans completed

Plans:

- [x] 05-01-PLAN.md: System diagnostics, environment inspection & remediation engine (monkey doctor)
- [x] 05-02-PLAN.md: Shell completion generation (monkey completions <shell>)
- [x] 05-03-PLAN.md: POSIX exit codes, error categorization & cargo-deny compliance

### Phase 6: Improve Architecture v1.0

**Goal:** As a developer, I want to improve the architecture of MonKey v1.0, so that the codebase has cleaner abstractions, better separation of concerns, and stronger maintainability.
**Depends on**: Phase 5
**Requirements**: ARCH-01
**Success Criteria** (what must be TRUE):

  1. Architectural bottlenecks and coupling between CLI, core driver, transport, and protocol layers are identified and decoupled.
  2. Module boundaries, error handling, and concurrency guarantees are aligned with the target architecture conventions.
  3. All existing unit and integration tests pass with zero regressions.

**Plans**: 0/0 plans completed

Plans:
- [ ] TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Workspace Architecture, Transport Foundation & Device Probing | 3/3 | Complete    | 2026-09-13 |
| 2. Protocol Codecs, Transaction Safety Rails & Benchmark Harness | 3/3 | Complete    | 2026-09-13 |
| 3. High-Performance LCD Rendering & Streaming Engine | 3/3 | Complete    | 2026-09-13 |
| 4. Ambient RGB Engine, Matrix Mapping & Two-Tier State Persistence | 3/3 | Complete    | 2026-09-14 |
| 5. Production Hardening, Packaging & Release Readiness | 3/3 | Complete    | 2026-09-14 |
| 6. Improve Architecture v1.0 | 0/0 | Not Started | - |
