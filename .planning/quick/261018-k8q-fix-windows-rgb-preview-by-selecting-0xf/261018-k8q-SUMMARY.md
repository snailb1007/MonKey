---
phase: quick
plan: 261018-k8q
subsystem: monkey-core, monkey-desktop
tags: [windows, hidapi, rgb-preview, lighting-ui, gh-18]
key-files:
  modified:
    - crates/monkey-core/src/device.rs
    - crates/monkey-core/src/transport/hid.rs
    - crates/monkey-core/tests/device_discovery_test.rs
    - apps/desktop/src/pages/LightingPage.tsx
    - apps/desktop/src-tauri/src/dto.rs
    - apps/desktop/src-tauri/src/contract_tests.rs
    - apps/desktop/tests/unit/lighting.test.tsx
decisions:
  - "Windows kernel HIDP_CAPS proved MI_01&COL05 (0xFFFF:0x0001) has FeatureReportByteLength=0 whereas MI_03 (0xFF13:0x0001) has InLen=65, OutLen=65, FeatLen=65; Windows strict Interface B classification now exclusively selects (0xFF13, 0x0001)."
  - "Windows get_feature_report now reads a 65-byte Report-ID-0 frame and strips the leading 0x00 byte into 64-byte caller buffers so RgbManager::readback_status works identically to macos_usb."
  - "LightingPage disables inapplicable parameters per mode (Off disables brightness/speed/direction/color; Static disables speed/direction; Rainbow disables color) and permits preview without direction for Off and Static modes."
completed: 2026-10-18
status: complete
---

# Quick Task 261018-k8q Summary

## Root Cause & Resolution
1. **Windows Interface B Selection (`crates/monkey-core/src/device.rs`)**:
   - Windows `HIDP_CAPS` diagnostics on physical Monka 3075 Pro hardware revealed `MI_01&COL05` (`0xFFFF:0x0001`) has `FeatureReportByteLength = 0` (4-byte input only), causing `HidD_SetFeature: (0x00000001) Incorrect function`.
   - The real 64-byte control collection on Windows is `MI_03` (`UsagePage = 0xFF13`, `Usage = 0x0001`, `InLen = 65`, `OutLen = 65`, `FeatLen = 65`).
   - Updated `windows_interface_role` to classify `(0xFF13, 0x0001)` as `InterfaceRole::InterfaceB` and reject `(0xFFFF, 0x0001)`.
2. **Windows `get_feature_report` 65-Byte Unpacking (`crates/monkey-core/src/transport/hid.rs`)**:
   - Added `unpack_windows_feature_read` and updated `HidTransport::get_feature_report` on Windows so 64-byte caller buffers (`RgbManager::readback_status`) query `HidD_GetFeature` with a 65-byte `[0x00; 65]` frame and receive the 64-byte payload (`frame[1..65]`).
3. **Mode-Aware Parameter Disabling (`apps/desktop/src/pages/LightingPage.tsx`, `apps/desktop/src-tauri/src/dto.rs`)**:
   - Disabled `Brightness`, `Speed`, `Animation Direction`, and `RGB Primary Color` when `mode === "off"`.
   - Disabled `Speed` and `Animation Direction` when `mode === "static"`.
   - Disabled `RGB Primary Color` when `mode === "rainbow"`.
   - Allowed `direction: null` in `validate_for_preview` and frontend `throttledPreview` when `mode` is `"off"` or `"static"`.

## Commits
- `fcb6f6f`: `fix(core): select 0xFF13:0x0001 Interface B and frame 65B get_feature_report on Windows`
- `9f89e94`: `fix(desktop): disable irrelevant lighting controls per mode and allow non-directional preview`
