# Phase 07 execution DAG

Planning card: t_033482d4. Research parent: t_be53c466 (done). All cards created through native tools and read back once; builder setup holds are intentional, not implementation failures.

```text
t_be53c466 (Scout done)
  └─ t_033482d4 (Plan & decompose)
       ├─ t_2be6d649 (Builder 07-01 native; blocked setup)
       │    └─ t_4820821f (Builder 07-02 frontend; blocked setup + native parent)
       └───────────────┘
                 └─ t_0fda1bc5 (Orchestrator Integrate; todo, both builders as parents)
```

| Card | Owner | Runtime | Isolated workspace | Required branch (not yet configured) |
|---|---|---|---|---|
| t_2be6d649 | builder, review by reviewer | 3600s | .worktrees/ui-v2-native | wt/ui-v2-native |
| t_4820821f | builder, review by reviewer | 7200s (eight-task full UI/build/test plan) | .worktrees/ui-v2-frontend | wt/ui-v2-frontend |
| t_0fda1bc5 | orchestrator | 3600s | repo main checkout, integration only through approved runner | no orchestrator branch checkout |

Native owns apps/desktop/src-tauri/**, contracts/ipc-v1.json and 07-01-SUMMARY. Frontend owns remaining apps/desktop/** and 07-02-SUMMARY. No shared root manifests/core/CLI. Sequential dependency also ensures native contract review precedes UI. Each Builder body ends with Reviewer request and exact verify gates.

Metadata/branch field limitations are explicit in each setup hold; required base_branch/gsd fields are in bodies, NOT represented as already written machine metadata. Both cards' created events confirm branch_name=null. Neither is dispatch-safe until operator sets fields. Default base origin/main exists but lacks Phase 6 APIs; main checkout dev has Phase 6. See 07-VALIDATION.md for alignment, planning commit and graph repair prerequisites.

Next action for operator: configure the above records through an authorized board surface, publish planning commit onto approved base, unblock native. Prepare frontend on approved native head and clear its hold after native review. Integrate wakes automatically only when both builders are done. Do not ask orchestrator to implement, bypass guards, or replay blocked cards unchanged.

No implementation, tests, screenshot rendering, hardware write or push/merge was performed by this planning task. Completion means planning and DAG exist, not that UI v2.0 is shipped.
