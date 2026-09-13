---
id: 260913-qzs
slug: fix-and-close-all-open-blocking-security
description: "Fix and close all open blocking security threats for Phase 02"
created: 2026-09-13
---

# Quick Task 260913-qzs: Fix and Close All Open Blocking Security Threats for Phase 02

## Goal
Implement concrete mitigations for all 6 open blocking security threats identified during the Phase 02 security audit:
1. **S-01**: Check & blacklist ISP bootloader PID `0x7140` in `device.rs`, and validate path/descriptors in `open_device_path`.
2. **T-03**: Secure `Transport` trait so callers cannot bypass `SafetyRails` or send unvalidated commands. Wrap or enforce safety checks across transmission calls.
3. **T-04**: Implement packet checksum verification on chunk/packet boundaries where CRC integrity is checked.
4. **T-05**: Enforce hardware write authorization in `TransactionManager::stream_bulk_chunks` before bulk data writes.
5. **T-06**: Implement complete transaction lifecycle (`0x18 Start` -> chunks -> `0x02 Commit` -> `0xF0 End`) with an RAII guard that sends `0xF0` cleanup frame on drop/abort.
6. **T-07**: Bind `0x02` Flash Commit strictly to an active, verified transaction / RAM preview state in `SafetyRails`.

## Tasks

### Task 1: Fix S-01 (ISP Bootloader PID Blacklist & Descriptor Validation)
- Files: `crates/monkey-core/src/device.rs`, `crates/monkey-core/tests/device_discovery_test.rs`
- Add constant `ISP_BOOTLOADER_PID: u16 = 0x7140` (and `is_isp_bootloader(vid, pid)` helper).
- In `find_monka_devices`, filter out any device matching `ISP_BOOTLOADER_PID`.
- In `open_device_path`, verify that the device opened does not match the ISP bootloader PID and matches expected MonKa VID, returning `TransportError::SecurityViolation` / `InvalidDevice` if matched.
- Add test coverage verifying detection and rejection of ISP bootloader PID `0x7140`.

### Task 2: Fix T-03, T-05, T-07 (Transport Safety Rails & Flash Commit Binding)
- Files: `crates/monkey-core/src/transport/mod.rs`, `crates/monkey-core/src/protocol/safety.rs`, `crates/monkey-core/src/protocol/transaction.rs`, `crates/monkey-core/tests/transaction_safety_test.rs`
- Update `SafetyRails` to track active transaction state: `in_transaction: bool`, `ram_preview_verified: bool`. Require `ram_preview_verified` before `FlashCommit` (`0x02`) is permitted.
- Add hardware write authorization checking in `TransactionManager::stream_bulk_chunks` (`allow_hardware_writes: bool`).
- Gate `Transport::write_bulk` and `Transport::send_feature_report` through `SafetyRails` or secure wrapper so that arbitrary raw writes are prevented.

### Task 3: Fix T-04 and T-06 (Checksum Boundary Integrity & RAII Transaction Lifecycle)
- Files: `crates/monkey-core/src/protocol/framing.rs`, `crates/monkey-core/src/protocol/transaction.rs`, `crates/monkey-core/src/protocol/crc.rs`, `crates/monkey-core/tests/protocol_codecs_test.rs`, `crates/monkey-core/tests/transaction_safety_test.rs`
- Add CRC16 verification helper to `BulkTransferPlan` / `ChunkIterator` ensuring data integrity over chunk boundaries.
- Implement `TransactionGuard` with RAII drop behavior sending `0xF0 EndTransaction` if dropped while an active transaction (`0x18`) is in progress.
- Provide full transaction execution sequence method in `TransactionManager`: `begin_transaction` (`0x18`) -> `stream_bulk_chunks` -> `commit_flash` (`0x02`) -> `end_transaction` (`0xF0`).
- Update `02-SECURITY.md` marking all 6 blocking threats as closed with verified implementation references.

## Verification
- Run `cargo test --workspace` ensuring all unit, integration, and CLI tests pass.
- Run `cargo check --all-targets` and `cargo clippy --all-targets`.
