# Phase 07 validation and operational handoff

## Inline planning review (operator waiver)
This is an orchestrator consistency check, NOT independent Planner/UI checker approval. The waiver authorizes inline synthesis, not hardware tests or self-review of implementation.

- Parity matrix accounts for OEM device/LCD/RGB/profile/diagnostic workflows, explicitly gates remap/macros, excludes destructive/unsupported firmware/reset/audio writes and generic mouse assets. Benchmark/completion UI is explicitly deferred to CLI guidance.
- UI spec locks four pages, minimal retro visual language, typography, palette, focus/error/recovery/demo behavior, unknown data and animation cancellation.
- IPC v1 is fixed before fan-out; native command takes typed actions not opcodes/paths. Rust owns serialization and all native files. Frontend owns all other desktop files. Root manifests/core/CLI are untouched. Each plan owns its SUMMARY. No shared files or parallel edits.
- Exact read-only gate commands and independently rerunnable Reviewer handoff appear in both PLANs. 07-02 production build validates actual Tauri wiring; browser mock success alone cannot satisfy acceptance.
- Native first is explicit --no-tracer decomposition; production request tracer is native mock list/probe, then complete device vertical UI before screen expansion. This trades a horizontal prerequisite for single-file ownership and native safety cohesion.
- Scoped file/decoded memory limits, deny tests, battery/capability gates and lifecycle/stop races are explicit. No frontend opt-in can bypass backend safety. Underlying core protocol/timing remains authority.
- No tests/builds/hardware performed in planning; no invented passing count. UI/OS/hardware certification is pending implementation + UAT.

## Dispatch prerequisites (must be solved before unblocking Builder)
1. Authorized operator commits only `.planning/` artifacts in main checkout and makes commit available to execution branches. Orchestrator has not committed or pushed; do not pretend uncommitted plans are present in fresh worktrees.
2. Base: AGENTS.md has no Base branch line, so declared default metadata.base_branch is `origin/main`. Read-only checks found main checkout `dev` at `7df22cb7677fb71fdf6ca08fecedd9dc06448b61`; origin/main `99fde4bc00ffa43f6b2209720d1afe281218b198`, ancestor missing phase 6. Operator must align approved base to include phase 6 (or explicitly authorize another base) before dispatch. Do not silently integrate phase 6 as part of UI PR.
3. Set native branch `wt/ui-v2-native`, frontend branch `wt/ui-v2-frontend`, isolated workspace paths as card records, metadata `{base_branch:'origin/main',gsd_phase:'07',gsd_plan:'07-01'|'07-02',plan_path:'.planning/phases/07-retro-pixel-desktop/07-0N-PLAN.md'}` after base alignment. Tool schema cannot set branch_name/metadata, so Builder cards are intentionally initially blocked. No CLI/DB workaround used. Frontend branch must include approved native parent head before work starts (branch from that head), retain declared integration base metadata.
4. Restore compatible GitNexus graph/engine under authorized maintenance; existing graph queries fail version 43 vs 42. Builders must obtain impact/check evidence before modifying existing symbols/committing; graph failure is not clearance. No new Scout research or repeated planner retries needed.
5. Unblock native card once setup complete. Frontend remains dependency-gated and its setup block must also be cleared through authorized surface. Native review and frontend review use reviewer profile; Integrate waits for both done.

## Integration and UAT gate
Orchestrator uses repository integration script if available; otherwise shared approved runner with both builder branches, --repo absolute main checkout, --create-pr --label ai-bot and verify commands joined with && (deduplicate native subset already included in final plan). Never merge into base. PR body under .planning/pr-body.md uses visual summary, actual verify logs, two-way UI/internal IPC rollback and high-risk native safety exposure. Block for human merge after Draft PR creation; merge conflict -> Builder arbiter card, failed verify -> dedicated fix card, not manual orchestrator code edits.

After human merge: advance GSD state/progress/UI2 requirements only on actual evidence; run gsd-verify-work 07 for consented hardware UAT. First read-only Device/probe/doctor; then explicit LCD image/GIF/sequence/pattern/start-stop and RGB RAM preview; separately consented healthy-power persistent save/readback, partial-interface and unplug/recovery. Record timing as measurements not guaranteed perf. Signed/app-sandbox certification remains deferred.

Harvest after Integrate merged: one Builder card parents=[Integrate], ce-compound, owned docs/solutions and any necessary convention updates, summary findings from cards with ready_for_ce_compound true; `/ce-compound mode:non-interactive depth:lightweight` (full for HIGH/critical), plus gsd-extract-learnings 07. Independent Reviewer; human knowledge merge, then QMD re-index through hook or qmd update && qmd embed. No solution docs created by orchestrator.
