---
quick_id: 260913-j5i
status: in_progress
---

# Correct source-evidence confidence and reproducibility claims

## Scope

Update only the three cited research documents:
- `.planning/research/SUMMARY.md`
- `.planning/research/ARCHITECTURE.md`
- `.planning/research/FEATURES.md`

## Required changes

1. Separate repository artifacts from physical USB captures; state that no `.pcap`/`.pcapng` evidence is present unless verified.
2. Keep OEM/XML facts as verified repository evidence: VID/PID, `RKGK890`, and the 81-key layout.
3. Downgrade protocol claims that come from prior art or unrelated Ajazz/Sonix hardware to `MEDIUM`/`INFERRED` and label them as unverified on Monka 3075 Pro.
4. Mark Q1–Q4 from `research/capture_plan.md` as open, and avoid calling LCD chunking, ST7789, RTC `0x51`, or RGB opcodes verified on Monka hardware.
5. Add source paths and evidence status so another reviewer can reproduce the repository-side checks.

## Verification

- Search the three documents for forbidden unsupported `HIGH` source/protocol claims and stale wording.
- Confirm the cited source files exist in the repository and no capture files are present.
- Run a documentation-only diff/status check; no implementation files should change.
