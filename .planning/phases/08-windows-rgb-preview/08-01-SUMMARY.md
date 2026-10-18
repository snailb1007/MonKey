---
phase: 08-windows-rgb-preview
plan: 01
subsystem: driver
tags: [hidapi, windows, hid, rgb, feature-report, interface-policy, rust]

# Dependency graph
requires:
  - phase: 07-retro-pixel-desktop
    provides: [desktop worker preview route, MockHardwareBackend rgb_tests harness]
provides:
  - Windows-only feature-frame normalization at the hidapi boundary (report ID 0 prefix, no double-prefix)
  - Strict Windows interface policy (exact vendor tuples only) with legacy Mac/Linux branch byte-identical
  - Private strict-flag grouping seam per plan letter: production calls it with cfg!(target_os = "windows"); strict matrix executes on every host
  - Strengthened desktop RAM-preview regression (exact raw64 bytes, command != 0x02, no bulk/save records)
affects: [08-windows-rgb-preview plan 02+, native Windows CI, GH #18 closure]

# Actuals — chars/4 over the realized diff (round-3 rework diff vs review-round-2 HEAD 89511aa)
actuals:
  tokens: 9200
  tasks: 2
  commits: 1

# Tech tracking
tech-stack:
  added: []
  patterns: [private policy-parameterized helpers with colocated unit tests, cfg!(target_os) production selection with unconditional seam tests]

key-files:
  created: []
  modified:
    - crates/monkey-core/src/device.rs
    - crates/monkey-core/tests/device_discovery_test.rs
    - apps/desktop/src-tauri/src/rgb_tests.rs
    - .planning/phases/08-windows-rgb-preview/08-01-SUMMARY.md
  removed:
    - 08-01-SUMMARY.md (worktree root duplicate; canonical summary lives beside the plan)

key-decisions:
  - "Reviewer round-3 direction restores the plan-letter private policy-aware grouping seam: classify_with_policy / role_with_policy / matches_set_with_policy / group_monka_devices_with_policy are private; production classify_interface and group_monka_devices invoke them with cfg!(target_os = windows). No new public API solely for tests."
  - "normalize_feature_frame stays private in transport/hid.rs; the Windows send_feature_report branch is its sole production caller."
  - "No worker, IPC, codec, safety, opcode, or manifest changes; matches_set keeps the base role-first order and the base matches_set symbol stays on the legacy path."
  - "Worktree-root 08-01-SUMMARY.md removed via git rm; this beside-the-plan file is the single canonical summary."

patterns-established:
  - "Platform policy via private strict-flag seam: production selects with cfg!, colocated tests invoke both policies on any host through the same tested helper."
  - "Framing failures before I/O: malformed feature input is rejected by a pure function before any HidDevice handle is touched."

requirements-completed: [DISC-01, DISC-04, RGB-01, RGB-02, PROT-03, PROT-05, UI2-03, UI2-05]

coverage:
  - id: D1
    description: "Windows feature-send normalization (raw64 -> 65-byte ID0 frame, prefixed65 passthrough, all rejections before I/O)"
    requirement: "PROT-03"
    verification:
      - kind: unit
        ref: "crates/monkey-core/src/transport/hid.rs#normalize_* (6 tests)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Strict Windows interface policy (accept exact vendor tuples; reject consumer/unknown FFFF/kb/mouse/zero-usage)"
    requirement: "DISC-01"
    verification:
      - kind: unit
        ref: "crates/monkey-core/src/device.rs#windows_policy_tests pure helper tests (2 tests, run on every host)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Windows grouping matrix through the production-called seam (vendor beats consumer both orders, consumer-only yields no B, unknown/zero rejected, serial pairs never cross-pair)"
    requirement: "DISC-04"
    verification:
      - kind: unit
        ref: "crates/monkey-core/src/device.rs#test_group_windows_*_via_seam (5 strict tests) + test_group_legacy_consumer_b_pairs_via_seam, all run on macOS arm64"
        status: pass
      - kind: integration
        ref: "crates/monkey-core/tests/device_discovery_test.rs Windows cfg tests (serial-paired dual-keyboard, single-pair)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Legacy Mac/Linux classification and grouping unchanged"
    requirement: "DISC-04"
    verification:
      - kind: integration
        ref: "crates/monkey-core/tests/device_discovery_test.rs (20 passed on macOS, incl. new cfg(not(windows)) Consumer-B pairing test)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Desktop RAM-preview route sends the exact raw64 codec packet once with no bulk/save records; command is RAM control, never 0x02 save"
    requirement: "RGB-01"
    verification:
      - kind: integration
        ref: "apps/desktop/src-tauri/src/rgb_tests.rs#test_rgb_preview_payload_matches_codec_ram_only_no_save_no_bulk"
        status: pass
    human_judgment: false
  - id: D6
    description: "Native Windows build/test/clippy evidence for root and desktop workspaces"
    requirement: "UI2-05"
    verification: []
    human_judgment: true
    rationale: "No Windows runner exists in this session; requires native Windows CI or maintainer run."
  - id: D7
    description: "Human wired-USB smoke: physical preview works, no HidD_SetFeature Incorrect-function error"
    requirement: "RGB-02"
    verification: []
    human_judgment: true
    rationale: "Mock transports cannot prove OS/hardware behavior; task 3 human gate owns this."

# Metrics
duration: multi-turn agent session on 2026-10-18
completed: 2026-10-18
status: pending
---

# Phase 08-01: Windows RGB Preview Routing (Review Round 3) Summary

**Strict Windows vendor-collection policy and report-ID-zero feature framing behind a private strict-flag seam per the plan letter; the strict grouping matrix executes on every host through the same helper production calls; Mac/Linux behavior byte-identical.**

## Performance

- **Duration:** multi-turn agent session on 2026-10-18
- **Completed:** 2026-10-18
- **Tasks:** 2 (core seam rework + desktop/summary evidence)
- **Files modified:** 3 code/test files + this summary; 1 root duplicate summary removed

## Base and Commits

- **Base SHA:** `30fab111b35a242bc898d5e4c9718b7767373db0` (origin/dev, operator-approved)
- **Evidence-time HEAD:** `89511aa90fb0b2e9fcf23ab6e24559db0abb7263` (review round-2 commit; all verify runs below executed against the working tree containing the round-3 changes on top of this HEAD)
- **This commit:** round-3 rework commit on worktree branch `wt/t_22c00c4c` (see Task Commits)
- **Diff vs base:** 6 files (incl. this summary), +1122/−34; code files: `device.rs` (1188 lines), `hid.rs` (370), `device_discovery_test.rs` (488), `hid_transport_test.rs` (215), `rgb_tests.rs` (422)

## Accomplishments

- Private `normalize_feature_frame` in `transport/hid.rs`, called by the Windows
  `send_feature_report` branch: raw64 magic04 payload gains the `0x00` report-ID
  prefix (65 bytes), already-framed 65-byte input passes through identically
  (LCD double-prefix impossible), zero-length/oversized/nonzero-prefix/wrong-magic
  rejected before any device handle is touched.
- Strict Windows interface policy as the only Windows branch: exact
  `(0xFF68, 0x0061)` -> A and `(0xFFFF, 0x0001)` -> B; consumer control,
  unknown FFFF usages, keyboard/mouse, and zero-usage metadata rejected.
  Mac/Linux branch and interface-number fallback preserved exactly.
- Private strict-flag seam per plan task 1 letter: `classify_with_policy`,
  `DiscoveredDevice::role_with_policy`, `matches_set_with_policy`,
  `group_monka_devices_with_policy`. Production `classify_interface` and
  `group_monka_devices` invoke them with `cfg!(target_os = "windows")`.
  Legacy path reuses the base `matches_set` symbol (role-first order restored;
  `DiscoveredDevice::role(d)` churn reverted), so the CRITICAL grouping diff
  stays minimal. No public API added solely for tests.
- Strict grouping matrix runs on every host: 5 colocated strict tests
  (vendor-beats-consumer both orders, consumer-only yields no B, unknown/zero
  rejected, serial pairs never cross-pair) plus legacy seam tests, 8 colocated
  tests total, all passing on macOS arm64.
- Dual-keyboard integration test fixed with reliable serial same-identity
  (KB1/KB1/KB2): without serials two unkeyed A candidates are ambiguous and
  default-deny isolation yields three singletons, so serials are the supported
  pairing signal; the test now asserts 2 sets with KB1 complete.
- New `cfg(not(target_os = "windows"))` Consumer-B pairing test keeps legacy
  Mac/Linux grouping behavior explicitly covered.
- Desktop preview test renamed to
  `test_rgb_preview_payload_matches_codec_ram_only_no_save_no_bulk`: asserts
  the exact 64-byte raw codec packet (magic 04, command 0x13, color bytes,
  AA55 markers), command explicitly `!= 0x02` (never the save command),
  exactly one SendFeature record, zero bulk/write records, volatile
  RAM persistence result; in-test comment states the mock bypasses HID so the
  test proves worker emission only, not Windows framing or collection
  selection.
- Worktree-root `08-01-SUMMARY.md` removed via `git rm`; this beside-the-plan
  file is the single canonical summary.

## Task Commits

1. **Task 1: core strict-flag seam rework** - round-3 commit (refactor/test)
2. **Task 2: desktop strengthening + summary evidence** - same commit (test/docs)

**Plan metadata:** summary-only change set beside code; no STATE/ROADMAP edits.
**Plan tasks 1-2:** done (true). **Plan task 3 (native Windows + human smoke):** pending, not closable by this session.

## Files Created/Modified

- `crates/monkey-core/src/transport/hid.rs` - private `normalize_feature_frame` + Windows-branch wiring + 6 colocated unit tests (unchanged in round 3)
- `crates/monkey-core/src/device.rs` - strict policy + private `*_with_policy` seam, base-order `matches_set` preserved, builders unconditional, grouping matrix via seam on every host + legacy seam test
- `crates/monkey-core/tests/hid_transport_test.rs` - unbound-preview regression (unchanged in round 3)
- `crates/monkey-core/tests/device_discovery_test.rs` - serial-paired dual-keyboard fix, new legacy Consumer-B test under `cfg(not(windows))`
- `apps/desktop/src-tauri/src/rgb_tests.rs` - renamed RAM-only test with `!= 0x02` assertion and mock-bypass comment
- `.planning/phases/08-windows-rgb-preview/08-01-SUMMARY.md` - this file (rewritten with exact verify-run evidence)
- `08-01-SUMMARY.md` (worktree root) - REMOVED via `git rm`

## Verification (exact commands, macOS arm64, 2026-10-18 ~04:03 UTC, evidence-time HEAD 89511aa90fb0)

All ten plan verification commands executed through `~/.hermes/bin/verify-run` from the worktree root; every run reports `VERIFY_STATUS: PASSED`, exit 0:

1. `cargo test -p monkey-core --lib --test hid_transport_test --test device_discovery_test --test monka_device_test` → PASSED, DURATION 3s, LOG_PATH `/Users/ethan/.hermes/cache/runs/t_22c00c4c_1792296211_39650.log`. Tallies: lib 15 passed; device_discovery 20 passed; hid_transport 16 passed; monka_device 11 passed; 0 failed.
2. `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --lib rgb_tests` → PASSED, DURATION 0s, LOG_PATH `/Users/ethan/.hermes/cache/runs/t_22c00c4c_1792296214_39799.log`. Tallies: 10 passed, 0 failed, 30 filtered out.
3. `cargo fmt --all -- --check` → PASSED, DURATION 0s, LOG_PATH `/Users/ethan/.hermes/cache/runs/t_22c00c4c_1792296150_36854.log` (clean after one mechanical `cargo fmt` pass; no manual edits).
4. `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --all -- --check` → PASSED, DURATION 0s, LOG_PATH `/Users/ethan/.hermes/cache/runs/t_22c00c4c_1792296150_36869.log`.
5. `cargo test --workspace --all-targets` → PASSED, DURATION 14s, LOG_PATH `/Users/ethan/.hermes/cache/runs/t_22c00c4c_1792296154_36990.log`. Tallies: 12+4+3+5+3+10+8+15+20+16+13+7+11 = 127 passed, 0 failed (plus two 0-test binaries).
6. `cargo clippy --workspace --all-targets -- -D warnings` → PASSED, DURATION 1s, LOG_PATH `/Users/ethan/.hermes/cache/runs/t_22c00c4c_1792296189_38641.log`. Zero warnings.
7. `cargo build --workspace --all-targets` → PASSED, DURATION 0s, LOG_PATH `/Users/ethan/.hermes/cache/runs/t_22c00c4c_1792296193_38904.log`.
8. `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets` → PASSED, DURATION 8s, LOG_PATH `/Users/ethan/.hermes/cache/runs/t_22c00c4c_1792296193_38962.log`. Tallies: 40 passed, 0 failed.
9. `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings` → PASSED, DURATION 3s, LOG_PATH `/Users/ethan/.hermes/cache/runs/t_22c00c4c_1792296204_39298.log`. Zero warnings.
10. `cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets` → PASSED, DURATION 1s, LOG_PATH `/Users/ethan/.hermes/cache/runs/t_22c00c4c_1792296207_39440.log`.

- TDD receipts: the new seam tests were written against the pre-seam code first (nonexistent `group_monka_devices_with_policy` → compile failure = RED), then passed after the seam implementation (GREEN); the serial-pairing and `!= 0x02` assertions encode the reviewed failure modes. No new test passed on first run without exercising the failure.
- Graph: gitnexus `impact` (upstream, repo MonKey) run pre-edit for `classify_interface` (HIGH), `group_monka_devices` (CRITICAL, 13 impacted functions/processes), `send_feature_report` (UNKNOWN, zero resolved callers → corroborated by text search: 131 matches via the `Transport` trait dynamic dispatch in lcd streamer / rgb manager / SafeTransport, change confined to the Windows branch). `detect_changes --scope all` run pre-commit: 11 changed symbols, 24 affected, risk_level critical (expected: grouping path touched); affected processes are the expected discover/open/diagnose flows through group/find/new with no unexpected blast radius; full result in the review handoff metadata.

## Decisions Made

- Followed reviewer round-3 findings, which restore the plan task-1 letter
  (private policy-aware grouping seam) after round 2 had removed it;
  reviewer direction is binding at implementation time.
- Legacy production path reuses the base `matches_set` symbol (with
  `cfg!`-based delegation inside) so the CRITICAL diff stays minimal and the
  symbol is never dead on any target; strict path uses the policy-aware
  helper through the same tested seam.
- Dual-keyboard test uses reliable serial identity rather than asserting
  three ambiguous singletons: serials are the supported pairing signal and
  match the plan's "reliable synthetic serial identity for positive pairing".

## Deviations from Plan

### Auto-fixed Issues

**1. [Reviewer binding - scope] Restored the plan-requested private grouping seam (round-2 removal reversed)**
- **Found during:** Task 1 (review round-3 findings HIGH #4 / LOW #7)
- **Issue:** Round 2 deleted `classify_with_policy` / `group_monka_devices_with_policy`; grouping matrix lived only in `cfg(windows)` tests that compile out on Mac, so the strict matrix never executed on the Mac host.
- **Fix:** Restored all four helpers as private (no public API), production invokes with `cfg!(target_os = "windows")`, colocated strict tests drive the seam unconditionally; role-first order and base `matches_set` symbol preserved.
- **Files modified:** `crates/monkey-core/src/device.rs`
- **Verification:** 8/8 colocated tests pass on macOS arm64; workspace suite green.
- **Impact on plan:** Implements the plan task-1 letter exactly; no scope creep.

**2. [Reviewer binding - scope] Root summary duplicate removed**
- **Found during:** Task 2 (review round-3 finding HIGH)
- **Issue:** Worktree-root `08-01-SUMMARY.md` duplicated the beside-the-plan summary with weaker verification.
- **Fix:** `git rm 08-01-SUMMARY.md`; this file is canonical.
- **Files modified:** `08-01-SUMMARY.md` (deleted)
- **Verification:** `git status` shows `D 08-01-SUMMARY.md` staged with code changes.
- **Impact on plan:** Docs-only; no behavior impact.

**Total deviations:** 2 (both reviewer-directed, no scope creep)
**Impact on plan:** Plan tasks 1–2 done (true); task 3 gates unchanged and explicitly tracked below. No must-have set false by this session.

## Issues Encountered

- `cargo fmt --check` failed on the new test code (multi-line `assert!` formatting); fixed with mechanical `cargo fmt` (no manual edits), both fmt checks re-verified clean.
- First seam implementation left the base `matches_set` wrapper dead (clippy `dead_code` warning → would fail `-D warnings`); fixed by routing the legacy path through the base symbol with a strict/legacy dispatch in `group_monka_devices_with_policy`, plus a legacy-via-seam test so the symbol is live on every target.
- Shell `grep` is aliased to the `rtk` wrapper in this environment; used `/usr/bin/grep` for log tally extraction.

## No-Hardware Statement

No HID hardware was connected, enumerated, opened, or written to in this session. All evidence comes from `MockTransport` call records and pure-function unit tests, which bypass the OS HID stack entirely. Mock success does not prove Windows framing or collection selection on real hardware.

## Native Windows / Human Status

- **PENDING:** native Windows `cargo test/clippy/build` for root + desktop (no Windows runner in this session; task 3 gate).
- **PENDING:** human wired-USB RAM-preview smoke on a reviewed Windows build (task 3 gate). GH #18 stays open until then.
- **Revert strategy:** `git revert` of the round-3 rework commit; never a runtime protocol fallback.

## User Setup Required

None - no external service configuration required. (Windows wired-USB smoke is owned by plan task 3, not agent setup.)

## Next Phase Readiness

- Code + summary ready for independent reviewer (`reviewer` profile) via kanban review handoff (round-3 re-review).
- Blockers for integration: independent review approval, native Windows evidence, human smoke (all tracked above, none closable by this session).

---
*Phase: 08-windows-rgb-preview, plan 01 (review round 3)*
*Status: pending — awaiting re-review; native Windows and human gates remain open*
