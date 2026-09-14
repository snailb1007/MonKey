# Phase 4: Ambient RGB Engine, Matrix Mapping & Two-Tier State Persistence - Research

**Date:** 2026-09-14
**Status:** Completed
**Domain:** USB HID Feature Reports, RGB Lighting Protocol, SPI Flash Endurance, 81-Key Matrix

## Executive Summary

Phase 4 delivers ambient RGB lighting controls, two-tier state synchronization (30Hz RAM Preview vs 500ms debounced Flash Commit), low-battery safety gating on wireless connections, JSON profile backup/restore with readback verification, and an 81-key hardware LED matrix abstraction for the Monka 3075 Pro (Shenzhen HFD / RKGK890, VID `0x05AC`, PID `0x024F`).

All communication for RGB and configuration uses **Interface B** (`Usage Page 0xFFFF`, `Usage 0x0001` or standard control collection) via 64-byte feature reports (`IOHIDDeviceSetReport` / `IOHIDDeviceGetReport` on macOS).

## 1. Protocol Architecture & Feature Report Codes

### Opcode Mapping
From prior art analysis (`research/prior_art_protocol.md`) and verified `GMK67` Shenzhen HFD captures:
- `04 18`: `CommandId::StartTransaction` (Open transaction session)
- `04 13`: `CommandId::RgbControl` (Single-packet ambient RGB mode, color, brightness, speed setting)
- `04 20`: `CommandId::RgbMatrix` (Per-key RGB table, 8 chunks)
- `04 02`: `CommandId::SaveSettings` (Commit settings to onboard SPI NOR flash)
- `04 F0`: `CommandId::EndTransaction` (Close transaction session)
- `04 F5`: `CommandId::StateReadback` (Readback configuration/RGB table)

### Gói tin `04 13` (RgbControl) Layout:
The 64-byte feature report for ambient RGB:
```
Offset  Field         Type    Description
0x00    magic         u8      0x04 (FEATURE_REPORT_MAGIC)
0x01    command       u8      0x13 (CommandId::RgbControl)
0x02    mode          u8      Lighting mode enum (0x01 = Static, 0x02 = Breath, etc.)
0x03    r             u8      Red component (0..255)
0x04    g             u8      Green component (0..255)
0x05    b             u8      Blue component (0..255)
0x06..0x07 reserved   [u8;2]  0x00
0x08    colortype     u8      Color type (0 = RGB, 1 = Rainbow/Dynamic)
0x09    speed         u8      Speed (1..5 or normalized 0..100)
0x0A    brightness    u8      Brightness (1..5 or normalized 0..100)
0x0B    direction     u8      Flow direction (0 = left/right, 1 = right/left)
0x0C..0x0D reserved   [u8;2]  0x00
0x0E    marker_lo     u8      0xAA (FEATURE_REPORT_MARKER[0])
0x0F    marker_hi     u8      0x55 (FEATURE_REPORT_MARKER[1])
0x10..0x3F payload    [u8;48] 0x00
```

### Supported Ambient Lighting Modes
1. `Static` (Single fixed color)
2. `Breathing` (Pulsing color)
3. `Wave` (Flowing rainbow waves)
4. `Rainbow` (Cycling spectrum)
5. `Ripple` (Keypress ripple outward)
6. `Reactive` (Keypress single-key lightup)
7. `Off` (All LEDs turned off)

## 2. Two-Tier State Model & Flash Wear Protection

### Problem: SPI NOR Flash Wear
Onboard SPI NOR flash chips typically withstand only 10,000 to 100,000 erase/write cycles per sector. If a user moves an RGB slider or runs an ambient reactive script committing at 30Hz, flash memory would burn out within hours.

### Solution: Two-Tier Architecture
1. **Volatile RAM Preview (Default)**:
   - Sends `04 13` directly to update keyboard volatile controller RAM.
   - Throttled at up to 30Hz.
   - Zero flash writes; changes reset on power cycle / disconnect.
2. **Flash Commit (Explicit `--commit`)**:
   - Executes standard transaction sequence: `04 18` -> `04 13` -> `04 02` -> `04 F0`.
   - Strictly debounced by 500ms using `SafetyRails::check_flash_commit()`.
   - Tracks atomic commit counts for hardware auditing.

### Wireless Battery Safety Gate
When communicating over wireless transport:
- If battery level is reported below 20%:
  - Flash commits are refused with error: `Battery level is below 20% on wireless transport. Flash commit blocked to prevent flash corruption.`
  - Can be bypassed with `--force` only if user explicitly accepts risk.

## 3. 81-Key Matrix Layout Mapping

### `research/layout_81keys.json` Structure
The Monka 3075 Pro has an 81-key exploded 75% layout:
- Rows 0 to 5, Columns 0 to 15.
- Each key has:
  - `name`: Key label (e.g., `"Esc"`, `"Enter"`, `"Space"`, `"Fn"`).
  - `row`, `col`: Matrix scan grid position.
  - `rgb_light_index`: Physical index in the LED chain (0..80).
  - `keycode`: Standard HID usage keycode.

### Implementation:
A compile-time or lazy static `KeyMatrix` struct:
- Provides `lookup_by_name(name: &str) -> Option<&KeyDefinition>`.
- Provides `lookup_by_pos(row: u8, col: u8) -> Option<&KeyDefinition>`.
- Provides `lookup_by_led_index(index: u8) -> Option<&KeyDefinition>`.
- Validates that out-of-bounds indices are rejected before packet construction.

## 4. Profile Management & Backup/Restore

### Profile JSON Schema (`rgb_profile.json`):
```json
{
  "schema_version": 1,
  "model": "Monka 3075 Pro",
  "created_at": "2026-09-14T08:00:00Z",
  "lighting": {
    "mode": "Wave",
    "color": "#FF0000",
    "brightness": 80,
    "speed": 50,
    "direction": "LeftToRight"
  }
}
```

### Save & Restore Flow:
- `monkey rgb save [--file <path>]`:
  - Exports active settings to JSON format.
- `monkey rgb restore [--file <path>] [--commit]`:
  - Parses and validates JSON file schema.
  - Sends `04 13` (and optionally `04 02` if `--commit`).
  - Verifies written configuration via `04 F5` readback.

## 5. Architectural Boundaries & Modules

- `crates/monkey-core/src/rgb/`:
  - `mode.rs`: `LightingMode`, `RgbColor`, `LightingConfig`.
  - `matrix.rs`: `KeyDefinition`, `KeyMatrix`, embedded `layout_81keys.json`.
  - `profile.rs`: JSON serialization, schema validation, import/export.
  - `manager.rs`: `RgbManager` orchestrating RAM preview, debounced flash commit, and readback.
- `crates/monkey-cli/src/commands/rgb.rs`:
  - Subcommands: `set`, `status`, `save`, `restore`.
  - Argument parsing: color parsing (`#RRGGBB`, `RRGGBB`, named aliases), speed, brightness, `--commit`, `--file`.
  - Terminal human and JSON formatting.
