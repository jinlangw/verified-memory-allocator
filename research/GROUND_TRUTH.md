# Ground Truth

- Current stage is `prove` in `research/PIPELINE_STATE.json`.
- `MISSION.md` states the acceptance target is retiring all non-`lib.rs` `todo();` call sites without adding project-owned trust, and `verus-mimalloc/lib.rs` C ABI shims are sanctioned TCB.
- `.verus_agent/goal.json` freezes four targets: `heap_init`, `heap_malloc`, `free`, and `global_init`.
- `make verify` at the project root exited 0 with `verification results:: 734 verified, 0 errors`.
- `grep -RIn "todo();" verus-mimalloc/*.rs | grep -v 'verus-mimalloc/lib.rs' | wc -l` reported `20`.
- `research/proof_attempts.jsonl` exists and is empty.
- `CHECKPOINT.md` does not exist at the project root.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.proof_state --crate-root . next` reported 192 reachable functions, 512 edges, 1 WIP trust node/1 trust leaf, 0 unentered nodes, 15 unknown nodes, top frontier 15, bottom frontier 14, and 16 unresolved reachable obligations.
- The reachable WIP trust node is `verus-mimalloc/types.rs::todo@L2189`.
- Direct reachable callers of `types.rs::todo` are: `alloc_generic.rs::malloc_generic@L21`, `os_alloc.rs::os_alloc_aligned_offset@L11`, `os_alloc.rs::os_mem_alloc_aligned@L135`, `page.rs::find_page@L27`, `segment.rs::segment_alloc@L927`, `segment.rs::segment_free@L1483`, `segment.rs::segment_os_alloc@L1328`, `segment.rs::segment_page_free@L1751`, `segment.rs::segment_span_free_coalesce@L1993`, and `segment.rs::segments_page_find_and_allocate@L186`.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.boundary_guard --crate-root . check` passed.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.operator_decisions --project-root . gate` passed with no unresolved decisions.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.completion_gate --crate-root .` exited 1 with `completion_gate: NOT DONE -- derived proof obligations remain`.
- Project-wide `spec_drift check` invocation for this repository layout is unresolved: attempts with module `verus-mimalloc` and `.` both failed with `pipeline_state.json not found`. Direct baseline `spec_drift git-diff` checks for `segment.rs`, `page.rs`, `os_alloc.rs`, `alloc_generic.rs`, `commit_mask.rs`, `layout.rs`, and `free.rs` exited 0 with no drifts.
- Project-wide `ast_consistency` invocation for this repository layout is unresolved: passing `verus-mimalloc` as the verified path failed because it is a directory. File-level `ast_consistency --base-ref 1ccf01387963f83a3defe0c8109037999ab4c843 ... count` is green for `page.rs`, `alloc_generic.rs`, `layout.rs`, and `commit_mask.rs`; `segment.rs` reports 2 known mismatches and `os_alloc.rs` reports 1 known mismatch from retired `todo();` executable branches.
