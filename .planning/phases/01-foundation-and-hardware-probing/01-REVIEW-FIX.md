---
phase: 01-foundation-and-hardware-probing
fixed_at: 2026-09-13T14:28:00Z
review_path: .planning/phases/01-foundation-and-hardware-probing/01-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 8
skipped: 0
status: all_fixed
---

# Phase 01: Code Review Fix Report

**Fixed at:** 2026-09-13T14:28:00Z  
**Source review:** `.planning/phases/01-foundation-and-hardware-probing/01-REVIEW.md`  
**Iteration:** 1  

**Summary:**
- Findings in scope: 8
- Fixed: 8
- Skipped: 0

---

## Fixed Issues

### CR-01: 64-byte feature report probe buffer causes `IOHIDDeviceGetReport` buffer overrun on macOS

- **Files modified:** [`crates/monkey-core/src/transport/hid.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/transport/hid.rs), [`crates/monkey-cli/src/commands/probe.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/probe.rs)
- **Commit:** `2c70294`
- **Applied fix:** Increased feature report probe buffer size from 64 to 65 bytes (`1 + 64`), reserving byte 0 for the unnumbered Report ID prefix per macOS `IOHIDDeviceGetReport` driver requirements to prevent kernel buffer overrun errors (`kIOReturnOverrun`).

### CR-02: Bluetooth-connected keyboards erroneously classified as `WiredUsb`

- **Files modified:** [`crates/monkey-core/src/transport/hid.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/transport/hid.rs), [`crates/monkey-core/tests/hid_transport_test.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/tests/hid_transport_test.rs)
- **Commit:** `48ceecf`
- **Applied fix:** Extended `HidTransport::new` to check `device.get_device_info().bus_type()` for `BusType::Bluetooth` and broadened `is_wireless_product` heuristics to match `bluetooth`, `bt`, and `-bt` prefixes. Updated `evaluate_state_query` to propagate `Err(TransportError::Timeout)` on wired connections rather than misclassifying wired timeouts as `WirelessSleeping`.

### WR-01: `physical_device_key` fails on macOS, risking cross-pairing multiple keyboards

- **Files modified:** [`crates/monkey-core/src/device.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/device.rs), [`crates/monkey-core/tests/device_discovery_test.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/tests/device_discovery_test.rs)
- **Commit:** `ad032d8`
- **Applied fix:** Implemented `parse_devsrvs_id` to parse macOS `DevSrvsID:<u64>` registry entry IDs and verify proximity (`delta <= 32`) between child interfaces on the same physical composite USB device. Added ambiguity detection so that when multiple unkeyed keyboards with non-proximity paths are discovered, they are isolated into separate sets rather than indiscriminately cross-paired.

### WR-02: `MockTransport::get_feature_report` buffer layout diverges from `HidTransport`

- **Files modified:** [`crates/monkey-core/src/transport/mock.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/transport/mock.rs), [`crates/monkey-core/src/transport/mod.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/transport/mod.rs), [`crates/monkey-core/tests/mock_transport_test.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/tests/mock_transport_test.rs)
- **Commit:** `367c900`
- **Applied fix:** Explicitly documented the `Transport::get_feature_report` buffer framing contract (`buf` includes Report ID byte at index 0). Aligned `MockTransport::get_feature_report` to seed `buf[0] = report_id` and place payload at `buf[1..]` for raw/unnumbered reports, exactly mirroring hardware `HidTransport` behavior.

### WR-03: Hardcoded capability matrix contradicts interface detection

- **Files modified:** [`crates/monkey-cli/src/commands/probe.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/probe.rs), [`crates/monkey-cli/tests/cli_probe_test.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/tests/cli_probe_test.rs)
- **Commit:** `39d0212`
- **Applied fix:** Implemented `determine_capabilities` to dynamically compute capabilities from detected interfaces: "LCD display 128x128 RGB565" requires Interface A, and "dual composite interface" requires both Interface A and Interface B. Updated `build_descriptor_only_probe_output` to default to `(false, false)` when `device_set` is `None`.

### WR-04: Silent error swallowing in `probe_device_with_transport`

- **Files modified:** [`crates/monkey-cli/src/commands/probe.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/probe.rs), [`crates/monkey-cli/tests/cli_probe_test.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/tests/cli_probe_test.rs)
- **Commit:** `985754c`
- **Applied fix:** Removed `.unwrap_or(...)` silent swallowing in `probe_device_with_transport`. Query errors are now logged via `tracing::warn!` and propagated into `ProbeOutput.transport_state` as `"Unknown/Error: {e}"`.

### IN-01: Empty serial number string printed as blank line in `monkey info`

- **Files modified:** [`crates/monkey-cli/src/commands/info.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/info.rs), [`crates/monkey-cli/tests/cli_probe_test.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/tests/cli_probe_test.rs)
- **Commit:** `6c01244`
- **Applied fix:** Added `.filter(|s| !s.trim().is_empty())` before falling back to `"N/A"` in `build_info_output`, preventing empty string serial numbers on macOS from printing as blank trailing whitespace.

### IN-02: Multi-device connections silently ignored

- **Files modified:** [`crates/monkey-cli/src/commands/info.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/info.rs), [`crates/monkey-cli/src/commands/probe.rs`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/probe.rs)
- **Commit:** `4cd8a8f`
- **Applied fix:** Added `tracing::warn!` in `run_info_with_writer` and `run_probe_with_writer` when `sets.len() > 1` to alert the user that multiple Monka keyboards are connected and the first set is being targeted.

---

## Verification

All fixes were verified with the complete workspace test suite:
- `cargo check --workspace`: Passed cleanly (0 warnings)
- `cargo test --workspace`: 45 passed, 0 failed, 0 ignored
- `cargo clippy --workspace --all-targets -- -D warnings`: Passed cleanly (0 warnings)
- Verification environment: main checkout
