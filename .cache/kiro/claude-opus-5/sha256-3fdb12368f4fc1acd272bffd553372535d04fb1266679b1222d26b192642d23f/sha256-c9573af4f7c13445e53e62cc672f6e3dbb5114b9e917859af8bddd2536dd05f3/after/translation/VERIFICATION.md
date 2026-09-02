# VERIFICATION.md — differential verification of the C→Rust translation

The C in `c_src/` is ground truth. The Rust `cdylib` must be byte-identical
through the FFI boundary. Everything below is run by `cargo test`; both `.so`s
are loaded with `libloading` and every call — including the Rust one — goes
through an exported `extern "C"` symbol, never through a Rust function directly.

## Result

**The translation is correct.** No behavioral divergence was found, so no change
was made to `src/lib.rs`. Two defects were found and fixed in the *verification
harness*, one of which had made the entire suite vacuous.

| artifact | what it covers |
|----------|----------------|
| `SYMBOLS.md` | all 9 exported symbols; C↔Rust `nm -D` diff is empty |
| `ERRORS.md` | 34 rejection/error rows, each with a passing differential test |
| `CONFIGS.md` | 44 valid-configuration rows, each randomized with a fixed seed |
| `tests/symbols.rs` | Phase D symbol parity, asserted (3 tests) |
| `tests/valid_paths.rs` | Phase B (28 tests) |
| `tests/errors.rs` | Phase C (20 tests) |
| `tests/methodology.rs` | controls proving the harness measures the library (4 tests) |
| `tests/fresh_process.rs` | allocation-pattern equality across processes (1 test) |
| `scripts/check_features.sh` | every feature combination + release profile + parallel |
| `scripts/mutation_check.sh` | proves the suite detects real bugs (23/23) |
| `scripts/check_c_optimization_levels.sh` | C at `-O0/-O1/-O2/-O3/-Os` |

56 tests, green in every configuration, stable over 5 consecutive runs.

## The one thing that makes this library hard to test

`compare_allocations` compares the *addresses* of two live `malloc(4)` blocks:

```c
int *ptr1 = malloc(sizeof(int));
int *ptr2 = malloc(sizeof(int));
...
if (ptr1 < ptr2) result = 1; else if (ptr1 > ptr2) result = 2; else result = 3;
uninit_ptr = ptr1;
result += (*uninit_ptr > 0) ? 10 : 0;
```

Its return value is therefore **not a function of its arguments**. It is a
function of glibc's allocator state, and `arity4`/`arity3`/`arity2`/`arity` all
inherit that, because they add its result into theirs. Since glibc's tcache is a
LIFO free list, the address ordering *flips on every call*: the first call sees
ascending addresses (`1`), the next sees descending (`2`), and so on.

Three consequences drove the whole test design:

1. **A naive differential harness reports a false failure.** Calling C once and
   then Rust once compares two *different* allocator states. Measured:
   interleaved calls give `(12, 11)` forever — and they give exactly the same
   `(12, 11)` when the C library is compared against **a second copy of itself**.
   `tests/methodology.rs::control_unseeded_interleaving_makes_c_disagree_with_itself`
   pins this down: the C library "disagrees with itself" on 10 out of 10 calls.
   Any conclusion drawn from an unseeded 1:1 comparison is measuring glibc.

2. **So the tests force the allocator state instead.** `common::seed_heap`
   drains the 32-byte tcache bin and re-fills it so that the next `malloc` pair
   is returned in a chosen order. It is called immediately before *every*
   measured call, on both sides. That makes both implementations fully
   deterministic and — more usefully — lets every heap-sensitive row be tested
   under **both** orderings, so the `result = 1` and `result = 2` branches are
   each reached deliberately rather than by luck.

   The draining is not incidental: glibc only pushes a freed chunk into the
   tcache while the bin is below capacity (7), and beyond that it goes to the
   fastbin. A seed that just does `free(hi); free(lo)` therefore stops
   controlling anything once the bin happens to be full — which is exactly how
   `valid_arity_dispatch_len3` failed in the release profile, and only when run
   after other tests. `control_seeding_survives_any_prefilled_tcache` pre-loads
   the bin with 0…12 chunks and checks the forced ordering still holds.

3. **What remains environment-dependent is stated, not hidden.** In a fresh
   process the first call's result depends on what the *dynamic loader* left in
   the bin, and `dlopen` of a 4 MB Rust `cdylib` has a different allocation
   footprint than a 16 KB C `.so`. That is outside the library, and no faithful
   translation can control it — forcing a fixed answer in Rust would make it
   *diverge* from the C whenever the C returned the other one. What the Rust can
   and does guarantee is an identical `malloc`/`free` pattern, which is verified
   two ways:
   * `tests/fresh_process.rs` loads one implementation per process, seeds once,
     then runs a 500+-value script with no host allocations. The two processes
     produce identical sequences, so the number, size, order and placement of
     every allocation match.
   * A minimal external C driver (`dlopen` + `dlsym`, nothing else) gets
     identical output from both `.so`s, including the alternating pattern:
     `11 12 11 12 11 12` and `32 33 32 33 32 33`.

## Harness defects found (both would have produced a false "verified")

**1. `cargo test` never built the `cdylib` — the suite tested a stale `.so`.**
With `crate-type = ["cdylib"]` and integration tests that (correctly) do not link
the crate, Cargo has no reason to produce `libarity_lib.so` at all. The loader
looked it up by path, found nothing in `target/debug/`, silently fell back to a
`target/release/` copy built before the tests existed, and reported 48 passing
tests. Proof of how bad this was: `scripts/mutation_check.sh` injected 23
deliberate behavioral bugs into `src/lib.rs` and the suite caught **0**.
`tests/common/mod.rs` now builds the `cdylib` on demand (separate target dir,
profile matched to the test profile, features forwarded via
`HARVEST_SO_FEATURES`) and asserts the artifact is newer than `src/lib.rs`. After
the fix, **23 of 23** mutations are caught.

**2. The tcache seeding could be defeated by a full bin** (see point 2 above).

Both are now guarded by tests, so neither can silently return.

## Behaviors the C exhibits that the Rust had to reproduce

Each of these is a place a plausible translation would have gone wrong; all are
already correct in `src/lib.rs` and each is covered by a mutation that the suite
catches.

* **`arity`'s parameter is `unsigned char`, not `int`.** The header says `int`,
  the definition says `unsigned char`, and the compiled code compares only the
  low byte with an *unsigned* branch. So `arity(256, p) == -1`, `arity(258, p)`
  behaves like `arity(2, p)`, and `arity(-1, p)` is **not** rejected — it maps to
  255 and dispatches to `arity4`, reading four ints. Verified at every `-O` level.
* **`param1 % 4` can be negative.** C's `%` truncates toward zero, so a negative
  `param1` yields `-1`, `-2` or `-3` — none of which is a `switch` label in
  `apply_bitmask`, so the mask is silently *not applied*. A translation using
  `rem_euclid` would look more "correct" and be wrong.
* **`(result * param3) / 100` truncates toward zero,** not toward negative
  infinity, so `div_euclid` is wrong for negative numerators.
* **Signed overflow wraps.** UB in C, but the emitted code wraps at every
  optimization level; the Rust uses explicit `wrapping_*` rather than panicking.
* **`shift_array`'s copy overlaps.** `memmove(arr + positions, arr, ...)` needs
  `ptr::copy`, not `copy_nonoverlapping`.
* **`compare_allocations` reads through `uninit_ptr`, which aliases `ptr1`,**
  so the `+10` bonus depends on `val1` and not on `val2`, and on `> 0` and not
  `>= 0`.
* **No NULL checks anywhere.** The Rust adds none. Where a guard happens to
  short-circuit first (`arity` with `len < 2`, `shift_array` with a failing
  guard), a NULL pointer is safe in both and that is asserted; where it does not,
  both fault identically and there is no rejection value to compare.
* **`result = 3` and the `-1` OOM sentinel in `compare_allocations` are
  unreachable.** Two live allocations cannot share an address, and `malloc(4)`
  does not fail. Rather than fake an allocation failure (which would test the
  interposer, not the library), the tests assert the observable consequence:
  neither `.so` ever returns those values across the whole randomized sweep.

## Reproducing

```sh
# C ground truth
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

cd translation
cargo test                                    # full suite (builds the .so itself)
./scripts/check_features.sh                   # all feature combos + release + parallel
./scripts/mutation_check.sh                   # suite sensitivity: 23/23 caught
./scripts/check_c_optimization_levels.sh       # C at -O0/-O1/-O2/-O3/-Os
```

## Completion gate

* [x] `SYMBOLS.md`: `nm -D` diff empty; 9/9 C symbols exported by Rust; 0
      unresolvable non-libc imports. Asserted by `tests/symbols.rs`.
* [x] Phase B: all 44 `CONFIGS.md` rows pass across randomized inputs (fixed
      seed), each under both allocator orderings.
* [x] Phase C: all 34 `ERRORS.md` rows have a passing error-path differential
      test, asserting the same sentinel and not merely "both failed".
* [x] Every configuration: default features, `--no-default-features` (the crate
      declares no `[features]`, so this is the complete powerset), the release
      profile (`panic = "abort"` + optimizations), and parallel test threads.
* [x] Suite sensitivity demonstrated: 23/23 injected bugs detected.
* [x] Matches the C at `-O0`, `-O1`, `-O2`, `-O3`, `-Os`.
* [x] `src/lib.rs` unchanged from the reviewed translation (all mutations
      reverted; verified with `diff`).
