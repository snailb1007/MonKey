---
phase: 03
slug: high-performance-lcd-rendering-streaming-engine
status: verified
threats_open: 0
asvs_level: 1
created: 2026-09-13
---

# Phase 03 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail for High-Performance LCD Rendering & Streaming Engine.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| File System → Ingestion Pipeline | Host file system to `image` & `GifDecoder` decoding routines | Untrusted static images and animated GIF files |
| User CLI Arguments → Pacing & Frame Regulator | Command-line flags parsed via `clap` into driver timing knobs | Frame rate (FPS) and inter-chunk delay values |
| Memory Frame Buffer → HID Hardware Transport | In-memory 32 KiB framebuffer sliced into 4096-byte chunks | USB Interface A unnumbered bulk OUT packets sent to physical hardware |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-03-01 | Denial of Service | `crates/monkey-core/src/lcd/image_loader.rs` | medium | mitigate | Dynamic memory allocation during malicious image / decompression bomb decode bounded by center-crop and resize ops; zero-division safe fallback (`side == 0`) returns blank 128x128 buffer | closed |
| T-03-02 | Elevation of Privilege / Device Damage | `crates/monkey-cli/src/commands/lcd.rs` | high | mitigate | Hardware write safety invariant enforced via mandatory `--allow-hardware-writes` consent flag; mock mode default safely intercepts writes without hardware transmission | closed |
| T-03-03 | Denial of Service | `crates/monkey-core/src/lcd/streamer.rs` | medium | mitigate | Driver pacing validation bounds target FPS strictly within safe hardware range (10–15 FPS) and inter-chunk delay (0–8 ms) to prevent MCU FIFO overflow and keyboard lockup | closed |
| T-03-04 | Tampering | `crates/monkey-core/src/lcd/chunker.rs` | high | mitigate | FrameChunker strictly validates buffer size against `LCD_FRAME_BYTES` (32,768 bytes), enforcing compile-time slice bounds to prevent buffer overruns or partial chunk corruption | closed |

*Status: open · closed · open — below high threshold (non-blocking)*  
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*  
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|

*No accepted risks.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-13 | 4 | 4 | 0 | gsd-security-auditor |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-13
