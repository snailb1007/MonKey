# Plan 04-03 Summary: CLI monkey rgb Command Tree, Ergonomics & Headless Integration Tests

## Completed Work
1. **CLI Commands (`crates/monkey-cli/src/commands/rgb.rs`)**:
   - Implemented `monkey rgb` subcommand tree with:
     - `set`: Mode, optional color (hex/name), brightness, speed, direction, `--commit` (flash vs RAM preview), `--force`, `--mock`, `--allow-hardware-writes`.
     - `status`: Active lighting mode and state readback.
     - `save`: Export active configuration to JSON profile file or `--stdout`.
     - `restore`: Import configuration from JSON profile with `--commit` and `--force` options.
   - Provided human-readable terminal table formatting and machine-readable `--json` output across all subcommands.
   - Enforced explicit write-consent gate (`--allow-hardware-writes`) on real hardware while keeping `--mock` immediately testable.
2. **CLI Registration (`crates/monkey-cli/src/main.rs`, `crates/monkey-cli/src/commands/mod.rs`)**:
   - Exposed `rgb` module in `commands`.
   - Added `Commands::Rgb` to clap parser and routed to `run_rgb`.
3. **Integration Testing (`crates/monkey-cli/tests/cli_rgb_test.rs`)**:
   - 8 automated tests covering:
     - RAM preview on mock transport.
     - Flash commit on mock transport.
     - JSON structured output format.
     - Status inspection.
     - Profile save to disk and restore roundtrip.
     - Invalid mode input error handling.
     - Invalid hex color input error handling.
     - Hardware write-consent gate verification.

## Verification
- `cargo test -p monkey-cli --test cli_rgb_test` passes (8/8 tests).
- Full workspace test suite passes (110 tests across all crates).
