---
phase: "06"
slug: "improve-architecture-v1-0"
status: draft
nyquist_compliant: true
wave_0_complete: true
created: "2026-09-16"
---

# Phase 06 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Cargo test (Rust built-in test runner) |
| **Config file** | `Cargo.toml` (workspace root) |
| **Quick run command** | `cargo test -p monkey-core` |
| **Full suite command** | `cargo test --all-targets` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p monkey-core` (or relevant crate)
- **After every plan wave:** Run `cargo test --all-targets && cargo clippy --all-targets -- -D warnings`
- **Before `/gsd-verify-work`:** Full suite must be green (all 122+ unit/integration tests passing)
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 06-01-01 | 01 | 1 | ARCH-01 | T-06-01 | Pure `Transport` trait without `&SafetyRails` | unit | `cargo test -p monkey-core transport` | ✅ | ⬜ pending |
| 06-01-02 | 01 | 1 | ARCH-01 | T-06-01 | `SafeTransport<'a>` guard façade write check | unit | `cargo test -p monkey-core transaction_safety` | ✅ | ⬜ pending |
| 06-02-01 | 02 | 2 | ARCH-01 | T-06-02 | `InterfacePolicy` and granular `OpenError` | unit | `cargo test -p monkey-core device` | ✅ | ⬜ pending |
| 06-02-02 | 02 | 2 | ARCH-01 | T-06-02 | `MonkaDevice` deep module operations & mock constructor | unit | `cargo test -p monkey-core device` | ✅ | ⬜ pending |
| 06-02-03 | 02 | 2 | ARCH-01 | T-06-03 | `MonkaDevice::diagnose()` non-early-returning diagnostics | unit | `cargo test -p monkey-core doctor` | ✅ | ⬜ pending |
| 06-03-01 | 03 | 3 | ARCH-01 | T-06-04 | CLI migration to `MonkaDevice` across all commands | integration | `cargo test -p monkey-cli` | ✅ | ⬜ pending |
| 06-03-02 | 03 | 3 | ARCH-01 | T-06-04 | Remove `MONKEY_SIMULATE_EMPTY` & unify ad-hoc test seams | integration | `cargo test -p monkey-cli --test cli_probe_test` | ✅ | ⬜ pending |
| 06-04-01 | 04 | 4 | ARCH-01 | T-06-05 | Full workspace regression & clippy verification | integration | `cargo test --all-targets && cargo clippy --all-targets -- -D warnings` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing test infrastructure covers all phase requirements (122 existing workspace tests are currently green).

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 10s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** verified 2026-09-16
