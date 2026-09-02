# CONFIGS.md — configuration surface (valid inputs)

Derived mechanically from `c_src/src/lib.c`, `c_src/include/lib.h`,
`c_src/CMakeLists.txt` and `translation/Cargo.toml`.

## Axes the C actually branches on

**Runtime options / modes / flags:** *none.*
`grep -c '#if\|#ifdef\|#ifndef' c_src/src/lib.c c_src/include/lib.h` → `0`.
The header declares no flags, no context struct, no init function.
`CMakeLists.txt` sets no `target_compile_definitions`.

**Cargo feature combinations:** *none.*
`translation/Cargo.toml` has no `[features]` section and no `[dependencies]`,
and `src/lib.rs` contains no `cfg(feature = …)`. The default build is the only
build, so "every feature combination" == the single default configuration.
(Verified by script, see bottom of this file.)

**Public entry points:** exactly one, `merge_sort`. The three lower-level
functions (`spritebatch_internal_sprite_less_than_or_equal`,
`…_merge_sort_iteration`, `…_merge_sort_recurse`) are `static`, absent from
`nm -D`, and therefore unreachable across the FFI boundary. They are driven
*through* `merge_sort`, and the axes below are chosen specifically to reach each
of their branches:

| lower-level branch | reached by |
|---|---|
| `recurse`: `hi - lo <= 1` early return | `size` 0 and 1; every leaf of every larger `size` |
| `recurse`: odd vs. even `(lo + hi) / 2` split | odd and even `size`, and non-power-of-two `size` |
| `recurse`: buffer role swap `a`↔`b` per level | `size` values whose recursion depth is odd vs. even — decides whether the sorted result lands in `a` or in `b`, so **both buffers must be compared** |
| `iteration`: comparator `<=` true | ascending / duplicate `sort_bits` |
| `iteration`: comparator false | descending `sort_bits` |
| `iteration`: `j >= hi` short-circuit | right run exhausted first, i.e. `size >= 2` with descending data |
| `iteration`: `i >= split` | left run exhausted first, i.e. ascending data |
| `leq`: dead `texture_id` tiebreak | equal `sort_bits` with varying `texture_id` |

**Input shape axes** (the only thing the code varies on):

* `size` (`int`): `0`, `1`, `2`, `3`, `4`, `5`, `7`, `8`, `9`, `15`, `16`, `17`,
  `31`, `32`, `33`, `63`, `64`, `100`, `127`, `128`, `129`, `1000`, `1024`, `4096`
  — covers empty / one / many, powers of two, ±1 around powers of two, odd and
  even, and both recursion-depth parities.
* `sort_bits` (`int`) distribution: all-equal, two-valued, small range (heavy
  duplicates), all-distinct random, already-ascending, already-descending,
  extremes (`INT_MIN`, `INT_MAX`, `0`, `-1`) mixed in.
* `texture_id` (`unsigned long long`) distribution: random, all-equal, extremes
  (`0`, `u64::MAX`) — must be verified, not assumed irrelevant.
* struct tail padding bytes: all-zero vs. random garbage (observable, since the
  struct copy is 16 bytes wide).
* scratch buffer `b` initial content: all-zero vs. random garbage.
* observation: byte image of **`a` and `b`** after the call.

## Configuration table

One row per combination the C treats differently. Every row is exercised with
many randomized inputs (fixed seed `0x5EED_C0FFEE`), not a single value, and
compares the full 16-byte-per-element byte image of **both** output buffers.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| 1  | `merge_sort` | `size = 0`; `a`/`b` filled with random bytes (incl. padding); assert **both buffers bit-identical to input** | [x] |
| 2  | `merge_sort` | `size = 1`; random element, random padding, random `b` | [x] |
| 3  | `merge_sort` | `size = 2`; all 2-element orderings (asc, desc, equal) × random `texture_id` | [x] |
| 4  | `merge_sort` | `size = 3` (odd split `(0+3)/2 = 1`); random `sort_bits` over a 3-value range | [x] |
| 5  | `merge_sort` | `size` ∈ {4, 8, 16, 32, 64, 128, 1024} (powers of two, even splits at every level); fully random `sort_bits`/`texture_id` | [x] |
| 6  | `merge_sort` | `size` ∈ {5, 7, 9, 15, 17, 31, 33, 63, 100, 127, 129, 1000, 4096} (non-powers-of-two, mixed odd/even splits) ; fully random | [x] |
| 7  | `merge_sort` | all `sort_bits` **equal**, `texture_id` all distinct random — exercises the dead tiebreak; C keeps input order, so `texture_id` must come out unsorted | [x] |
| 8  | `merge_sort` | all `sort_bits` equal **and** all `texture_id` equal (total duplicates) | [x] |
| 9  | `merge_sort` | `sort_bits` already strictly **ascending** (best case: `i >= split` path dominates) | [x] |
| 10 | `merge_sort` | `sort_bits` already strictly **descending** (worst case: `j >= hi` short-circuit path dominates) | [x] |
| 11 | `merge_sort` | `sort_bits` drawn from a **2-value** set (massive duplicates, stability-sensitive) | [x] |
| 12 | `merge_sort` | `sort_bits` drawn from a **small range** (`0..4`), many runs of duplicates | [x] |
| 13 | `merge_sort` | `sort_bits` = full-range random `i32` **including `INT_MIN`/`INT_MAX`** (comparator uses signed `<=`; catches any unsigned mix-up) | [x] |
| 14 | `merge_sort` | `sort_bits` ∈ {`INT_MIN`, `INT_MAX`} only — maximal-distance signed pairs | [x] |
| 15 | `merge_sort` | `texture_id` ∈ {`0`, `u64::MAX`} with random `sort_bits` (u64 boundary values) | [x] |
| 16 | `merge_sort` | struct **tail padding filled with random garbage**, `sort_bits`/`texture_id` random — verifies the 16-byte (not 12-byte) copy | [x] |
| 17 | `merge_sort` | scratch buffer `b` pre-filled with random garbage, `size` odd and even — verifies `b`'s final content matches C exactly (incl. which buffer the sorted result lands in) | [x] |
| 18 | `merge_sort` | `size = 4096`, `sort_bits` all equal + padding garbage + `b` garbage (all axes at once, deep recursion) | [x] |
| 19 | `merge_sort` | **fuzz sweep**: 400 iterations, `size` uniform in `0..=256`, every field and every padding byte uniformly random, `b` random | [x] |
| 20 | `merge_sort` | **repeated invocation** on the same buffers (call `merge_sort` three times in a row on the already-sorted result) — checks idempotence matches C, incl. `b`'s state | [x] |
| 21 | `merge_sort` | **`a == b`** (aliased buffers): `memcpy` with `src == dst`, and every `b[k] = a[i]` reads and writes the same array | [x] |
| 22 | `merge_sort` | **overlapping** `a` and `b` at every whole-element offset `1..=n` (the only overlap reachable for a 16-byte-slot pointer) | [x] |
| 23 | `merge_sort` | **both pointers misaligned** by 1,2,3,4,5,7 bytes — the C's `mov`-based accesses tolerate this, so the Rust must too | [x] |
| 24 | `merge_sort` | **exactly one** pointer misaligned (`a` only, then `b` only) | [x] |
| 25 | `merge_sort` | `size` **smaller** than the real allocation, for every `size in 0..=n` — the untouched tail of both buffers must match | [x] |
| 26 | `merge_sort` | `size` **larger** than the caller's logical length but inside the real allocation (17…128 over a 128-element store) — out-of-logical-range indexing must match | [x] |
| 27 | `merge_sort` | exhaustive `size` sweep `0..=512` against one fixed 512-element allocation | [x] |
| 28 | `merge_sort` | `size` = 65535 / 65536 / 65537 / 131071 / 131072 — deepest affordable recursion (17 levels), exercising the midpoint arithmetic at every level | [x] |

## Feature-combination enumeration (script-verified)

```sh
$ ./run_all_configs.sh
== declared features: 0 (none)
```

No `[features]` ⇒ one feature configuration. `run_all_configs.sh` nevertheless
enumerates the feature powerset mechanically from `Cargo.toml` and runs the full
suite for each, and additionally runs **both build profiles**, because the two
profiles are genuinely different code paths for this translation: the `dev`
profile enables `debug_assertions`, which turns on Rust's UB precondition checks
(pointer alignment, `copy_nonoverlapping` non-overlap) and integer overflow
checks. Configurations covered:

| configuration | Rust `.so` under test | result |
|---|---|---|
| `--release --no-default-features` | `target/release/libmerge_sort_lib.so` | 44/44 pass |
| `--no-default-features` (dev) | `target/debug/libmerge_sort_lib.so` | 44/44 pass |
| `--release` (default features) | `target/release/libmerge_sort_lib.so` | 44/44 pass |
| (dev, default features) | `target/debug/libmerge_sort_lib.so` | 44/44 pass |

## Divergence found and fixed

The dev-profile run is what exposed the one real defect in the translation. The
original Rust formed `&*ptr` references to sprites and used
`core::ptr::copy_nonoverlapping`, which imposes alignment / non-overlap / non-null
preconditions **that the C does not have** — the C just issues `mov`s and plain
address arithmetic. Rows 21–24 aborted the dev build with

```
misaligned pointer dereference: address must be a multiple of 0x8
unsafe precondition(s) violated: ptr::copy_nonoverlapping requires that both
pointer arguments are aligned and non-null and the specified memory ranges do
not overlap
```

i.e. the translation was relying on latent UB that merely happened to work with
optimizations on. `src/lib.rs` was rewritten to mirror gcc's codegen exactly:
unaligned field loads instead of references, a two-`u64` load-then-store sprite
copy instead of `copy_nonoverlapping`, `wrapping_offset` instead of `offset`, and
a direct call to libc `memcpy` for the bulk copy. All 28 rows then pass in both
profiles.

## Harness discrimination (mutation-tested)

To confirm the rows above are load-bearing rather than vacuously passing, five
mutations were injected into `src/lib.rs` and the suite re-run. Failures per
suite:

| mutation | valid_paths | error_paths | blind_spots |
|---|---|---|---|
| comparator `<=` → `<` (makes the dead `texture_id` tiebreak live) | 12 | 4 | 3 |
| `copy_sprite` copies 12 bytes instead of 16 (drops padding) | 16 | 9 | 5 |
| `sort_bits` compared as `u32` instead of `i32` | 14 | 8 | 3 |
| negative `size` clamped instead of wrapping to a ~2**64 length | 0 | 1 | 0 |
| `recurse` merges into the wrong buffer | 16 | 9 | 4 |

A no-op control mutation produced 0 failures. `src/lib.rs` was restored and
rebuilt after each mutation (verified with `diff`).
