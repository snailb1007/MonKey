---
phase: 01-foundation-and-hardware-probing
plan: 02
subsystem: transport
tags: [rust, hidapi, macos-shared-device, dual-interface, device-discovery, report-id-0, connection-state]

requires:
  - phase: 01-foundation-and-hardware-probing
    plan: 01
provides:
  - Monka 3075 Pro / RKGK890 hardware discovery and identity constants (VID 0x05AC, PID 0x024F)
  - Dual HID interface disambiguation (Interface A 0xFF68:0x0061 bulk OUT vs Interface B 0x000C:0x0001 / 0xFFFF control)
  - Composite MonkaDeviceSet grouping with macOS multi-usage record deduplication
  - Path-based non-exclusive device opening via macos-shared-device (hid_darwin_set_open_exclusive(0))
  - Concrete HidTransport wrapping hidapi::HidDevice with userspace Report ID 0 0x00 prefix framing
  - Non-destructive connection state heartbeat detection (WiredUsb, WirelessAwake, WirelessSleeping)
affects:
  - 01-03-PLAN
  - 02-protocol-codecs
  - 03-lcd-rendering

actuals:
  tokens: 28000
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Dual HID interface disambiguation via (UsagePage, Usage) tuples with interface_number fallback (D-02)"
    - "Non-exclusive macOS device opening with macos-shared-device to prevent OS keyboard driver lockout (D-03)"
    - "Deduplication of duplicate usage-pair records from composite IOHIDDevice handles (RESEARCH.md)"
    - "Report ID 0 userspace prefix byte framing (0x00) for macOS unnumbered reports (D-02, D-05)"
    - "Non-destructive connection state detection via read queries with finite timeout and zero flash writes (DISC-05, D-12)"

key-files:
  created:
    - crates/monkey-core/src/device.rs
    - crates/monkey-core/src/transport/hid.rs
    - crates/monkey-core/tests/device_discovery_test.rs
    - crates/monkey-core/tests/hid_transport_test.rs
  modified:
    - crates/monkey-core/src/lib.rs
    - crates/monkey-core/src/error.rs
    - crates/monkey-core/src/transport/mod.rs

key-decisions:
  - "D-01: Pinned hardware identity constants: MONKA_VID = 0x05AC (1452), MONKA_PID = 0x024F (591), PRODUCT_IDENTIFIER = RKGK890"
  - "D-02: Disambiguated Interface A (0xFF68:0x0061, 4096B OUT) and Interface B (0x000C:0x0001 / 0xFFFF, control) with fallback to interface_number"
  - "D-03: Used open_device_path to open exact enumerated path with macos-shared-device (hid_darwin_set_open_exclusive(0))"
  - "D-05: Implemented HidTransport wrapping hidapi::HidDevice with Report ID 0 0x00 userspace buffer prefixing"
  - "DISC-05 & D-12: Evaluated connection state (WiredUsb, WirelessAwake, WirelessSleeping) using non-destructive reads without emitting flash writes or DFU opcodes"

patterns-established:
  - "Composite device set grouping: Aggregate candidate DiscoveredDevice records by serial number or physical path, merging duplicate macOS usage pairs into observed_usages"
  - "Exact path opening: Always open candidate devices via api.open_path(&device.path) to prevent hidapi from picking an arbitrary matching device"
  - "Buffer prefix framing: Always prepend Report ID (0x00 for unnumbered) in userspace and subtract 1 from raw write byte count when computing payload length"

requirements-completed:
  - DISC-01
  - DISC-05

coverage:
  - id: D1
    description: "Monka 3075 Pro / RKGK890 device identification matches VID 0x05AC and PID 0x024F"
    requirement: "DISC-01"
    verification:
      - kind: unit
        ref: "crates/monkey-core/tests/device_discovery_test.rs#test_device_info_json_constants_match"
        status: pass
    human_judgment: false
  - id: D2
    description: "Dual HID interfaces disambiguated into Interface A and Interface B with fallback"
    requirement: "DISC-01"
    verification:
      - kind: unit
        ref: "crates/monkey-core/tests/device_discovery_test.rs#test_classify_interface_a_bulk_pipe"
        status: pass
      - kind: unit
        ref: "crates/monkey-core/tests/device_discovery_test.rs#test_classify_interface_b_control_pipe_standard"
        status: pass
      - kind: unit
        ref: "crates/monkey-core/tests/device_discovery_test.rs#test_classify_interface_fallback_to_interface_number"
        status: pass
    human_judgment: false
  - id: D3
    description: "Multi-record descriptor grouping and duplicate usage-pair deduplication"
    requirement: "DISC-01"
    verification:
      - kind: unit
        ref: "crates/monkey-core/tests/device_discovery_test.rs#test_group_devices_deduplicates_macos_multi_usage_records"
        status: pass
    human_judgment: false
  - id: D4
    description: "HidTransport Report ID 0 userspace prefix prepending and offset calculations"
    requirement: "DISC-01"
    verification:
      - kind: unit
        ref: "crates/monkey-core/tests/hid_transport_test.rs#test_frame_bulk_buffer_report_id_zero"
        status: pass
      - kind: unit
        ref: "crates/monkey-core/tests/hid_transport_test.rs#test_calculate_payload_written"
        status: pass
    human_judgment: false
  - id: D5
    description: "Non-destructive connection state heartbeat detection (WiredUsb, WirelessAwake, WirelessSleeping)"
    requirement: "DISC-05"
    verification:
      - kind: unit
        ref: "crates/monkey-core/tests/hid_transport_test.rs#test_evaluate_connection_state_wired_success"
        status: pass
      - kind: unit
        ref: "crates/monkey-core/tests/hid_transport_test.rs#test_evaluate_connection_state_wireless_awake"
        status: pass
      - kind: unit
        ref: "crates/monkey-core/tests/hid_transport_test.rs#test_evaluate_connection_state_wireless_sleeping"
        status: pass
    human_judgment: false

duration: 10min
completed: 2026-09-13
status: complete
---

# Phase 1 Plan 02: Dual-Interface HID Discovery, HidTransport, and Heartbeat State Detection Summary

**Dual-interface HID device discovery, composite interface disambiguation (`0xFF68:0x0061` bulk OUT vs `0x000C:0x0001`/`0xFFFF` control), userspace Report ID 0 buffer framing, and non-destructive connection heartbeat detection implemented with 21 automated unit tests.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-09-13T07:05:00Z
- **Completed:** 2026-09-13T07:15:00Z
- **Tasks:** 3
- **Files modified:** 7
- **Commits:** 3

## Accomplishments

- Implemented `device.rs` discovery module declaring canonical identity constants (`MONKA_VID = 0x05AC`, `MONKA_PID = 0x024F`, `PRODUCT_IDENTIFIER = "RKGK890"`), `DiscoveredDevice`, and `init_hidapi()` leveraging `macos-shared-device` (`hid_darwin_set_open_exclusive(0)`) to ensure non-exclusive access without locking OS keyboard drivers.
- Implemented `InterfaceRole` classification and `MonkaDeviceSet` composite grouping logic to disambiguate Interface A (`0xFF68:0x0061`, 4096B OUT) and Interface B (`0x000C:0x0001` or `0xFFFF`), deduplicating multi-record descriptors emitted by macOS `IOHIDManager` while retaining observed usage tuples.
- Implemented `open_device_path()` ensuring exact path-based opening rather than non-deterministic VID/PID matching.
- Implemented concrete `HidTransport` wrapping `hidapi::HidDevice` conforming to the synchronous `Transport` trait, with explicit userspace `0x00` buffer prefixing for unnumbered Report ID 0, return byte offset mathematics, and input report timeout mapping.
- Implemented `ConnectionState` (`WiredUsb`, `WirelessAwake`, `WirelessSleeping`) and non-destructive `detect_connection_state()` querying device feature reports with a finite timeout (50ms) and zero flash/DFU writes.
- Authored 21 new automated unit tests across `device_discovery_test.rs` (9 tests) and `hid_transport_test.rs` (12 tests), bringing total workspace tests to 32 passing with zero failures and zero warnings.

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement tracer device discovery and handle opening with macos-shared-device** - `cdc56b7` (feat)
2. **Task 2: Disambiguate dual HID interfaces (Interface A bulk pipe vs Interface B control pipe)** - `34e1992` (feat)
3. **Task 3: Implement HidTransport with Report ID 0 prefixing and connection heartbeat detection** - `037a056` (feat)

## Files Created/Modified

- `crates/monkey-core/src/device.rs` - Monka device discovery, constants, interface classification, grouping, and path opening
- `crates/monkey-core/src/transport/hid.rs` - Concrete `HidTransport` wrapping `hidapi::HidDevice` with Report ID 0 framing and connection state detection
- `crates/monkey-core/src/transport/mod.rs` - Exported `hid` module, `HidTransport`, and `ConnectionState`
- `crates/monkey-core/src/error.rs` - Added `From<hidapi::HidError> for TransportError`
- `crates/monkey-core/src/lib.rs` - Re-exported device discovery items, `InterfaceRole`, `MonkaDeviceSet`, `HidTransport`, and `ConnectionState`
- `crates/monkey-core/tests/device_discovery_test.rs` - 9 unit tests for interface disambiguation, grouping, deduplication, and fallback
- `crates/monkey-core/tests/hid_transport_test.rs` - 12 unit tests for buffer framing, prefix injection, offset math, and connection state evaluation

## Decisions Made

- Implemented `classify_interface` with primary `(UsagePage, Usage)` tuple matching for Interface A (`0xFF68:0x0061`) and Interface B (`0x000C:0x0001` or `0xFFFF`), falling back to `interface_number` (0 -> A, 1 -> B) when usage fields are 0 (e.g. Linux libusb backend).
- In `group_monka_devices`, deduplicated candidate records with the same device path (the macOS `IOHIDManager` behavior where multiple usage pairs on the same interface yield duplicate `DeviceInfo` records) while collecting all observed usage pairs into `observed_usages`.
- Implemented `calculate_payload_written` to return `raw_written.saturating_sub(1)`, correctly accounting for the userspace Report ID byte prefix without leaking driver framing details to callers.

## Deviations from Plan

None. All deliverables were implemented and verified according to plan specifications and project architectural invariants.

## Issues Encountered

None. All 32 automated unit tests across the workspace passed cleanly.

## User Setup Required

None.

## Next Phase Readiness

- Ready for Plan 03 (`01-03-PLAN.md`): CLI Probing Commands (`monkey probe`, `monkey info`), non-destructive hardware inspection, and machine-readable JSON output.

---
*Phase: 01-foundation-and-hardware-probing*
*Completed: 2026-09-13*
