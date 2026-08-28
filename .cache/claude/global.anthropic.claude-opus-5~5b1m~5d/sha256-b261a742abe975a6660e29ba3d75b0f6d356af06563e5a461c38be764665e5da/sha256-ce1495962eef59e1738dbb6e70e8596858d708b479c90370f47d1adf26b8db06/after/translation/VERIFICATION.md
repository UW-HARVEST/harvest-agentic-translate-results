# Verification report — C ⇄ Rust differential testing

The C in `../c_src` is the ground truth. Everything below compares the two
**shared objects** through `libloading`; the Rust implementation is never called
directly, so the `#[no_mangle] extern "C"` export wrapper is under test too.

## How to run

```sh
# One-shot: builds both .so's, diffs symbols, runs every test across
# all feature combinations AND both build profiles.
./verify.sh

# Anti-vacuity check: inject known-wrong behaviour and confirm the suite catches it.
./mutation_check.sh

# Knobs (all optional):
#   TRIALS=<n>       randomized inputs per (row, size)   [default 12]
#   FUZZ_ITERS=<n>   iterations for the cross-product fuzz [default 4000]
#   SEED=<u64>       PRNG seed                            [default fixed]
#   C_LIB_PATH / RUST_LIB_PATH   point at specific .so files
TRIALS=60 FUZZ_ITERS=40000 cargo test --tests
```

## Artifacts

| file | contents |
|---|---|
| `SYMBOLS.md` | every `nm -D` symbol of the C `.so`, and its Rust counterpart |
| `ERRORS.md` | error-surface table (20 rows) + test mapping + observed signal parity |
| `CONFIGS.md` | configuration-surface table (30 rows) over all input-shape axes |
| `tests/common/mod.rs` | harness: dual `dlopen`, generators for every axis, byte-exact comparator |
| `tests/phase_b_configs.rs` | 30 tests — one per `CONFIGS.md` row |
| `tests/phase_c_errors.rs` | 15 tests — one per `ERRORS.md` row (fatal rows via subprocess signal comparison) |
| `tests/phase_d_symbols.rs` | 3 tests — symbol parity enforced by `cargo test` |
| `verify.sh` | Phase D driver: symbol diff × feature combos × profiles |
| `mutation_check.sh` | 11 mutants × 2 profiles, all must be caught |

## Bugs found and fixed

Both were **real divergences** from the C, found by the differential tests. The C
was never modified.

### 1. Struct padding was not propagated (all profiles)

`spritebatch_sprite_t` is `{ unsigned long long; int; }` → 16 bytes with **4 bytes
of trailing padding** at offsets 12..16.

GCC compiles the C's `b[k] = a[i]` into two 8-byte `mov`s spanning all 16 bytes,
so the padding **travels with the element** and is observable in the output. The
original Rust used `*b.offset(k) = *a.offset(i)`; rustc/LLVM treat padding as
`undef` and copy only the two real fields, so the destination kept whatever
padding the earlier `memcpy` had left there:

```
input   a[0].pad = 0x80000001   a[1].pad = 0x81010101
C    →  a[0].pad = 0x81010101   (padding moved with the element)
Rust →  a[0].pad = 0x80000001   (padding stayed behind)   ← WRONG
```

Fixed by `sprite_assign()`, which mirrors GCC exactly: two `MaybeUninit<u64>`
loads (both issued *before* either store, so the aliased case is also correct),
then two stores. Caught by `CONFIGS.md` rows 23–25/27/29–30 and `ERRORS.md` #20.
The mutants `drop_padding_word` and `drop_first_word` re-detect it.

### 2. `copy_nonoverlapping` aborted where C succeeds/segfaults (profile-dependent)

`merge_sort`'s `memcpy` was translated as `ptr::copy_nonoverlapping` behind an
`if bytes != 0` guard. That diverges in two ways:

* **Aliased buffers** (`merge_sort(p, p, n)`, `ERRORS.md` #18): violates
  `copy_nonoverlapping`'s no-overlap precondition. In a **debug** build the
  `unsafe precondition(s) violated` check fires and the process aborts, where the
  C returns normally. Release silently worked, so this was invisible until the
  suite was run against the debug `.so`.
* **Null pointers / negative `size`**: Rust's own precondition and zero-guard
  could report a different failure mode than the C's raw `memcpy`.

Fixed by binding libc `memcpy` via `extern "C"` and calling it **unconditionally**,
exactly as the C does. Both sides now hand the *same* byte count to the *same*
glibc routine, so out-of-domain inputs agree by construction. Confirmed at the
instruction level:

```
C     merge_sort:  cltq    ; shl $0x4,%rax ; call memcpy@plt
Rust  merge_sort:  movslq  ; shl $0x4,%rdx ; call *memcpy@GLIBC_2.14
```

## Notable C behaviours replicated verbatim (not "fixed")

* **`less_than_or_equal` line 9 is dead code.** `if (a->sort_bits <= b->sort_bits) return 1;`
  already covers the `==` case, so the `texture_id` tiebreak is **never reached**
  and `texture_id` has **zero** influence on ordering. `err12_13` pins this as a
  property: inputs differing only in `texture_id` must be permuted identically.
  The `cmp_make_dead_branch_live` mutant confirms the tests would notice if the
  Rust "fixed" it.
* **The result lands in `a`, not `b`.** `b` is scratch and is only partially
  written; the recursion's leaf guard relies on the initial `memcpy` having made
  both buffers equal. Both buffers are compared on every test.
* **Negative `size` is not rejected.** It sign-extends to a ~2^64 byte count;
  `-1`/`-2` silently no-op, `-1000`/`INT_MIN` segfault (see `ERRORS.md`).

## Completion gate

- [x] **`SYMBOLS.md`** — `comm -23` of the C and Rust defined-symbol lists is
      **empty**; 0 undefined non-libc symbols in the Rust `.so`. The single C
      export `merge_sort` is exported by Rust under the exact same name. The three
      C `static` helpers are correctly absent from both. Enforced by
      `tests/phase_d_symbols.rs`.
- [x] **Phase B** — all **30** `CONFIGS.md` rows pass across randomized inputs
      (fixed seed; additionally re-run under 6 different seeds and at
      `TRIALS=60 FUZZ_ITERS=40000`).
- [x] **Phase C** — all **20** `ERRORS.md` rows either have a passing
      differential test (18 rows) or are documented as structurally unreachable
      (#17 requires a lying `size`; #19 requires 17 GB). Fatal rows compare the
      exact `WTERMSIG`; surviving rows additionally compare a digest of both
      buffers so no "both survived" can pass vacuously.
- [x] **Every feature combination** — the crate declares **no** `[features]`
      (`cargo metadata` → `"features":{}`, no `cfg(feature=…)` anywhere), so the
      combination set is `{default}`. `verify.sh` nonetheless runs `<default>`,
      `--no-default-features` and `--all-features` × `{release, debug}` = **6
      configurations**, all green. Testing both profiles is what exposed bug #2.
- [x] **Anti-vacuity** — `mutation_check.sh`: 11 mutants × 2 profiles, **22/22
      caught**; `src/lib.rs` SHA-256 re-verified as pristine afterwards.
