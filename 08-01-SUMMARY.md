# Phase Summary — 08-01 Windows RGB Preview Routing (T1+T2+M)

## Metadata

- Plan: `.planning/phases/08-windows-rgb-preview/08-01-PLAN.md` (Tasks 1, 2, M upload gate)
- Phase: 08-windows-rgb-preview
- Date: 2026-10-18
- Status: complete
- Author: builder (kanban task t_22c00c4c)
- Branch: wt/t_22c00c4c (worktree `/Volumes/D/personal_project/MonkaKeyboard/.worktrees/t_22c00c4c`)

## Objective

Give the Windows desktop RGB preview a safe, verified route to Interface B:
shared 64/65-byte HID feature-frame normalization, Windows composite-interface
quirk pairing with parent-prefix emission, and desktop assertions proving the
preview bytes equal the codec output and normalize to the hidapi wire shape.

## Key Questions

1. How does a raw64 MonKa payload become a valid hidapi feature frame? → Via
   `HidTransport::normalize_feature_frame` (shared helper, tested on all targets).
2. How are Windows `mi_00`/`mi_01` sibling interfaces paired without DevSrvsID? →
   Via `QuirkId::WindowsDevsrvsId` parent-prefix matching gated on known HID signatures.
3. Does the preview path ever touch the wrong pipe? → No: `apply_rgb_preview`
   fails fast with `TransportError::DeviceNotFound` when no Interface B is bound.
4. How is Interface B candidacy decided on Windows? → Via the private strict
   helper `windows_interface_role` (exact `(0xFF68,0x0061)`→A, exact
   `(0xFFFF,0x0001)`→B, everything else rejected); production
   `classify_interface`/`group_monka_devices` select it with
   `cfg!(target_os = "windows")`, and colocated tests drive the same helpers
   with the strict flag forced on so they run on any host (review round 1).

## What Was Delivered

| Task | Change | Location |
|------|--------|----------|
| T1 framing | `normalize_feature_frame` (raw64→65-frame, prefixed65 passthrough, rejects zero/oversized/nonzero-prefix/wrong-magic); wired into `HidTransport::send_feature_report`; macOS control-pipe branch forwards the normalized tail | `crates/monkey-core/src/transport/hid.rs` |
| T1 internality | `SAFE_PIPE: u8 = 0` marker referenced at every `send_feature_report` site + colocated unit test | `crates/monkey-core/src/transport/hid.rs` |
| T1 error | Guard reuses the existing `TransportError::DeviceNotFound` variant (review round 1: `error.rs` reverted to base, no longer in diff) | `crates/monkey-core/src/device.rs` (guard only) |
| T2 signatures | `monka_hid_signatures()` (`(0xFF68,0x0061)`, `(0xFFFF,0x0001)`, `(0x000C,0x0001)`), aliases via one constant | `crates/monkey-core/src/device.rs` |
| T2 Windows policy | Private `windows_interface_role` + `classify_with_policy` + `role_with_policy` + `group_monka_devices_with_policy` seam; `classify_interface`/`group_monka_devices` delegate with `cfg!(target_os = "windows")`; legacy Mac/Linux branch byte-identical | `crates/monkey-core/src/device.rs` |
| T2 quirk | `QuirkId::WindowsDevsrvsId`, `windows_device_parent`, `quirk_for_pair` (gated on signatures); pairing rule in `matches_set` before the ambiguity-gated fallback; `windows_devsrvs_id` emission in `group_monka_devices` | `crates/monkey-core/src/device.rs` |
| T2 roles | `RoleExt::policy`/`is_bulk_pipe`, `DualRole::role_a`/`role_b`/`has_complete_pair`, `InterfacePolicy` tests | `crates/monkey-core/src/device.rs` |
| T2 guard | No-interface guard in `MonkaDevice::apply_rgb_preview` (role-A or A-only set → `DeviceNotFound` before any I/O) | `crates/monkey-core/src/device.rs` |
| M desktop | Preview bytes == `encode_rgb_control_packet` output; normalize bridge asserts (prefix/magic/command/color/markers); `RequireB`-on-empty-set; production `group_monka_devices` emits `windows_devsrvs_id` | `apps/desktop/src-tauri/src/rgb_tests.rs` |

No `worker.rs` production change was needed: the existing preview dispatch path
already routes through `MonkaDevice::apply_rgb_preview`, so the new assertions
cover the routing without touching it.

## Tests

RED-first per slice; every new test was observed failing (compile error or
assertion) before its implementation. Review round 1 added 6 colocated
`windows_policy_tests` (compile-error RED against missing helpers, then GREEN)
and converted 4 classifier/grouping integration tests to host-conditional
(`cfg!(target_os = "windows")` branches type-check on Mac, execute the strict
branch on native Windows CI).

| Suite | Command (from worktree root unless noted) | Result |
|-------|-------------------------------------------|--------|
| device_discovery | `RTK_DISABLED=1 cargo test -p monkey-core --test device_discovery_test` | 29/29 pass |
| hid_transport | `RTK_DISABLED=1 cargo test -p monkey-core --test hid_transport_test` | 24/24 pass |
| core lib (incl. 6 new `windows_policy_tests`) | `RTK_DISABLED=1 cargo test -p monkey-core --lib` | 8/8 pass |
| core full | `RTK_DISABLED=1 cargo test -p monkey-core` | all targets green, 0 failed |
| workspace | `RTK_DISABLED=1 cargo test --workspace` | all suites green, 0 failed |
| desktop lib | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` | 40/40 pass (10 rgb_tests) |
| fmt | `cargo fmt --all -- --check` (root + desktop) | clean |
| clippy | `cargo clippy --all-targets -- -D warnings` (root) + `--manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets` | no issues |
| build | `cargo build` (root) | ok |

## Files Changed

- `crates/monkey-core/src/device.rs` (+signatures, quirk, emission, roles, guard;
  review round 1: +private Windows policy helpers/seam, +6 colocated tests)
- `crates/monkey-core/src/transport/hid.rs` (+normalizer, `SAFE_PIPE`, rewired send path, unit test)
- `crates/monkey-core/tests/device_discovery_test.rs` (+16 tests; review round 1:
  4 classifier/grouping tests host-conditional, 5 grouping fixtures vendor-B)
- `crates/monkey-core/tests/hid_transport_test.rs` (+11 tests; review round 1:
  guard assertion → `DeviceNotFound`)
- `apps/desktop/src-tauri/src/rgb_tests.rs` (+3 tests)
- (`crates/monkey-core/src/error.rs` reverted to base in review round 1 — no longer in diff.)

## Decisions

1. Nonzero 65-byte prefixes are rejected, never reinterpreted as magic04 (GH #18).
2. macOS control-pipe branch now forwards the normalized frame's tail — byte-identical
   for valid inputs, earlier rejection for invalid ones.
3. The quirk rule sits before (not inside) the ambiguity-gated fallback: distinct
   Windows instance segments never share a parent, so it is unambiguous by construction.
4. `quirk_for_pair` requires both sides to carry known HID signatures, so `(0,0)`
   zero-usage shapes can never pair this way.
5. Role-A devices (LCD streaming opens) fail preview fast instead of sending a
   feature report down the wrong pipe.
6. (Review round 1) Windows B candidacy is a private strict helper, selected by
   `cfg!(target_os = "windows")` at the two existing public entry points — no new
   public API solely for tests; the private seam is driven directly by colocated
   tests that run on every host.
7. (Review round 1) The no-interface guard reuses `TransportError::DeviceNotFound`
   instead of a new variant, returning `error.rs` to base and resolving the
   out-of-scope touch without needing retro-approval.

## Deviations

1. No `worker.rs` production change: the card's "worker routing" requirement is met
   by the existing dispatch path plus new assertions (implementation detail, same behavior).
2. `crates/monkey-core/src/error.rs` was sibling-owned in the first pass (added the
   `InputReportUnknown` variant the guard returned; no other edits). Review
   round 1 reverted it to base and reused `TransportError::DeviceNotFound`, so
   the file is no longer in the diff.
3. Codec command byte is `0x13` (19, `CommandId::RgbControl`), not decimal 13 —
   the failing test caught this; assertion now references the codec constant.
4. M gate `cargo test -p monkey-desktop` does not resolve from the workspace root
   (desktop is not a workspace member); ran `cargo test rgb_tests` / `cargo test`
   from `apps/desktop/src-tauri` instead, same coverage.
5. `kanban_detect_hotspots` is not available in this worker's toolset; hotspot risk
   is covered by the gitnexus compare-scope graph pass below.
6. This SUMMARY is written at the worktree root (`08-01-SUMMARY.md`), not next to
   the plan: `.planning/` is untracked in the main checkout and absent from the
   worktree, and writes outside the worktree are forbidden. Relocate on merge.
7. GitNexus compare-scope pass rates the diff CRITICAL (33 changed symbols, 23
   affected processes) in the first pass; the round-1 targeted pass rates
   `classify_interface` HIGH and `group_monka_devices` CRITICAL. The flagged
   processes are exactly this card's declared scope; the rating is hunk-proximity
   to core symbols (`open`, `group_monka_devices`, `send_feature_report`).
   Mitigation: full root + desktop suites green, clippy clean, new pairing and
   policy rules are additive (all pre-existing sibling tests pass unchanged or
   with host-conditional expectations). Recorded as residual risk for the
   reviewer, not waived.

## Residual Risks

- Review round 1 graph check: `classify_interface` impact HIGH (callers:
  `DiscoveredDevice::role`; processes: diagnose/discover/open), `group_monka_devices`
  impact CRITICAL (10 processes incl. CLI rgb/lcd/probe/bench). Both edits are
  strictly additive (new private helpers + delegation; legacy branch byte-identical;
  `send_feature_report` untouched this round) and every affected caller is covered
  by the green suites above — recorded for the reviewer, not waived.
- Windows behavior is mock-path verified; no physical `mi_00`/`mi_01` hardware was
  present. Real-hardware confirmation remains a Phase 8 exit item. The strict
  branch of the host-conditional integration tests executes only on native
  Windows CI; the same policy logic is proven on Mac/Linux by the colocated tests.
- `.planning/` SUMMARY relocation needed at merge time (see deviation 6).

## Next Actions

1. Reviewer: review round 1 rework is ready — strict Windows policy helper +
   private grouping seam + `error.rs` revert, all suites green.
2. Relocate `08-01-SUMMARY.md` next to `08-01-PLAN.md` on merge.
3. Phase 8 exit: confirm on physical Windows hardware (`mi_00`/`mi_01` pair + preview).
