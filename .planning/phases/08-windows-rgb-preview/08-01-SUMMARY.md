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
  - Production-path Windows grouping with no test-only seams or quirk tables
  - Strengthened desktop RAM-preview regression (exact raw64 bytes, no bulk/save records)
affects: [08-windows-rgb-preview plan 02+, native Windows CI, GH #18 closure]

# Actuals — chars/4 over the realized diff (round-2 rework diff vs review-round-1 HEAD)
actuals:
  tokens: 11559
  tasks: 2
  commits: 1

# Tech tracking
tech-stack:
  added: []
  patterns: [private pure helpers with colocated unit tests, cfg-gated platform tests on production paths]

key-files:
  created:
    - .planning/phases/08-windows-rgb-preview/08-01-SUMMARY.md
  modified:
    - crates/monkey-core/src/transport/hid.rs
    - crates/monkey-core/src/device.rs
    - crates/monkey-core/tests/hid_transport_test.rs
    - crates/monkey-core/tests/device_discovery_test.rs
    - apps/desktop/src-tauri/src/rgb_tests.rs

key-decisions:
  - "Reviewer round-2 direction over plan letter: all test-only seams and quirk tables removed; Windows grouping tests run the production path under cfg(target_os = windows) on native Windows CI."
  - "normalize_feature_frame stays private in transport/hid.rs; the Windows send_feature_report branch is its sole production caller."
  - "No worker, IPC, codec, safety, opcode, or manifest changes; apply_rgb_preview guard removed back to base behavior."

patterns-established:
  - "Platform policy without test seams: cfg-gated tests drive production code; pure helpers carry host-independent unit tests."
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
        ref: "crates/monkey-core/src/device.rs#windows_policy_tests (pure helper tests)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Production-path Windows grouping (vendor beats consumer both orders, consumer-only yields no B, no cross-pairing)"
    requirement: "DISC-04"
    verification:
      - kind: integration
        ref: "crates/monkey-core/src/device.rs#test_group_windows_* (cfg windows, production group_monka_devices)"
        status: unknown
    human_judgment: true
    rationale: "Tests are cfg(target_os = windows) and compiled out on Mac; they must execute on native Windows CI before counting as proof."
  - id: D4
    description: "Legacy Mac/Linux classification and grouping unchanged"
    requirement: "DISC-04"
    verification:
      - kind: integration
        ref: "crates/monkey-core/tests/device_discovery_test.rs (19 passed on macOS)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Desktop RAM-preview route sends the exact raw64 codec packet once with no bulk/save records"
    requirement: "RGB-01"
    verification:
      - kind: integration
        ref: "apps/desktop/src-tauri/src/rgb_tests.rs#test_rgb_preview_payload_matches_codec_and_normalizes_to_hid_frame"
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
status: complete
---

# Phase 08-01: Windows RGB Preview Routing (Review Round 2) Summary

**Strict Windows vendor-collection policy and report-ID-zero feature framing with all test-only seams removed; Mac/Linux behavior byte-identical by restored base paths.**

## Performance

- **Duration:** multi-turn agent session on 2026-10-18
- **Completed:** 2026-10-18
- **Tasks:** 2 (core slice rework + desktop evidence/summary)
- **Files modified:** 5 (+ this summary)

## Base and Commits

- **Base SHA:** `30fab111b35a242bc898d5e4c9718b7767373db0` (origin/dev, operator-approved)
- **Prior HEAD:** `2d2bdbc` (review round 1: strict policy with private grouping seam)
- **This commit:** rework commit on worktree branch `wt/t_22c00c4c` (see Task Commits)
- **Diff vs round-1 HEAD:** 5 files, +187/−596 (net deletion of seam/quirk/test-bridge code)

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
- Production grouping restored to the base algorithm (exact-path, role,
  physical-key, DevSrvsID-proximity, unambiguous fallback); Windows behavior
  follows from strict classification with no quirk table, no parent-prefix
  emission, no policy seam.
- Removed per reviewer: `QuirkId`, `monka_hid_signatures`, `quirk_for_pair`,
  `windows_device_parent`, `windows_devsrvs_id`, `RoleExt`, `DualRole`,
  `classify_with_policy`, `role_with_policy`, `group_monka_devices_with_policy`,
  the `apply_rgb_preview` no-interface guard, and all public-test bridges.
- Desktop worker preview test now asserts the exact 64-byte raw codec packet
  (magic 04, command 0x13, color bytes, AA55 markers), exactly one SendFeature
  record, and zero bulk/write (no flash-save) records.

## Task Commits

1. **Task 1: core Windows selection/framing rework** - round-2 commit (refactor/test)
2. **Task 2: desktop evidence + summary** - same commit (test/docs)

**Plan metadata:** summary-only change set beside code; no STATE/ROADMAP edits.

## Files Created/Modified

- `crates/monkey-core/src/transport/hid.rs` - private `normalize_feature_frame` + Windows-branch wiring + 6 colocated unit tests
- `crates/monkey-core/src/device.rs` - strict `windows_interface_role`, base grouping restored, colocated policy/grouping tests (grouping tests `cfg(windows)`)
- `crates/monkey-core/tests/hid_transport_test.rs` - seam/bridge/guard tests removed; unbound-preview regression kept
- `crates/monkey-core/tests/device_discovery_test.rs` - quirk/signature/RoleExt/DualRole tests removed; strict-vs-legacy expectations
- `apps/desktop/src-tauri/src/rgb_tests.rs` - exact raw64 payload + no-bulk/no-save assertions; id-emission test replaced with pairing test
- `.planning/phases/08-windows-rgb-preview/08-01-SUMMARY.md` - this file

## Verification (actual commands, macOS arm64, RTK_DISABLED=1)

- `cargo test -p monkey-core` → 107 passed, 0 failed (lib 10; device_discovery 19; hid_transport 16; lcd 13; mock 7; monka_device 11; protocol_codecs 8; protocol_types 6; rgb_codecs 4; rgb_manager 5; transaction_safety 8). Zero warnings.
- `cargo clippy -p monkey-core --all-targets` → clean, exit 0, zero warnings.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml rgb_tests` → 10 passed, 0 failed.
- `cargo fmt --all -- --check` (root) → clean; desktop crate → one mechanical diff in `rgb_tests.rs`, fixed with `cargo fmt` (no manual edits), re-check clean.
- TDD receipts: new colocated tests first failed against real code (nonexistent `command_bytes` helper at compile time; `BufferTooSmall`-vs-`ProtocolViolation` assertion at runtime), then passed after correction — no test passed on first run without exercising the failure.
- Graph: gitnexus `impact` (upstream) run pre-edit for `classify_interface`, `group_monka_devices`, `send_feature_report`; `detect-changes --scope compare --base-ref 30fab11` precedes this commit (see review handoff metadata).

## Decisions Made

- Followed reviewer round-2 findings over the plan's letter where they conflict
  (see Deviations); reviewer direction is binding at implementation time.
- `cfg_attr(not(target_os = "windows"), allow(dead_code))` on
  `normalize_feature_frame`: the helper is Windows-production-only; on Mac/Linux
  builds it is exercised solely by unit tests. Documented inline.
- Windows-path test builders in `device.rs` gated `cfg(target_os = "windows")`
  for the same reason; legacy test gated `cfg(not(target_os = "windows"))`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Reviewer binding - scope] Removed the plan-requested policy-aware grouping seam**
- **Found during:** Task 1 (review round-2 finding D3)
- **Issue:** Plan asked for a private `group_monka_devices_with_policy`-style seam so Windows policy cases execute on Mac. Reviewer round 2 explicitly forbids test-only seams and indirection through them.
- **Fix:** Deleted the seam and all `*_with_policy` helpers; grouping tests drive the production `group_monka_devices` path and are `cfg(target_os = "windows")`, executing on native Windows CI. Pure-helper policy tests run on every host.
- **Files modified:** `crates/monkey-core/src/device.rs`, `crates/monkey-core/tests/device_discovery_test.rs`
- **Verification:** `cargo test -p monkey-core` green; Windows-only tests compile out on Mac by construction.
- **Impact on plan:** Windows grouping proof moves to the already-gated native-Windows evidence (task 3 / coverage D3+D6); mock-policy proof on Mac unchanged.

**Total deviations:** 1 (reviewer-directed, no scope creep)
**Impact on plan:** Must-have truths T1–T5 hold by Mac-executable tests; T6 (hardware pending) unchanged and explicitly tracked.

## Issues Encountered

- Round-2 test code initially referenced a nonexistent `command_bytes` helper and a wrong error variant (`ProtocolViolation` for empty input; implementation correctly returns `BufferTooSmall`). Both surfaced as real RED failures and were fixed in the tests, not the implementation.
- Desktop `rgb_tests.rs` on-disk content differed from the session-start snapshot (worker-mock e2e shape, not the remembered helper shape); rewrote assertions against the actual file and re-verified.

## No-Hardware Statement

No HID hardware was connected, enumerated, opened, or written to in this session. All evidence comes from `MockTransport` call records and pure-function unit tests, which bypass the OS HID stack entirely. Mock success does not prove Windows framing or collection selection on real hardware.

## Native Windows / Human Status

- **PENDING:** native Windows `cargo test/clippy/build` for root + desktop (no Windows runner in this session; task 3 gate).
- **PENDING:** human wired-USB RAM-preview smoke on a reviewed Windows build (task 3 gate). GH #18 stays open until then.
- **Revert strategy:** `git revert` of the round-2 rework commit; never a runtime protocol fallback.

## User Setup Required

None - no external service configuration required. (Windows wired-USB smoke is owned by plan task 3, not agent setup.)

## Next Phase Readiness

- Code + summary ready for independent reviewer (`reviewer` profile) via kanban review handoff.
- Blockers for integration: independent review approval, native Windows evidence, human smoke (all tracked above, none closable by this session).

---
*Phase: 08-windows-rgb-preview, plan 01 (review round 2)*
*Completed: 2026-10-18*
