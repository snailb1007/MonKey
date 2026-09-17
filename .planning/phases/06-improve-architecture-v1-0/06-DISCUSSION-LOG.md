# Phase 6: Improve Architecture v1.0 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-16
**Phase:** 06-improve-architecture-v1-0
**Areas discussed:** Seam Architecture, Interface Variation Policy, Doctor Diagnostics & OpenError Hierarchy, SafetyRails Ownership & Deep Module API, Test Seam Harmonization & Env-Var Removal, Upstream Impact & Blast Radius

---

## Seam Architecture & MonkaDevice Structure

| Option | Description | Selected |
|--------|-------------|----------|
| MonkaDevice as Trait + MockDevice | Create a new mock adapter at the MonkaDevice level (two parallel mock seams) | |
| Single Seam on Transport + Concrete MonkaDevice | Keep Transport as the only seam. MonkaDevice is a concrete struct wrapping Box<dyn Transport> via from_transport() | ✓ |

**User's choice:** Keep Transport as the single seam; MonkaDevice is a concrete struct.
**Notes:** "in-memory test adapter" already exists — `MockTransport` (transport/mock.rs) implements `Transport`, has `assert_no_writes()`, call recording, and error injection. Adding a second fake at `MonkaDevice` violates the principle "two adapters means a real seam" — `Transport` is the real seam; do not duplicate it.

---

## Interface Variation vs Duplication

| Option | Description | Selected |
|--------|-------------|----------|
| Flatten Interface Selection | Unify all commands into a single hardcoded open logic | |
| InterfacePolicy Parameter | Preserve legitimate hardware variations via InterfacePolicy enum (PreferB, RequireA, RequireB, BulkFirst, ControlFirst, Any) | ✓ |

**User's choice:** Retain variations via `InterfacePolicy` enum.
**Notes:** Interface-selection is REAL variation, not accidental duplication:
- `probe` → B only, fallback to descriptor-only when open fails
- `rgb` → B, fallback A
- `lcd` → A only, hard fail
- `bench` → A-then-B or B-then-A depending on `bench_type.runs_bulk()`
- `doctor` → both, reports independently
Blindly merging would break hardware policy.

---

## Doctor Diagnostics & Layered OpenError

| Option | Description | Selected |
|--------|-------------|----------|
| Single Result<MonkaDevice, E> | MonkaDevice::open() fails fast on first error (unsuitable for doctor) | |
| Layered OpenError + MonkaDevice::diagnose() | OpenError stratified by failure stage; separate diagnose() method for non-early-returning doctor checks | ✓ |

**User's choice:** Layered `OpenError` + dedicated `MonkaDevice::diagnose()`.
**Notes:** Consumer #6 (`doctor.rs:63-199`) is the toughest case: it cannot early-return because it must report Pass/Fail/Warn + actionable remediations for each step independently. `OpenError` stages: `HidInit`, `NoDevice`, `InterfaceUnavailable(role)`, `InterfaceOpenFailed(role, err)`.

---

## SafetyRails Ownership & Deep Module Operations

| Option | Description | Selected |
|--------|-------------|----------|
| Caller Threads SafetyRails | Expose device.transport_mut() and device.safety() accessors | |
| MonkaDevice Owns SafetyRails + Deep Operations | MonkaDevice encapsulates SafetyRails and exposes high-level domain operations | ✓ |

**User's choice:** `MonkaDevice` owns `SafetyRails`; expose operations on `MonkaDevice`.
**Notes:** In Rust, `transport` requires `&mut` while `safety` requires `&`. Exposing both accessors simultaneously triggers borrow checker conflicts. Encapsulating domain operations (`device.stream_frame(...)`, `device.apply_rgb_preview(...)`, `device.apply_rgb_commit(...)`) turns `MonkaDevice` into a true deep module, though it requires adjusting signatures in `LcdStreamer` and `RgbManager`.

---

## Seam Leakage: Transport Trait & Safety Authorization

| Option | Description | Selected |
|--------|-------------|----------|
| Option A: Strip Param & Enforce in Caller | Strip &SafetyRails from Transport, validate only in TransactionManager | |
| Option B: Pure Transport Trait + SafeTransport Guard Façade | Make Transport pure byte I/O. Add SafeTransport<'a> guard façade holding (&mut Transport, &SafetyRails) consumed by TransactionManager and LcdStreamer | ✓ |

**User's choice:** Option B (pure trait + guard façade).
**Notes:**
- `Transport` trait leaked domain knowledge (`&SafetyRails`) just to check a single boolean (`validate_hardware_write_permitted()`).
- Stripping the parameter without a guard façade creates a severe vulnerability: `LcdStreamer` writes directly to `Transport` and does not go through `TransactionManager`. If enforcement is only in `TransactionManager`, `LcdStreamer` or future callers bypass authorization silently.
- Option B makes `Transport` pure I/O (`pub(crate)`), eliminating `SafetyRails` from adapter implementations and mock tests.
- `SafeTransport<'a>` centralizes write authorization in one place and enforces the safety invariant at compile time for both `TransactionManager` and `LcdStreamer`.

---

## Test Seams & MONKEY_SIMULATE_EMPTY Removal

| Option | Description | Selected |
|--------|-------------|----------|
| Keep Ad-hoc Test Seams & Env Vars | Maintain run_probe_with_transport, run_bench_with_transport, MONKEY_SIMULATE_EMPTY | |
| Eliminate Seams & Env Vars | Standardize testing on MonkaDevice::from_transport(MockTransport) and remove MONKEY_SIMULATE_EMPTY | ✓ |

**User's choice:** Eliminate all 5 ad-hoc test seams and remove `MONKEY_SIMULATE_EMPTY` from `probe.rs` and `info.rs`.
**Notes:** Production code should never inspect environment variables like `MONKEY_SIMULATE_EMPTY` to fake hardware states. Consolidating the test path through `MonkaDevice::from_transport` eliminates this technical debt.

---

## Blast Radius & GitNexus Pre-Flight

| Option | Description | Selected |
|--------|-------------|----------|
| Proceed without Index Rebuild | Rely on existing GitNexus index (which was 15 commits stale) | |
| Rebuild Index & Acknowledge CRITICAL Risk | Run `node .gitnexus/run.cjs analyze --index-only`, verify CRITICAL blast radius | ✓ |

**User's choice:** Index rebuilt and blast radius analyzed.
**Notes:** Upstream impact of `open_device_path` is CRITICAL (18 affected symbols, 12 processes, 5 direct callers across CLI and core). Planning must include atomic verification and regression safety across all 122 workspace tests.

---

## the agent's Discretion

- Choice of exact file structure (`crates/monkey-core/src/device/` vs `crates/monkey-core/src/device.rs`).
- Exact naming and variant order for `OpenError` and `InterfacePolicy`.

## Deferred Ideas

None — discussion stayed within Phase 6 scope.
