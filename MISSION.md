# Mission — retire the `todo()` stubs in verus-mimalloc

## Context

This branch (`upstream-main`) is the **original, fully verified** verus-mimalloc
at `verus-lang/verified-memory-allocator@9f99d4c`. `make verify` is **green at
the freeze**: `736 verified, 0 errors`.

Green does **not** mean finished. The allocator ships 50 call sites of

```rust
#[verus::trusted]
#[verifier::external_body]
pub fn todo()
    ensures false          // <-- proves anything downstream
{ panic!("todo"); }
```

`todo()` has `ensures false`, so every path that reaches it is vacuously
verified and every obligation after it is discharged for free. These are the
allocator's genuinely **unfinished** code paths — the ones upstream never got
around to implementing or proving. Retiring them is this mission.

## The task

For every reachable `todo()` call site: replace the stub with a real
implementation and **prove it**, under the frozen goal contracts. When a site
is genuinely unreachable, prove it unreachable (a discharged precondition),
do not leave `todo()` standing as the proof.

Current distribution (50 sites):

| file | sites | nature |
| --- | --- | --- |
| `verus-mimalloc/lib.rs` | 21 | C ABI shims (`realloc`, `posix_memalign`, `memalign`, `valloc`, `pvalloc`, `reallocarray`, …), all `#[verifier::external]` — **sanctioned TCB**, out of scope |
| `verus-mimalloc/segment.rs` | 14 | segment alloc/free paths, huge-segment handling |
| `verus-mimalloc/os_alloc.rs` | 6 | OS allocation fallbacks |
| `verus-mimalloc/init.rs` | 2 | heap/thread init edge cases |
| `verus-mimalloc/alloc_generic.rs` | 2 | deferred free / heap-init paths |
| `verus-mimalloc/page.rs`, `layout.rs`, `free.rs`, `commit_segment.rs`, `commit_mask.rs` | 1 each | |

The 29 non-`lib.rs` sites are the real work.

## Frozen boundaries (`.verus_agent/`)

Goals (contracts frozen byte-identical, bodies free):

- `heap_init` — `verus-mimalloc/init.rs`
- `heap_malloc` — `verus-mimalloc/alloc_fast.rs`
- `free` — `verus-mimalloc/free.rs`
- `global_init` — `verus-mimalloc/init.rs` (proof fn; `frontier: false`,
  invisible to the LSP call graph, contract still enforced)

Sanctioned permanent TCB (`tcb_manifest.json`) — the only trust allowed at
completion:

- `verus-mimalloc/os_mem.rs` — `mmap`/`munmap`/`mprotect` syscalls
- `verus-mimalloc/thread.rs` — thread-id primitive
- `verus-mimalloc/lib.rs` — the C ABI export shims
- `print_hex`

**`todo` is deliberately NOT sanctioned.** It is derived proof debt, and
`grep -c 'todo();' verus-mimalloc/*.rs` outside `lib.rs` must reach 0 before
this mission is complete.

## Hard rules

1. **Never weaken a spec to make something pass.** The frozen goal contracts
   are byte-identical-enforced; the middle-layer contracts must stay at least
   as strong as what the callers already rely on.
2. **Never enlarge the TCB.** No new `external_body`, `external`, `uninterp`,
   `assume`, `admit`, `assume_specification`, `unimplemented!()`. Net-new trust
   is rejected by `boundary_guard check`.
3. **Do not swap `todo()` for another escape hatch.** Deleting `todo()` and
   adding `assume(false)`, `admit()`, or `external_body` to the enclosing
   function is not progress; it is the same trust wearing a different hat.
4. **`ensures false` may not be introduced anywhere.**
5. Behaviour of already-proven code stays put: `make verify` must remain at
   0 errors on every commit, with the verified count monotonically growing as
   real bodies replace stubs.
6. Every materially distinct failed / inconclusive proof approach is recorded
   append-only in `research/proof_attempts.jsonl` before moving on.

## Toolchain (pinned)

- Verus `0.2026.06.28.1847ab3`, built **from source with `--features singular`**
  at `/users/jinlang/tools/verus-src/source/target-verus/release`
  (`layout.rs` uses `#[verifier::integer_ring]`, which the stock release
  binary cannot run).
- Singular: `/usr/bin/Singular` (`VERUS_SINGULAR_PATH`).
- `libc` rlib must be built with rustc **1.96.0** — the Verus toolchain —
  not 1.97.1 (`setup-libc-dependency.sh`).
- Verify with `make verify` at the project root. Do not upgrade Verus.

## Integrity — no answer key

verus-mimalloc derives from published work, but these `todo()` sites are
precisely the parts nobody has published a proof for. Do not fetch, copy, or
reconstruct proofs from any external allocator source, and do not treat the
upstream C mimalloc as a specification oracle. Derive the proofs here.
