---
id: 260913-m9v
title: Fix Phase 1 Code Review Findings
date: 2026-09-13
type: quick
status: completed
completed_at: 2026-09-13T09:16:00Z
---

# Quick Summary: Fix Phase 1 Code Review Findings

All 4 code review findings in `crates/monkey-core` and `crates/monkey-cli` have been resolved, verified, and confirmed against unit tests, integration tests, and the review reproduction harness.

## Key Changes
1. **Device Grouping Logic (`crates/monkey-core/src/device.rs`)**:
   - Replaced naive greedy pairing with closest-matching logic for `DevSrvsID` proximity on macOS.
   - Prevents cross-device interface pairing when multiple devices are connected.
   - Added unit test `test_group_devices_multiple_devices_closest_matching`.

2. **HID Transport State Query (`crates/monkey-core/src/transport/hid.rs`)**:
   - `evaluate_state_query` now requires read count `n > 0` before returning `WirelessAwake`.
   - `Ok(0)` reads are treated as `WirelessSleeping` (or `TransportError::Timeout` for wired).
   - Added tests for 0-byte reads and sleeping transitions.

3. **CLI Probe & Fallbacks (`crates/monkey-cli/src/commands/probe.rs`)**:
   - Replaced hardcoded `rev1.0` / `v1.0.0` with actual packet parsing and safe `"unknown"` fallback.
   - Fallback `build_descriptor_only_probe_output` checks `set.is_wireless()` and sets `WirelessSleeping` instead of assuming `WiredUsb`.

4. **Protocol Types (`crates/monkey-core/src/protocol/types.rs`)**:
   - Added `parse_from_slice` and `as_bytes` helpers to validate zero-copy packet extraction.

## Verification
- `cargo test --workspace`: 47 tests passed cleanly.
- `/private/tmp/monkey_phase1_review.rs`:
  - `paired "DevSrvsID:1000" / "DevSrvsID:1007"`
  - `paired "DevSrvsID:1010" / "DevSrvsID:1017"`
  - `arbitrary response -> revision=unknown, firmware=unknown`
  - `no transport -> state=WirelessSleeping`
  - `zero-byte success -> Ok(WirelessSleeping)`
