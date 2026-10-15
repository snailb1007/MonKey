# Phase 07 context

Goal: minimal retro pixel desktop UI v2.0 based on a complete classification of legacy OEM workflows, exposing only verified MonKey v1 core functionality.

Locked direction and IPC: 07-UI-SPEC.md. Research and citations: 07-RESEARCH.md. User supplied no screenshot approval or specific accent preference; palette/layout are explicit planner decisions, not claimed user approvals.

Planning entry: gsd-plan-phase 07 --skip-research --no-tracer, interpreted inline under operator waiver recorded on t_033482d4. External gsd-tools, named subagent planner/checker and UI gates are waived for this sandbox run only. Inline consistency review is not represented as independent review. Native IPC safety work and UI still require cross-provider Reviewer.

Decomposition choice: two bounded ownership domains, native safety adapter and complete frontend. The native prerequisite is horizontal by deliberate --no-tracer decision: no desktop scaffolding exists, and a single native ownership domain prevents multiple workers modifying Tauri registration/security files. First native test is a production request path for list/probe with mocked device I/O, not an empty scaffold. Frontend follows after native approval and includes the end-to-end desktop/device tracer before screen expansion. No parallel builders, no shared mutable manifests.

Milestone handling: v1 completed history stays intact. Phase 07 is UI v2.0 foundation scope, not a declaration that all future v2 daemon/remap/sandbox goals are delivered. Preserve old metrics; track phase 07 separately until merge/UAT.

Operational constraint: native kanban_create does not expose branch_name or metadata. Do not use CLI mutations/direct DB writes to work around that restriction. Create execution cards blocked pending operator configuration of wt/* branch + base_branch/gsd metadata and commit of main-checkout .planning so worktrees can see these plans. Integrate is created upfront with builder dependencies, not as a later promise. All execution cards get isolated worktree paths and retry-safe creation keys.
