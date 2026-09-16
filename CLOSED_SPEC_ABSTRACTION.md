# Manual closed-spec abstraction

## Scope and outcome

- Checkout: `strip-general-9f99d4c`, baseline commit
  `41b1aebf5ad296b3943a3b77854689a6e4c99cc0` (initial working tree clean).
- This allocator repair is separate from the accompanying Argus vertical
  guidance update. Archived run sources and reports were not modified.
- Manual source patches abstract **117 existing closed definitions**:
  **82 Boolean bodies to `true`**, **35 non-Boolean bodies to `uninterp`**.
- Additionally, one previously macro-generated closed Boolean predicate,
  `InvariantPredicate_auto_ThreadLLSimple_atomic::atomic_inv`, is now explicit
  source with a `true` body. It is accounted separately, not counted as a newly
  discovered original source declaration.
- **No active concrete closed bodies remain** in the reviewed project source.
  Existing open specs, executable function declarations, contracts, resource
  operations, and source trust declarations are preserved.
- The required allocator build succeeds. Full-project verification is
  diagnostic and **fails**; this is not a verified allocator or a complete
  proof-removal claim.

## Exact source changes

Counts below refer to existing source declarations, including declarations in
macros and implicit closed specs. The extra materialized predicate is separate.

| Source | Closed bool changed | Closed nonbool changed |
| --- | ---: | ---: |
| [verus-mimalloc/commit_mask.rs](verus-mimalloc/commit_mask.rs) | 0 | 4 |
| [verus-mimalloc/dealloc_token.rs](verus-mimalloc/dealloc_token.rs) | 1 | 4 |
| [verus-mimalloc/flags.rs](verus-mimalloc/flags.rs) | 6 | 1 |
| [verus-mimalloc/layout.rs](verus-mimalloc/layout.rs) | 0 | 3 |
| [verus-mimalloc/linked_list.rs](verus-mimalloc/linked_list.rs) | 5 | 7 |
| [verus-mimalloc/os_mem_util.rs](verus-mimalloc/os_mem_util.rs) | 0 | 4 |
| [verus-mimalloc/page_organization.rs](verus-mimalloc/page_organization.rs) | 34 | 11 |
| [verus-mimalloc/queues.rs](verus-mimalloc/queues.rs) | 2 | 0 |
| [verus-mimalloc/tokens.rs](verus-mimalloc/tokens.rs) | 33 | 1 |
| [verus-mimalloc/types.rs](verus-mimalloc/types.rs) | 1 | 0 |
| **Total** | **82** | **35** |

The linked-list change also materializes and abstracts the one generated
`atomic_inv` Boolean predicate noted above.

### Non-Boolean declarations (35)

- `commit_mask`: `mod64`, `div64`, `set_8_64`, `CommitMask::view`.
- `MimDealloc`: `block_id`, `ptr`, `inst`, `size`.
- `flags`: `flags2_retire_expire`.
- `layout`: `segment_start`, `start_offset`, `block_start`.
- `LL`: `next_ptr`, `len`, `page_id`, `block_size`, `instance`, `heap_id`, `ptr`.
- `Local`: `segment_page_range`, `segment_pages_range_total`,
  `segment_page_used`, `segment_pages_used_total`.
- `PageOrg::State`: `popped_len`, `get_list_idx`, `page_id_of_popped`,
  `popped_page_id`, `ec_of_popped`, `popped_ec`, `ucount`, `ucount_sum`,
  `one_count`, `insert_front`, `insert_back`.
- `Mim::State`: `mk_fresh_segment_id`.

### Boolean declarations (82)

- `MimDealloc::wf` (private `type_invariant`).
- `flags`: `flags0_is_reset`, `flags0_is_committed`, `flags0_is_zero_init`,
  `flags1_in_full`, `flags1_has_aligned`, `flags2_is_zero`.
- `LL`: `valid_node`, `wf`, `fixed_page`; `ThreadLLSimple::wf`;
  `StuffAgree::State::inv_eq`.
- `PageOrg::State`: `ll_basics`, `page_id_domain`, `count_off0`, `end_is_unused`,
  `count_is_right`, `popped_basics`, `data_for_used_header`,
  `inv_segment_creating`, `inv_very_unready`, `inv_ready`, `inv_used`,
  `data_for_unused_header`, `ll_inv_valid_unused`, `ll_inv_valid_used`,
  `ll_inv_valid_unused2`, `ll_inv_valid_used2`, `ll_inv_exists_in_some_list`,
  `attached_ranges`, `attached_ranges_segment`, `seg_free_prefix`,
  `attached_rec0`, `attached_rec`, `popped_ranges_match`,
  `popped_ranges_match_for_sid`, `popped_for_seg`, `is_any_the_popped`,
  `is_the_popped`, `expect_out_of_lists`, `good_range_very_unready`,
  `good_range0`, `good_range_unused`, `good_range_used`, `does_count`,
  `if_popped_or_other_then_for`.
- `queues`: `local_direct_update`, `pfd_direct_update`.
- `Mim::State`: `inv_reserved`, `inv_reserved2`, `inv_right_to_set_inst`,
  `inv_heap_of_page_delay`, `inv_delay_state`, `inv_delay_state_for_page`,
  `inv_delay_actor`, `inv_delay_actor_for_page`, `inv_delay_actor_sub`,
  `inv_checked_threads`, `inv_no_delay_actor_for_checked`,
  `right_to_use_thread_complement`, `heap_of_thread_is_valid`,
  `wf_heap_shared_access_requires_inst`, `wf_heap_shared_access`,
  `inv_thread_of_segment1`, `inv_thread_of_segment2`,
  `inv_thread_has_segment_for_page`, `inv_thread_of_page1`, `inv_thread_of_page2`,
  `heap_of_page_is_correct`, `inv_page_shared_access_dom`,
  `inv_page_shared_access_eq`, `inv_segment_shared_access_dom`,
  `inv_segment_shared_access_eq`, `inv_block_id_valid`,
  `inv_block_id_valid_for_block`, `inv_block_id_at_idx_uniq`,
  `heap_ids_thread_id1`, `heap_ids_thread_id2`, `inv_heap_shared_access`,
  `page_implies_segment_enabled`, `blocks_has`.
- `BoolAgree::State::inv_eq`.

## Inventory and preservation

All **30 tracked Rust sources** were inventoried against the exact baseline
revision, then against the final working-tree bytes, using the existing general
Verus source-AST frontend. All parsed with zero diagnostics. Source publication
syntax, not VIR fuel, determined open/closed classification.

The baseline contains **106 explicit and 12 implicit closed definitions with
bodies**, including the struct macro's closed `wf`. One of these,
`commit_mask::is_bit_set`, already has `{ true }` and was left unchanged.
The 117 changed definitions comprise 106 explicit and 11 implicit definitions.
The implicit changes include private accessors, both agreement-machine
invariants, `MimDealloc::wf`, and `Mim::State::blocks_has`.

The final source has **84 literal-true closed definitions** (82 changed original
source definitions, the unchanged `is_bit_set`, and the materialized
`atomic_inv`) and **39 bodyless non-Boolean declarations**. Four bodyless specs
were already present and remain unchanged: `size_of_bin`, `CommitMask::bytes`,
`OsMem::view`, and `IsThread::view`.

Native AST comparisons found no differences in:

- 132 ordinary/source-machine open spec declarations, including restricted
  `open(crate)` declarations;
- 271 executable function declarations (including their bodies);
- 90 source trust attributes;
- retained function contract clauses, including `recommends` and `decreases`.

The native frontend treats `struct_with_invariants!` as an opaque source macro.
All four original instances were therefore read manually. Three have open `wf`
declarations and remain byte-identical, bringing the preserved source-open count
to **135**. The fourth, `ThreadLLSimple`, is handled below rather than exempted.
Remaining macro definitions/calls are unchanged and do not hide additional
source closed spec declarations.

The apparent closed definitions in [verus-mimalloc/bitmap.rs](verus-mimalloc/bitmap.rs)
and [verus-mimalloc/arena.rs](verus-mimalloc/arena.rs) are entirely inside block
comments (respectively two bool/two nonbool and one bool spellings). They are
not compiled declarations and were left untouched. Other historical commented
spec snippets in the linked-list, queues, tokens, and OS-memory utility sources
likewise remain inert. There are **no active concrete-body exemptions**.

An independent review initially compared an unrelated audit `.orig` file and
reported missing `external_body` annotations. Exact Git checks rejected that
finding: `linked_list.rs` has zero such annotations in both `9f99d4c` and
`41b1aeb`, and still has zero after this repair. Historical audit snapshots
are not the baseline and must not authorize adding trust to this branch.

## Macro and compiler compatibility

`state_machine!` and `tokenized_state_machine!` accept source `uninterp spec fn`
declarations directly. Their fields, transitions, properties, and tracked
resource operations were not deleted or replaced.

The struct-invariant DSL requires an invariant declaration for each placeholder
atomic field. Minimal compiler probes established:

1. `wf { true }` fails: expected an invariant or predicate declaration.
2. `wf { predicate { true } }` fails: no invariant declared for the atomic field.
3. Explicit resource types, aliases, predicate type, and trait implementation
   compile with literal-true `wf` and `atomic_inv` bodies.

Accordingly, the `ThreadLLSimple` macro was manually desugared following the
installed generic macro implementation. Its field order, constant tuple,
`Tracked<LL>` resource type, generated type names/aliases, and trait signature
are retained. No `external_body`, assumptions, or axioms were added. Four
non-camel-case warnings are expected because the generated type names are
deliberately preserved.

Verus rejected `opaque` on bodyless `get_list_idx` and `ucount`, so those two
attributes were removed. It then rejected ten reveals of those bodyless specs;
only those ten statements were removed from their otherwise retained proofs.
No unrelated proof deletion was performed.

**No decreases clauses were removed in the final source.** A dedicated
state-machine probe confirmed that `uninterp ... decreases idx;` both compiles
and verifies (0 verified, 0 errors) on this toolchain. `ucount_sum` retains its
original clause; the initial provisional removal was reversed.

Uninterpreted functions intentionally lose their defining equations; Boolean
abstraction intentionally weakens predicates, including invariants. Unchanged
open functions can consequently have different semantics through their closed
dependencies. Source trust-marker preservation is not a claim that expanded
trust annotations are identical: uninterpreted declarations and tokenized
resource interfaces use Verus-generated machinery. The retained generated
interfaces are not evidence that their protocol/inductive obligations hold.

## Validation

- Baseline [build-verus-mimalloc.sh](build-verus-mimalloc.sh): exit **0**.
- Final same build: exit **0**, four naming warnings, shared library produced.
- Full-project diagnostic: `verus` on the allocator crate, libc dependency,
  `override_system_allocator` feature, `--crate-type lib`, `--rlimit 10`, no
  function/module filtering: exit **1**, **499 errors**, four naming warnings.
  Diagnostics include a computation assertion, preconditions, loop invariants,
  indexing, and arithmetic obligations. No successful verification summary was
  emitted. Full baseline verification was not rerun, so these are not all
  attributed to this patch.
- Final source-AST parse/preservation checks and `git diff --check`: pass.
- No runtime benchmark or binary-equivalence test was performed. Runtime
  preservation evidence is unchanged executable source plus the reviewed
  equivalent struct resource-type expansion and successful build.

Local, ignored build evidence (not automatically included in a later commit):

| Artifact | SHA-256 |
| --- | --- |
| [build/closed-reviewed-build.log](build/closed-reviewed-build.log) | `b76ed8814f0490163ea130c7e1104e29605398a265cf016bf8288db6f2e69e6a` |
| [build/closed-reviewed-verification.log](build/closed-reviewed-verification.log) | `4e2ae11e0719c21be3786a52d88619ab3c69fa38c39196bcc62f947393e24876` |
| [build/closed-reviewed-source-inventory.json](build/closed-reviewed-source-inventory.json) | `772ce0bd6135263e85e86cd7ed093080c32cf93c10c0e809cd415e771442162c` |

The inventory binds the source root, baseline revision, frontend binary hash,
and baseline/final hashes and source spans for each Rust file. No
allocator-specific parser or replacement script was created; inline Python
only invoked the existing frontend and compared/reported its facts. All source
edits, probes, and this report were made with manual patches.