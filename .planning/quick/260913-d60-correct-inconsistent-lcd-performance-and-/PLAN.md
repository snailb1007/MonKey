---
quick_id: 260913-d60
status: in_progress
---

# Correct inconsistent LCD performance targets and timing claims

## Scope

Reconcile mathematically and physically inconsistent LCD performance targets and microsecond timing claims across planning and research documentation:
- `.planning/research/ARCHITECTURE.md`
- `.planning/research/PITFALLS.md`
- `.planning/research/SUMMARY.md`
- `.planning/research/STACK.md`
- `.planning/REQUIREMENTS.md`
- `.planning/ROADMAP.md`
- `.planning/PROJECT.md`
- `AGENTS.md`

## Required changes

1. Correct USB bus load calculation for 15 FPS:
   - Replace "below 5%" claims with realistic numbers: ~32.8% theoretical raw wire utilization (~3.93 Mbps on 12 Mbps link), or ~40-46% effective USB Full-Speed bandwidth considering protocol framing and overhead.
   - Explain why inter-chunk pacing and frame-drop backpressure are necessary to avoid degrading keyboard input latency.
2. Decouple and realistic latency metrics:
   - Host processing (decode + Floyd-Steinberg dither): Target <15ms.
   - Host-to-device transport: 35ms-75ms depending on empirical inter-chunk pacing (3ms-8ms).
   - Total static frame delivery target: Update from `<50ms` to `<100ms` (visually instantaneous for user, safely within physical/pacing constraints).
   - Animation target: 10-15 FPS (66.6ms-100ms frame interval).
3. Correct timing resolution terminology:
   - Replace "microsecond-accurate timing" with "deterministic millisecond-level pacing" (1ms-10ms), acknowledging macOS non-realtime scheduling jitter and USB Full-Speed 1ms frame boundaries.

## Verification

- Search for stale `<50ms` or `under 50ms` targets for static LCD transfer and update them to `<100ms`.
- Search for incorrect "below 5%" USB utilization claims.
- Search for "microsecond-accurate" timing references and replace with millisecond pacing.
- Ensure all planning documents are internally consistent.
