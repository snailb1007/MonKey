---
phase: 07-retro-pixel-desktop
plan: '01'
subsystem: desktop-native-adapter
tags: [tauri, ipc, native-adapter, retro-pixel, rgb, lcd, mock-transport]

requires:
  - phase: 06-improve-architecture-v1-0
    provides: Pure Transport, MonkaDevice, SafeTransport, InterfacePolicy
provides:
  - Tauri v2 native desktop command adapter (desktop_request)
  - Full IPC v1 typed request/response contract with fixtures
  - Serialized hardware channel worker off UI thread
  - Strict capability, partial-interface, battery gating, and RAM preview coalescing
  - Scoped asset parsing, bounded conversion (RGB565), and sequence streaming
affects: [apps/desktop/src-tauri, apps/desktop/contracts, frontend-integration]

actuals:
  tasks: 5
  commits: 1

tech-stack:
  added: [tauri 2.12, rfd 0.15, base64 0.22]
  patterns: [single-serialized-hardware-worker, opaque-session-identifiers, bounded-asset-pipeline, typed-ipc-envelope]

key-files:
  created:
    - apps/desktop/src-tauri/Cargo.toml
    - apps/desktop/src-tauri/tauri.conf.json
    - apps/desktop/src-tauri/src/lib.rs
    - apps/desktop/src-tauri/src/main.rs
    - apps/desktop/src-tauri/src/adapter.rs
    - apps/desktop/src-tauri/src/worker.rs
    - apps/desktop/src-tauri/src/asset.rs
    - apps/desktop/src-tauri/src/dto.rs
    - apps/desktop/src-tauri/src/error.rs
    - apps/desktop/src-tauri/src/fixtures.rs
    - apps/desktop/src-tauri/src/profile.rs
    - apps/desktop/contracts/ipc-v1.json
  modified: []

key-decisions:
  - "Single invoke command desktop_request({request}) with strict camelCase DTO envelopes and snake_case action tags."
  - "Opaque device, asset, and job IDs preventing path or raw opcode leakage to frontend."
  - "Serialized background worker thread owning MonkaDevice and SafeTransport, enforcing 10-25ms chunk pacing and cooperative cancellation."
  - "Strict hardware safety rails: unsupported actions (remap, macro, firmware) rejected natively; persistent writes blocked on low/unknown wireless battery; preview never commits to flash."

requirements-completed: [UI2-01, UI2-02, UI2-03]

coverage:
  - id: V1
    description: "Desktop native IPC contract test suite"
    requirement: "UI2-01"
    verification:
      - kind: automated
        ref: "apps/desktop/src-tauri/src/contract_tests.rs"
        status: pass
    human_judgment: false
  - id: V2
    description: "Hardware worker lifecycle, isolation, and safety rails"
    requirement: "UI2-02"
    verification:
      - kind: automated
        ref: "apps/desktop/src-tauri/src/worker_tests.rs"
        status: pass
    human_judgment: false
  - id: V3
    description: "Asset ingestion, bounded decode, and streaming jobs"
    requirement: "UI2-03"
    verification:
      - kind: automated
        ref: "apps/desktop/src-tauri/src/asset_tests.rs"
        status: pass
    human_judgment: false

duration: 25 min
completed: 2026-10-15
status: complete
---

# 07-01 Summary: Safe Tauri Native Adapter + IPC Contract Tests

## Overview
Implemented the real Tauri v2 native adapter and safety tests under `apps/desktop/src-tauri` with contract fixture definitions in `apps/desktop/contracts/ipc-v1.json`. All IPC operations are routed through a single typed invoke handler (`desktop_request`) backed by a dedicated background hardware worker thread that owns `MonkaDevice` and `SafeTransport`.

## Key Verification Results
- `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check`: PASSED
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --lib`: PASSED (37 tests)
- `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --lib -- -D warnings`: PASSED
- `cargo test --workspace`: PASSED (132 core/cli tests + 37 desktop tests = 169 total)
- `cargo clippy --all-targets -- -D warnings`: PASSED

## Disjoint Ownership & Boundaries
All modified/created files are strictly contained within `apps/desktop/src-tauri/**`, `apps/desktop/contracts/ipc-v1.json`, and `.planning/phases/07-retro-pixel-desktop/07-01-SUMMARY.md`. Root manifests and core/CLI crates remain completely untouched.
