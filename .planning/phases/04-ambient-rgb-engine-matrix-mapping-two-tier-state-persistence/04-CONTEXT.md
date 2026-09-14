# Phase 4: Ambient RGB Engine, Matrix Mapping & Two-Tier State Persistence - Context

**Gathered:** 2026-09-14
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous mode)

<domain>
## Phase Boundary

As a keyboard user, I want to configure ambient lighting and save profiles with two-tier flash wear protection, so that I can customize RGB effects without wearing out onboard SPI flash memory.

### Success Criteria
1. User can change ambient lighting mode, speed, brightness, and static RGB color using `monkey rgb set`.
2. Rapid interactive lighting adjustments update volatile RAM at up to 30Hz without triggering SPI NOR flash wear, debouncing flash commits to a single write after 500ms of inactivity.
3. Flash commit operations are automatically blocked when the keyboard reports battery level below 20% on wireless connections, displaying a clear safety warning.
4. User can back up the active lighting configuration with `monkey rgb save` and restore it with `monkey rgb restore`, verified via `04 F5` readback.

</domain>

<decisions>
## Implementation Decisions

### CLI Syntax & Ergonomics
- **Command hierarchy**: Dedicated subcommands under `monkey rgb`: `set`, `save`, `restore`, `status`.
  *Rationale*: Clear separation of concerns matching existing CLI patterns (`monkey lcd ...`).
- **Color format**: Support Hex strings (`#RRGGBB` or `RRGGBB`) plus common named color aliases (`red`, `green`, `blue`, `cyan`, `magenta`, `yellow`, `white`, etc.).
  *Rationale*: User preference established in Smart Discuss; convenient for both quick typing and exact hex specification.
- **Parameter validation**: Strict range validation for brightness (0-100) and speed (0-100) with descriptive error messages and hints.
  *Rationale*: Prevents out-of-range protocol parameters from reaching the hardware layer.
- **Default feedback**: Formatted output showing applied mode, brightness, speed, color, and write mode (volatile RAM preview vs permanent Flash commit).
  *Rationale*: Transparently informs the user whether the change was persisted to flash or kept in RAM.

### Two-Tier State Persistence & Debounce Mechanics
- **Flash commit trigger behavior**: Immediate volatile RAM preview by default; permanent flash save requires `--commit` flag.
  *Rationale*: Strictly protects low-cost onboard SPI NOR flash from rapid wear during interactive experimentation.
- **Low-battery safety gate**: Automatically block flash commits when the keyboard is on wireless connection and battery level < 20%, returning an error code and remediation advice; provide `--force` override.
  *Rationale*: Prevents catastrophic MCU state corruption during low-voltage flash sector erase/write cycles.
- **Flash write wear accounting**: Track flash commit count per session using atomic counter; log in diagnostics and debug traces.
  *Rationale*: Provides telemetry for hardware safety verification and debugging.
- **Debounce window duration**: Default 500ms debounce window as established in Phase 2 `SafetyRails`, configurable if necessary.
  *Rationale*: Balances interactive responsiveness with safe flash commit coalescing.

### Profile Management & Backup/Restore Format
- **Profile file format**: JSON schema (.json) containing keyboard metadata (model, timestamp, schema_version), lighting settings (mode, speed, brightness, color), and matrix configuration.
  *Rationale*: Human-readable, versionable, and easily validated against JSON schemas.
- **Default backup location**: CLI option `--file <path>`, defaulting to `./rgb_profile.json` if omitted, or stdout with `--stdout`.
  *Rationale*: Standard CLI convention that avoids hardcoded machine-dependent file paths while providing easy scripting.
- **Verification on restore**: Readback verification via `04 F5` feature report query to verify written parameters match requested profile.
  *Rationale*: Guarantees physical device state matches requested configuration before claiming success.
- **Handling unsupported profiles**: Validate schema and check mode IDs against capability matrix before sending any USB packets.
  *Rationale*: Enforces default-deny safety posture before touching the USB bus.

### 81-Key Matrix Mapping & Hardware Addressing
- **Scope for Phase 4**: Whole-board ambient lighting mode controls + 81-key matrix data model and layout parser loaded from `research/layout_81keys.json`.
  *Rationale*: Fulfills core RGB requirements while laying the typed foundational data structures for per-key addressing.
- **Matrix representation**: Strongly typed `KeyMatrix` struct in `monkey-core` mapping key positions (row, column, keycode) to hardware LED indexes.
  *Rationale*: Type-safe abstraction preventing index out-of-bounds or misaligned LED mapping.
- **Error handling for unmapped keys**: Safe error return without writing to transport if invalid key position is specified.
  *Rationale*: Preserves hardware safety rails.
- **Matrix layout loading**: Built-in 81-key layout compiled in as default, with optional `--layout <file>` override.
  *Rationale*: Works out-of-the-box for Monka 3075 Pro while remaining extensible to related OEM variants.

### The Agent's Discretion
- Internal layout and module structure in `crates/monkey-core/src/rgb/` and `crates/monkey-cli/src/commands/rgb.rs`.
- Unit and mock integration test fixtures and tests.
- Formatting details of terminal tables and `--json` machine-readable output.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `monkey_core::protocol::codecs::FeaturePacket`: 64-byte feature report construction and zerocopy layout.
- `monkey_core::protocol::safety::{SafetyGate, WriteMode, CommandId}`: Whitelist gating and 500ms flash debouncer already implemented in Phase 2.
- `monkey_core::protocol::channel::HardwareChannel`: Thread-safe worker channel for sending feature reports and transactions.
- `monkey_core::transport::{Transport, MockTransport, HidTransport}`: Complete transport abstraction with call recording.
- `research/layout_81keys.json`: Verified 81-key matrix definition.

### Established Patterns
- Strongly typed CLI commands via `clap` derive.
- Clean separation between core protocol logic in `monkey-core` and CLI interface in `monkey-cli`.
- Comprehensive mock transport tests validating packet bytes, write prevention, and error handling.
- Structured diagnostics using `tracing`.

### Integration Points
- `monkey-core::rgb` module exposing `RgbManager`, `LightingMode`, `RgbColor`, `KeyMatrix`, and profile serialization.
- `monkey-cli::commands::rgb` providing the `monkey rgb` subcommand tree (`set`, `save`, `restore`, `status`).
- Connection to `HardwareChannel` and `SafetyGate` for executing feature reports over Interface B.

</code_context>

<specifics>
## Specific Ideas
- Support named colors: `red` (0xFF0000), `green` (0x00FF00), `blue` (0x0000FF), `yellow` (0xFFFF00), `cyan` (0x00FFFF), `magenta` (0xFF00FF), `white` (0xFFFFFF), `off`/`black` (0x000000).
- Support standard modes: Static, Breathing, Wave, Ripple, Reactive, Rainbow, etc., mapped to HFD protocol opcodes.

</specifics>

<deferred>
## Deferred Ideas
- Interactive GUI color picker / live web canvas (deferred to Tauri v2 post-v1).
- Per-key animated lighting scripting language (deferred to Milestone 2).

</deferred>
