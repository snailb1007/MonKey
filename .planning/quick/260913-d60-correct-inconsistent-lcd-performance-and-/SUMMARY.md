---
quick_id: 260913-d60
status: complete
---

# Correct inconsistent LCD performance and timing targets

Reconciled mathematical and physical contradictions regarding USB Full-Speed bus utilization, inter-packet pacing latency, and OS timing guarantees across design and planning documents.

## Key Changes Made

1. **USB Full-Speed Bus Utilization (.planning/research/ARCHITECTURE.md)**:
   - Corrected inaccurate statement claiming `<5%` bus utilization at 15 FPS.
   - Updated with exact calculations: 32,768 bytes/frame × 15 FPS = 3.932 Mbps (~32.8% of raw 12 Mbps link, ~40%–46% effective bandwidth under USB framing and overhead).
   - Documented explicit requirement for pacing and frame-drop backpressure to protect keyboard typing latency.

2. **Inter-chunk Pacing & Latency Disaggregation (.planning/research/ARCHITECTURE.md, PITFALLS.md)**:
   - Reconciled inter-chunk pacing numbers to realistic bounds: 3ms–8ms (bus transfer ~40ms–75ms) and 5ms–10ms safety pacing (~65ms–100ms transfer) to prevent MCU SPI DMA FIFO overflows.
   - Separated Host Processing latency (<15ms for decode + dither) from Transport transfer latency (35ms–75ms) and panel refresh (~10ms).
   - Updated static LCD display target from unrealistic `<50ms` to `<100ms` total latency (visually instantaneous for static images).

3. **Millisecond Pacing vs Microsecond Claims (.planning/research/STACK.md, SUMMARY.md, SKELETON.md, PITFALLS.md)**:
   - Replaced claims of "microsecond-accurate timing" with deterministic "millisecond-level pacing" (1ms–10ms), acknowledging USB Full-Speed 1ms SOF frames and non-real-time OS scheduling jitter.

4. **Planning Alignment (.planning/PROJECT.md, REQUIREMENTS.md, ROADMAP.md, FEATURES.md)**:
   - Synchronized static LCD display target across all specification and planning files to `<100ms total latency (<15ms host + <75ms transport)`.

## Verification

- Ran `cargo test` across all workspace crates (`monkey_core`, `monkey_cli`, unit tests, integration tests); all 45 tests pass with 0 failures.
