---
phase: 01-foundation-and-hardware-probing
reviewed: 2026-09-13T14:20:00Z
depth: standard
files_reviewed: 16
files_reviewed_list:
  - Cargo.toml
  - crates/monkey-core/Cargo.toml
  - crates/monkey-core/src/lib.rs
  - crates/monkey-core/src/error.rs
  - crates/monkey-core/src/device.rs
  - crates/monkey-core/src/transport/mod.rs
  - crates/monkey-core/src/transport/mock.rs
  - crates/monkey-core/src/transport/hid.rs
  - crates/monkey-core/src/protocol/mod.rs
  - crates/monkey-core/src/protocol/types.rs
  - crates/monkey-cli/Cargo.toml
  - crates/monkey-cli/src/main.rs
  - crates/monkey-cli/src/output.rs
  - crates/monkey-cli/src/commands/mod.rs
  - crates/monkey-cli/src/commands/info.rs
  - crates/monkey-cli/src/commands/probe.rs
findings:
  critical: 2
  warning: 4
  info: 3
  total: 9
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-09-13T14:20:00Z  
**Depth:** standard  
**Files Reviewed:** 16  
**Status:** issues_found  

## Summary

The Phase 01 implementation establishes a clean virtual Cargo workspace (`monkey-core` and `monkey-cli`) with zero-copy packet structures, deterministic in-memory `MockTransport`, synchronous `Transport` trait, non-exclusive macOS device access via `macos-shared-device`, and CLI subcommands (`info` and `probe`). All 39 automated unit and integration tests compile and pass cleanly without compiler warnings.

However, an adversarial audit against hardware constraints, `hidapi` driver internals, and protocol requirements surfaced **2 Critical issues** and **4 Warnings**:
1. **Critical:** Feature report query buffers are sized at 64 bytes instead of 65 bytes (`1 + 64`), causing an `IOHIDDeviceGetReport` buffer overrun error on macOS for unnumbered Report ID 0.
2. **Critical:** Hardware connection state detection fails on Bluetooth keyboards (e.g. `BT5.1-KB`), erroneously classifying them as `WiredUsb` due to brittle substring checks while ignoring `hidapi`'s native `bus_type`. Downstream bulk LCD streaming could attempt to transmit over Bluetooth.
3. **Warnings:** Flawed multi-device isolation without serial numbers, mock-vs-hardware buffer layout divergence in `get_feature_report`, hardcoded capability tuples that contradict live interface availability, and silent error swallowing in `monkey probe`.

---

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: 64-byte feature report probe buffer causes `IOHIDDeviceGetReport` buffer overrun on macOS

**File:** [`crates/monkey-core/src/transport/hid.rs:105-106`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/transport/hid.rs#L105-L106) and [`crates/monkey-cli/src/commands/probe.rs:33-34`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/probe.rs#L33-L34)  
**Issue:**  
In `hidapi` on macOS (`mac/hid.c`), when invoking `IOHIDDeviceGetReport` with unnumbered Report ID 0, the driver logic reserves byte 0 for the report ID and slices the buffer:
```c
if (report_id == 0x0) {
    report = data + 1;
    report_length = length - 1;
}
res = IOHIDDeviceGetReport(dev->device_handle, type, report_id, report, &report_length);
```
Both `HidTransport::detect_connection_state` and `probe_device_with_transport` allocate a 64-byte buffer:
```rust
let mut probe_buf = [0u8; 64];
let query_res = transport.get_feature_report(0, &mut probe_buf);
```
Because `length` is 64, `report_length` passed to the kernel is only 63 bytes. When the device returns its standard 64-byte feature report, `IOHIDDeviceGetReport` fails with `kIOReturnOverrun` (`0xE00002CD`, buffer overrun). In `probe.rs`, this failure is masked by `.unwrap_or(...)`, but in `detect_connection_state`, it causes an unexpected `TransportError::HidError`. The buffer must be at least `65` bytes (`1 + FEATURE_REPORT_SIZE`).

**Fix:**  
Increase the probe buffer to 65 bytes in both locations:
```rust
// crates/monkey-core/src/transport/hid.rs
pub fn detect_connection_state(&mut self) -> Result<ConnectionState, TransportError> {
    let mut probe_buf = [0u8; 65];
    let query_res = self.get_feature_report(0, &mut probe_buf);
    Self::evaluate_state_query(self.is_wireless, query_res)
}

// crates/monkey-cli/src/commands/probe.rs
pub fn probe_device_with_transport<T: Transport>(
    transport: &mut T,
    device_set: Option<&MonkaDeviceSet>,
    is_wireless: bool,
) -> ProbeOutput {
    let mut probe_buf = [0u8; 65];
    let query_res = transport.get_feature_report(0, &mut probe_buf);
    ...
}
```

---

### CR-02: Bluetooth-connected keyboards erroneously classified as `WiredUsb`

**File:** [`crates/monkey-core/src/transport/hid.rs:52-55`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/transport/hid.rs#L52-L55) and [`crates/monkey-core/src/transport/hid.rs:85-100`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/transport/hid.rs#L85-L100)  
**Issue:**  
`is_wireless_product` checks only for literal substrings:
```rust
pub fn is_wireless_product(product: &str) -> bool {
    let p = product.to_lowercase();
    p.contains("wireless") || p.contains("2.4g") || p.contains("receiver") || p.contains("dongle")
}
```
When the Monka 3075 Pro is connected via Bluetooth on macOS, its product string is `"BT5.1-KB"`. Because this does not match any substring, `is_wireless` is evaluated as `false`. Consequently, `evaluate_state_query` marks the device as `ConnectionState::WiredUsb`.

This violates requirement DISC-05 and creates a severe hazard for Phase 3: the system will believe the device is connected over high-bandwidth wired USB and may attempt 32KB bulk LCD frame transfers over BLE, which is explicitly documented as incapable of supporting bulk frames. Furthermore, in `evaluate_state_query`, if a wired device query times out, it returns `WirelessSleeping`, erroneously claiming a wired device is wireless.

**Fix:**  
1. Include `"bt"` and `"bluetooth"` in `is_wireless_product` (or capture `info.bus_type()` from `hidapi::DeviceInfo`).
2. Do not return `WirelessSleeping` for non-wireless devices on timeout:
```rust
pub fn is_wireless_product(product: &str) -> bool {
    let p = product.to_lowercase();
    p.contains("wireless")
        || p.contains("2.4g")
        || p.contains("receiver")
        || p.contains("dongle")
        || p.contains("bluetooth")
        || p.starts_with("bt")
}

pub fn evaluate_state_query(
    is_wireless: bool,
    query_result: Result<usize, TransportError>,
) -> Result<ConnectionState, TransportError> {
    match query_result {
        Ok(_) => {
            if is_wireless {
                Ok(ConnectionState::WirelessAwake)
            } else {
                Ok(ConnectionState::WiredUsb)
            }
        }
        Err(TransportError::Timeout) => {
            if is_wireless {
                Ok(ConnectionState::WirelessSleeping)
            } else {
                Err(TransportError::Timeout)
            }
        }
        Err(e) => Err(e),
    }
}
```

---

## Warnings

### WR-01: `physical_device_key` fails on macOS, risking cross-pairing multiple keyboards

**File:** [`crates/monkey-core/src/device.rs:119-135`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/device.rs#L119-L135) and [`crates/monkey-core/src/device.rs:174-187`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/device.rs#L174-L187)  
**Issue:**  
Monka 3075 Pro keyboards often report an empty serial number string (`""`) on macOS. In that case, `physical_device_key` attempts to parse parent paths looking for `:1.` or `/IOUSBHostInterface@`. However, `hidapi 2.6.7` on macOS formats device paths strictly as `DevSrvsID:<IORegistryEntryID>` (e.g. `DevSrvsID:4295837800`). Consequently, neither pattern ever matches, and `physical_device_key` returns `None`.

When `physical_device_key` returns `None`, `group_monka_devices` falls back to pairing candidate interfaces with any existing set that "needs this role":
```rust
if target_set_idx.is_none() {
    for (i, set) in sets.iter().enumerate() {
        let needs_role = match role {
            InterfaceRole::InterfaceA => set.interface_a.is_none(),
            InterfaceRole::InterfaceB => set.interface_b.is_none(),
        };
        if needs_role {
            target_set_idx = Some(i);
            break;
        }
    }
}
```
If two Monka keyboards without serial numbers are connected simultaneously, Keyboard 1's Interface A and Keyboard 2's Interface B can be merged into a single `MonkaDeviceSet`, resulting in bulk transfers being sent to one physical keyboard while feature reports are sent to the other.

**Fix:**  
Do not indiscriminately cross-pair interfaces when physical parent keys are unavailable. When serial numbers or physical paths are ambiguous, create separate sets or check matching USB location/address attributes if available.

---

### WR-02: `MockTransport::get_feature_report` buffer layout diverges from `HidTransport`

**File:** [`crates/monkey-core/src/transport/mock.rs:137-138`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/transport/mock.rs#L137-L138)  
**Issue:**  
In `HidTransport::get_feature_report`, `buf[0]` is seeded with `report_id` by `prepare_feature_buffer`, and the operating system places the payload in `buf[1..]` (for unnumbered report ID 0), returning the total byte length including `buf[0]`.

In `MockTransport::get_feature_report`, the mock directly overwrites `buf` starting at index 0:
```rust
buf[..canned.len()].copy_from_slice(canned);
Ok(canned.len())
```
This causes test doubles to behave differently from physical hardware: code developed against `MockTransport` will expect payload data at `buf[0]`, whereas against `HidTransport` it appears at `buf[1]`.

**Fix:**  
Align `MockTransport::get_feature_report` behavior with `HidTransport`: seed `buf[0] = report_id` and copy payload to `buf[1..]` when simulating unnumbered or framed feature reports, or explicitly document and standardize whether the `buf` parameter to `get_feature_report` includes or excludes the report ID framing byte.

---

### WR-03: Hardcoded capability matrix contradicts interface detection

**File:** [`crates/monkey-cli/src/commands/probe.rs:53-66`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/probe.rs#L53-L66) and [`crates/monkey-cli/src/commands/probe.rs:71-89`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/probe.rs#L71-L89)  
**Issue:**  
In `probe.rs`, the capability list is statically defined:
```rust
capabilities: vec![
    "LCD display 128x128 RGB565".to_string(),
    "81-key RGB matrix".to_string(),
    "dual composite interface".to_string(),
]
```
When running on real hardware connected over Bluetooth where Interface A is absent, `monkey probe` prints:
```
Hardware Capabilities:
  [✓] LCD display 128x128 RGB565
  [✓] 81-key RGB matrix
  [✓] dual composite interface

Interface Status:
  Interface A (Bulk OUT 0xFF68:0x0061): Not Present
  Interface B (Control  0x000C:0x0001): Detected
```
Displaying green checkmarks for "LCD display" and "dual composite interface" directly above "Interface A: Not Present" is contradictory and misleads users. Furthermore, `build_descriptor_only_probe_output` defaults `(interface_a, interface_b)` to `(true, true)` when `device_set` is `None`.

**Fix:**  
Make the capability list dynamic based on detected interfaces:
```rust
let mut capabilities = vec!["81-key RGB matrix".to_string()];
if interface_a {
    capabilities.push("LCD display 128x128 RGB565".to_string());
}
if interface_a && interface_b {
    capabilities.push("dual composite interface".to_string());
}
```

---

### WR-04: Silent error swallowing in `probe_device_with_transport`

**File:** [`crates/monkey-cli/src/commands/probe.rs:36-42`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/probe.rs#L36-L42)  
**Issue:**  
In `probe_device_with_transport`:
```rust
let state = HidTransport::evaluate_state_query(is_wireless, query_res).unwrap_or(
    if is_wireless {
        ConnectionState::WirelessAwake
    } else {
        ConnectionState::WiredUsb
    },
);
```
If `query_res` fails due to an I/O error, device disconnection, or buffer overrun, `evaluate_state_query` returns `Err(e)`. The `.unwrap_or(...)` call silently discards this error and fabricates a healthy `WiredUsb` state. This prevents diagnostic detection of broken endpoints.

**Fix:**  
Log a warning on query errors and reflect the failure in `ProbeOutput` (e.g. `transport_state: format!("Unknown/Error: {e}")`).

---

## Info

### IN-01: Empty serial number string printed as blank line in `monkey info`

**File:** [`crates/monkey-cli/src/commands/info.rs:89`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/info.rs#L89)  
**Issue:**  
```rust
serial_number: set.serial_number.clone().or_else(|| Some("N/A".to_string())),
```
When `set.serial_number` is `Some("")` (as observed on macOS), `.or_else()` is not invoked. `format_info_human` prints `Serial Number:          ` with trailing whitespace instead of `N/A`.  
**Fix:** Use `.filter(|s| !s.trim().is_empty()).or_else(|| Some("N/A".to_string()))`.

---

### IN-02: Multi-device connections silently ignored

**File:** [`crates/monkey-cli/src/commands/info.rs:165`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/info.rs#L165) and [`crates/monkey-cli/src/commands/probe.rs:149`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-cli/src/commands/probe.rs#L149)  
**Issue:**  
Both `run_info_with_writer` and `run_probe_with_writer` unconditionally index `&sets[0]`. If multiple keyboards are attached, `sets[1..]` are silently omitted without notice.  
**Fix:** If `sets.len() > 1`, log an informational message or iterate over all sets.

---

### IN-03: Heap allocation on bulk write in `HidTransport::write_bulk`

**File:** [`crates/monkey-core/src/transport/hid.rs:60-65`](file:///Volumes/D/personal_project/MonkaKeyboard/crates/monkey-core/src/transport/hid.rs#L60-L65)  
**Issue:**  
`frame_bulk_buffer` allocates a new `Vec<u8>` for every write call (`Vec::with_capacity(1 + data.len())`). While negligible for Phase 1 smoke probing, high-frequency 15 FPS LCD animation streaming (Phase 3, 120 packets/sec) will create unnecessary heap churn.  
**Fix:** Maintain a reusable pre-allocated buffer `[u8; 4097]` inside `HidTransport` or accept pre-framed buffers.

---

_Reviewed: 2026-09-13T14:20:00Z_  
_Reviewer: adversarial code reviewer (gsd-code-reviewer)_  
_Depth: standard_
