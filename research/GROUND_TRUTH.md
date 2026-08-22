# Ground Truth

- Current stage is `prove` in `research/PIPELINE_STATE.json`.
- `MISSION.md` states the acceptance target is retiring all non-`lib.rs` `todo();` call sites without adding project-owned trust, and `verus-mimalloc/lib.rs` C ABI shims are sanctioned TCB.
- `.verus_agent/goal.json` freezes four targets: `heap_init`, `heap_malloc`, `free`, and `global_init`.
- `make verify` at the project root exited 0 with `verification results:: 734 verified, 0 errors` on 2026-08-22 13:44.
- `grep -RIn "todo();" verus-mimalloc/*.rs | grep -v 'verus-mimalloc/lib.rs' | wc -l` reported `16` on 2026-08-22 13:44; this literal count includes commented legacy `todo();` text.
- `research/proof_attempts.jsonl` exists and contains one recorded failed attempt: `retire-segment-alloc-success-branch-20260822T1303`.
- `CHECKPOINT.md` does not exist at the project root.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.proof_state --crate-root . next` reported 192 reachable functions, 511 edges, 1 WIP trust node/1 trust leaf, 0 unentered nodes, 15 unknown nodes, top frontier 15, bottom frontier 14, and 16 unresolved reachable obligations on 2026-08-22 13:44.
- The reachable WIP trust node is `verus-mimalloc/types.rs::todo@L2189`.
- Direct reachable callers of `types.rs::todo` are: `alloc_generic.rs::malloc_generic@L21`, `os_alloc.rs::os_alloc_aligned_offset@L11`, `os_alloc.rs::os_mem_alloc_aligned@L135`, `page.rs::find_page@L27`, `segment.rs::segment_free@L1491`, `segment.rs::segment_page_free@L1759`, and `segment.rs::segment_span_free_coalesce@L2001`.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.boundary_guard --crate-root . check` passed on 2026-08-22 13:44.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.operator_decisions --project-root . gate` passed with no unresolved decisions on 2026-08-22 13:44.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.completion_gate --crate-root .` exited 1 on 2026-08-22 13:44 with `completion_gate: NOT DONE -- derived proof obligations remain`; listed obligations include external/vstd unknowns plus the unsanctioned `verus-mimalloc/types.rs::todo@L2189` WIP trust leaf.
- Direct baseline `spec_drift git-diff --before 1ccf01387963f83a3defe0c8109037999ab4c843` checks for `page.rs`, `os_alloc.rs`, `alloc_generic.rs`, `segment.rs`, `commit_mask.rs`, `layout.rs`, and `free.rs` exited 0 with no drifts on 2026-08-22 13:44.
- File-level `ast_consistency --base-ref 1ccf01387963f83a3defe0c8109037999ab4c843 ... count` is green for `page.rs`, `alloc_generic.rs`, `commit_mask.rs`, and `layout.rs`; `segment.rs` reports 5 known mismatches and `os_alloc.rs` reports 1 known mismatch from retired `todo();` executable branches.
