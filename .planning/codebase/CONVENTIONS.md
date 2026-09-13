# Codebase Conventions

## Architecture & Crate Boundaries

- **`monkey-core`**: Core hardware driver and protocol implementation. Exposes a synchronous API with an internal dedicated OS worker thread; zero Tokio runtime dependencies.
- **`monkey-cli`**: Terminal command interface (`clap` v4). Handles CLI flags, terminal progress rendering (`indicatif`), and maps user actions to `monkey-core`.
- **Transport Abstraction**: All hardware interactions route through the `Transport` trait (`HidTransport` for real devices, `MockTransport` for deterministic unit testing).

## Memory & Hardware Safety Rails

- **Zero Speculative Flash Writes**: Read before write; live configuration changes go to RAM buffers before debounced flash commits.
- **Opcode Whitelisting**: Every protocol opcode must be validated against known capability matrices before transmission (`SafetyRails`).
- **Timing & Pacing Enforcement**: Inter-chunk delays (10-25ms) are strictly enforced in `protocol::channel::HardwareChannel` and `lcd::LcdStreamer` to prevent MCU buffer overruns.
- **Report ID 0 Handling**: macOS `IOHIDDeviceSetReport` strips Report ID 0; `HidTransport` handles prepending `0x00` while protocol codecs produce raw on-wire bytes.

## Data Layout & Types

- **Zero-Copy Serialization**: Packet structures use `zerocopy` (`FromBytes`, `IntoBytes`, `KnownLayout`) with explicit endianness types (`U16<LittleEndian>`, etc.).
- **LCD Frame Layout**: 128x128 pixels in RGB565 format (32,768 bytes), segmented into 8 chunks of 4096 bytes (`LCD_CHUNK_COUNT = 8`, `LCD_CHUNK_SIZE = 4096`).

## Error Handling & Logging

- **Domain Errors**: Strongly-typed errors in `monkey-core::error::MonkeyError` and `TransportError` using `thiserror`.
- **CLI Errors**: Application-level error chaining and reporting in `monkey-cli` using `anyhow::Result`.
- **Diagnostics**: Structured instrumentation via `tracing` (`trace!`, `debug!`, `info!`).

## Testing Conventions

- **Mock Transports**: Hardware-independent tests use `MockTransport` with recorded expectation calls (`TransportCall`).
- **Unit & Integration Coverage**: Protocol codecs, CRC checksums, image preprocessing, and LCD chunk framing maintain automated test coverage under `crates/*/tests`.
