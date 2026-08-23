# Ground Truth

- Current stage is `prove` in `research/PIPELINE_STATE.json`.
- `MISSION.md` states the acceptance target is retiring all non-`lib.rs` `todo();` call sites without adding project-owned trust, and `verus-mimalloc/lib.rs` C ABI shims are sanctioned TCB.
- `.verus_agent/goal.json` freezes four targets: `heap_init`, `heap_malloc`, `free`, and `global_init`.
- `make verify` at the project root exited 0 with `verification results:: 736 verified, 0 errors` on 2026-08-22 14:27.
- `grep -RIn "todo();" verus-mimalloc/*.rs | grep -v 'verus-mimalloc/lib.rs' | wc -l` reported `12` on 2026-08-22 14:27; this literal count includes commented legacy `todo();` text.
- `research/proof_attempts.jsonl` exists and latest observed attempt ids include `retire-segment-alloc-success-branch-20260822T1303`, `retire-segment-page-free-abandoned-20260822T1419-decommit-bound`, and `retire-segment-coalesce-abandoned-trigger-20260822T1407`.
- `CHECKPOINT.md` does not exist at the project root.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.proof_state --crate-root . next` reported 192 reachable functions, 508 edges, 1 WIP trust node/1 trust leaf, 0 unentered nodes, 15 unknown nodes, top frontier 15, bottom frontier 14, and 16 unresolved reachable obligations on 2026-08-22 14:27.
- The reachable WIP trust node is `verus-mimalloc/types.rs::todo@L2210`.
- Direct reachable callers of `types.rs::todo` are: `alloc_generic.rs::malloc_generic@L21`, `os_alloc.rs::os_mem_alloc_aligned@L136`, `page.rs::find_page@L27`, and `segment.rs::segment_free@L1491`.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.boundary_guard --crate-root . check` passed on 2026-08-22 14:28.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.operator_decisions --project-root . gate` passed with no unresolved decisions on 2026-08-22 14:28.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.completion_gate --crate-root .` exited 1 on 2026-08-22 14:28 with `completion_gate: NOT DONE -- derived proof obligations remain`; listed obligations include external/vstd unknowns plus the unsanctioned `verus-mimalloc/types.rs::todo@L2210` WIP trust leaf.
- Direct baseline `spec_drift git-diff --before 1ccf01387963f83a3defe0c8109037999ab4c843` checks for `page.rs`, `os_alloc.rs`, `alloc_generic.rs`, `segment.rs`, `commit_mask.rs`, `layout.rs`, and `free.rs` exited 0 with no drifts on 2026-08-22 14:29.
- File-level `ast_consistency --base-ref 1ccf01387963f83a3defe0c8109037999ab4c843 ... count` is green for `page.rs`, `alloc_generic.rs`, `commit_mask.rs`, `layout.rs`, and `free.rs`; `segment.rs` reports 8 mismatches and `os_alloc.rs` reports 2 mismatches. `ast_consistency summary` names the `os_alloc.rs` mismatches as `os_alloc_aligned_offset` and `unix_mmap`, and the `segment.rs` mismatches as `segment_alloc`, `segment_os_alloc`, `segment_page_alloc`, `segment_page_clear`, `segment_page_free`, `segment_span_free`, `segment_span_free_coalesce`, and `segments_page_find_and_allocate`.
