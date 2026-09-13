---
quick_id: 260913-j5i
status: complete
---

# Correct source-evidence confidence and reproducibility claims

Updated the three requested research documents:

- `.planning/research/SUMMARY.md`
- `.planning/research/ARCHITECTURE.md`
- `.planning/research/FEATURES.md`

Changes:

- Downgraded overall evidence from HIGH to MEDIUM.
- Kept HIGH confidence limited to directly inspectable OEM identity/layout metadata.
- Classified GMK-67 RGB opcodes as prior-art/inferred, not Monka captures.
- Classified Ajazz/Sonix LCD chunking, ST7789 behavior, and RTC `0x51` as cross-family hypotheses.
- Recorded that Q1–Q4 remain open and that no `.pcap`/`.pcapng` files exist.
- Added reproducibility boundaries for local/ignored source artifacts and raw-capture absence.
- Removed stale claims that target-board captures or verified opcodes already exist.

Verification completed:

- `git diff --check` passed.
- Confirmed `raw_capture_count=0` across the repository excluding `.git`.
- Confirmed all 7 cited local evidence paths exist in the working tree.
- Confirmed the documentation diff contains only the three requested research files.
