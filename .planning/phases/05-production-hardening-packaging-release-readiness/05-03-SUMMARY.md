# 05-03 Summary: POSIX Exit Codes, Error Categorization & `cargo-deny` Compliance

## Achievements
- Established standardized POSIX exit code mappings in `crates/monkey-cli/src/error.rs`:
  - `EXIT_SUCCESS = 0`: Clean command completion.
  - `EXIT_GENERAL = 1`: Internal / operational failure.
  - `EXIT_USAGE = 2`: CLI parsing / parameter validation error.
  - `EXIT_NO_DEVICE = 3`: Monka 3075 Pro / RKGK890 keyboard not found or disconnected.
  - `EXIT_PERMISSION = 4`: OS security / TCC / udev access denied.
  - `EXIT_BLOCKED = 5`: Hardware safety gate violation (unconsented write, low battery, flash throttled).
- Implemented `classify_error` to traverse the `anyhow::Error` cause chain and map domain errors to exact exit codes.
- Wired `classify_error` and formatted error diagnostics to `stderr` in `crates/monkey-cli/src/main.rs`.
- Audited workspace dependencies with `cargo-deny` 0.20:
  - Specified `version = "0.1.0"` on `monkey-core` path dependency to eliminate wildcard violation.
  - Configured `deny.toml` skip rules for expected transitive dual-version crates (`miniz_oxide`, `syn`).
  - Achieved 100% compliance across advisories, bans, licenses, and sources with **0 errors and 0 warnings**.
- Added test coverage in `crates/monkey-cli/tests/cli_exit_codes_test.rs` validating all 5 failure categories.

## Verification
- `cargo deny check`: passed with 0 errors and 0 warnings.
- `cargo test --test cli_exit_codes_test`: 5 passed.
