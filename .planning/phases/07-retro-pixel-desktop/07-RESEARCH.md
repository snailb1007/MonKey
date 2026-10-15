# Phase 07 — cited research handoff

Source: completed Scout t_be53c466, retrieved through parent handoff on t_033482d4. This document summarizes the cited answer, not independent hardware verification.

- Main and assigned worktree were at 7df22cb during Scout research; neither contained a frontend/desktop scaffold.
- OEM UI is Windows MFC DeviceDriver.exe/mui.dll plus XML, PNG skin assets and .lan string tables. Reuse workflow knowledge, not proprietary assets/binaries.
- Verified v1 core candidate workflows: device discovery/probe; LCD image/GIF/sequence/pattern; RGB/status/profile workflows; doctor diagnostics. Existing CLI remains reference for exact semantics.
- Research citations: crates/monkey-core/src/device.rs:38–120; src/lcd/mod.rs:1–120; src/rgb/mod.rs:1–100; src/doctor.rs:1–120. Device.rs and lib.rs were freshly inspected by orchestrator; exports include MonkaDevice, InterfacePolicy, SafeTransport, LCD conversion/streaming, RgbManager and doctor APIs.
- OEM references: vendor_driver/language/1033.lan:64–125 (remap/Fn), 176–208 (macro), 228–234 and 600–606 (firmware), 246–263 (audio). Generic mouse skin assets do not constitute keyboard requirements.
- Unverified remap/macros must remain gated. Firmware/ISP, erase/reset and audio visualization cannot become executable UI actions. No new whitelist entries.
- Scout found no docs/solutions/ or docs/decisions/ on disk. Compound pre-check therefore has no prior solution pattern to embed. No stale solution was identified; no Refresh card is needed.
- New scoped phase 07 follows completed phases 01–06. ROADMAP phase 6 progress row disagrees with its completed detail and STATE; preserve historical bookkeeping and note inconsistency rather than claiming a new verification.
- Native Tauri v2 adapter over synchronous core; local typography; mocked browser UI tests and native contract tests; actual hardware acceptance remains human UAT.

## Cautions / unresolved evidence
Fresh GitNexus query for MonkaDevice/LCD/RGB/doctor failed: database storage version 43 versus engine 42. No call-graph coverage or low-risk claim is made. Builders must obtain usable graph impact analysis before touching existing symbols; no existing core/CLI modifications are planned. Do not rebuild the shared graph from this planning run.
Scout's prose mentions differing RGB persistence opcode descriptions; STATE/REQUIREMENTS distinguish commit and readback differently. UI spec intentionally defines no raw opcodes: use current core APIs and tests, not research prose or OEM guesses.

## Verification proposals made executable in plans
Native: root cargo test/clippy plus standalone src-tauri library tests/clippy/format. Frontend: locked pnpm install, typecheck, ESLint, Vitest, Vite production build, Playwright with axe/keyboard and deterministic mock IPC. Final gate: Tauri production build without signing/bundling. No tests run as part of planning.
