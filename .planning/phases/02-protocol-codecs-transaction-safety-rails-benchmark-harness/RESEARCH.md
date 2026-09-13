# Phase 02: Protocol Codecs, Transaction Safety Rails & Benchmark Harness - Research

## Executive Summary
Phase 02 builds upon the hardware probing and dual-interface discovery foundation established in Phase 01. The objective is to implement:
1. Shenzhen HFD vendor packet codecs with zero-copy binary serialization/deserialization for 64-byte feature reports (Interface B) and 4096-byte bulk reports (Interface A).
2. Protocol transactions with strict lifecycle sequencing: Transaction Start (0x18) -> Data transfer / command -> Commit (0x02) -> Transaction End (0xF0).
3. Checksum generation and verification using CRC16 algorithms (via `crc` crate) and XOR checksums.
4. Capability matrix and hardware safety gates preventing blind flash writes, enforcing RAM buffer preview before flash commit, and blocking unverified/dangerous opcodes.
5. Synthetic benchmark harness for simulating 10-15 FPS bulk streaming and latency measurement without physical hardware dependency.

## Key Findings & Prior Art Analysis

### 1. Dual-Interface Architecture & Packet Sizing
- **Interface B (Configuration & Control Pipe)**:
  - Usage Page: `0xFFFF` (vendor-defined) or standard keyboard/mouse/consumer control composite on wired USB.
  - Report Type: Feature Report, fixed 64 bytes on-wire.
  - Packet Structure (`FeatureReportPacket`):
    - Byte 0: Magic (`0x04`)
    - Byte 1: Opcode (`0x18` start, `0x11` keymap, `0x13` RGB single packet, `0x20` RGB matrix, `0x02` save, `0xF0` end, `0xF5` readback)
    - Bytes 2..7: Command arguments (6 bytes)
    - Byte 8: Chunk index / chunk count
    - Bytes 9..13: Reserved (5 bytes)
    - Bytes 14..15: Marker `[0xAA, 0x55]`
    - Bytes 16..63: Payload (48 bytes)
- **Interface A (Vendor Bulk Pipe)**:
  - Usage Page: `0xFF68`, Usage: `0x0061`
  - Report Type: Output report, 4096 bytes per chunk.
  - Total frame for 128x128 RGB565 display: 32,768 bytes = 8 chunks of 4096 bytes.

### 2. Transaction Sequence & Safety Rails
- State machine sequence:
  - `Idle` -> `start_transaction()` (emits `0x04 0x18 ...`) -> `InTransaction`
  - In transaction: execute commands or write chunks.
  - Optional `commit()` (emits `0x04 0x02 ...`) to persist to flash.
  - Always `end_transaction()` (emits `0x04 0xF0 ...`) -> `Idle`.
- RAII Safety Guard: If a transaction guard drops while still open without explicit commit/finish, it auto-aborts or cleanly ends the transaction to avoid leaving keyboard MCU in an unstable state.
- Write Protection:
  - Only whitelisted opcodes (`0x18`, `0x13`, `0x20`, `0x11`, `0x02`, `0xF0`, `0xF5`) are permitted.
  - Dangerous bootloader/ISP opcodes (e.g. `0x55`, `0x7140`, flash erase) are explicitly rejected by the Capability Matrix.
  - RAM preview mode bypasses flash commit (`0x02`), avoiding EEPROM/Flash endurance wear.

### 3. Checksum Verification
- Shenzhen HFD protocols utilize CRC16 (CCITT / MODBUS / ARC depending on packet type) or XOR fold.
- The `crc` crate (version 3.4.0) is already specified in STACK.md and available for zero-copy computation over packet slices.

### 4. Benchmark Harness Requirements
- Must evaluate synthetic throughput of Interface A (4096-byte chunks) at 10-15 FPS (327 - 491 KB/s).
- Must measure roundtrip latency for Interface B 64-byte command transactions.
- Provides a CLI command `monkey bench` with configurable duration, synthetic frame generator, and statistical summary (min/max/avg latency, throughput, jitter).

## Dependencies & Crates
- `zerocopy` (0.8.57): derives on packet structs with endian-safe primitives.
- `crc` (3.4.0): CRC calculation.
- `thiserror` (2.0.20): domain errors (`ProtocolError`, `SafetyViolationError`).
- `crossbeam-channel` (0.5.17): queueing synthetic streaming frames in benchmark harness.
- `indicatif` (0.18.6): CLI progress indicators for benchmark run.

## Validation Strategy
- Unit tests: packet serialization, deserialization round-trips, corrupt packet rejections, checksum verification.
- Transaction state machine tests: lifecycle validation, illegal transitions, RAII auto-close on drop.
- Safety gate tests: opcode whitelist enforcement, unauthorized flash commit prevention.
- Mock transport integration tests: end-to-end transaction playback.
- Benchmark tests: timing verification and throughput calculation.
