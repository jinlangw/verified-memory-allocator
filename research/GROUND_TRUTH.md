# Ground Truth

- Current stage is `prove` in `research/PIPELINE_STATE.json`.
- `MISSION.md` states the acceptance target is retiring all non-`lib.rs` `todo();` call sites without adding project-owned trust, and `verus-mimalloc/lib.rs` C ABI shims are sanctioned TCB.
- `.verus_agent/goal.json` freezes four targets: `heap_init`, `heap_malloc`, `free`, and `global_init`.
- `make verify` at the project root exited 0 with `verification results:: 736 verified, 0 errors`.
- `grep -R "todo();" verus-mimalloc/*.rs | grep -v 'verus-mimalloc/lib.rs' | wc -l` reported `27`.
- `research/proof_attempts.jsonl` exists and is empty.
- `CHECKPOINT.md` does not exist at the project root.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.proof_state --crate-root . next` reported 192 reachable functions, 516 edges, 1 WIP trust node, 0 unentered nodes, 15 unknown nodes, and 16 unresolved reachable obligations.
- The reachable WIP trust node is `verus-mimalloc/types.rs::todo@L2189`.
- Direct reachable callers of `types.rs::todo` are: `alloc_generic.rs::malloc_generic@L21`, `commit_segment.rs::segment_perhaps_decommit@L349`, `os_alloc.rs::os_alloc_aligned_offset@L11`, `os_alloc.rs::os_mem_alloc_aligned@L135`, `os_alloc.rs::unix_mmap@L220`, `page.rs::find_page@L27`, `segment.rs::segment_alloc@L931`, `segment.rs::segment_free@L1487`, `segment.rs::segment_os_alloc@L1332`, `segment.rs::segment_page_alloc@L36`, `segment.rs::segment_page_clear@L1780`, `segment.rs::segment_page_free@L1755`, `segment.rs::segment_span_free_coalesce@L1999`, and `segment.rs::segments_page_find_and_allocate@L190`.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.boundary_guard --crate-root . check` passed.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.operator_decisions --project-root . gate` passed with no unresolved decisions.
- `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.completion_gate --crate-root .` exited 1 with `completion_gate: NOT DONE -- derived proof obligations remain`.
- Project-wide `spec_drift check` invocation for this repository layout is unresolved: attempts with module `verus-mimalloc` and `.` both failed with `pipeline_state.json not found`. A direct baseline invocation, `$ARGUS_SKILL_PYTHON -m argus_skill.verticals.verus.tools.spec_drift git-diff verus-mimalloc/lib.rs --before 1ccf01387963f83a3defe0c8109037999ab4c843 --after HEAD --json`, exited 0 with no drifts for that source path.
- Project-wide `ast_consistency` invocation for this repository layout is unresolved: passing `verus-mimalloc` as the verified path failed because it is a directory.
