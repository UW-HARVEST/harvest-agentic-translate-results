# CONFIGS.md — Phase B configuration surface table (valid inputs)

Mechanically derived from the branches the C in `c_src/src/lib.c` actually takes.

## Axes the C code branches on

**A. Runtime options / flags.** The public header exposes exactly one function
and no options struct, no mode flags, no globals, no `#ifdef`. So there are
**zero runtime option axes**. All variation comes from input shape and data.

**B. Public entry points (full set, incl. lowest level).**

| entry point | linkage | reachable from a `.so` consumer? |
|---|---|---|
| `merge_sort(a, b, size)` | extern | yes — the only dynamic symbol |
| `spritebatch_internal_merge_sort_recurse(b, lo, hi, a)` | `static` | no (not exported by C either); exercised *indirectly* via every `size >= 2` row, which is why the deep/odd sizes below matter |
| `spritebatch_internal_merge_sort_iteration(a, lo, split, hi, b)` | `static` | no; exercised indirectly. Its `i < split` / `j >= hi` / predicate branches are covered by rows 6-14 |
| `spritebatch_internal_sprite_less_than_or_equal(a, b)` | `static` | no; its three `return`s are covered by rows 10-13 |

Note: because the three low-level functions are `static`, "exercising the
low-level entry points directly" means driving `merge_sort` with the input
shapes that force each internal branch — which is what the table below does,
one row per distinct internal branch combination.

**C. Input shapes the code special-cases.**

1. `size` vs the `hi - lo <= 1` guard: `0`, `1`, `>= 2`.
2. `size` parity / split structure: `(lo+hi)/2` makes odd sizes produce
   unequal halves, powers of two produce perfectly balanced trees.
3. Recursion depth: `ceil(log2(size))` — determines which of `a`/`b` holds the
   final sorted run (the buffers are swapped at every level), so BOTH buffers
   must be compared, and the *parity of the depth* is itself an axis.
4. `sort_bits` relations driving the predicate: `<`, `==`, `>`; ascending,
   descending, all-equal, random, negative, `INT_MIN`/`INT_MAX`.
5. `texture_id` relations: irrelevant to ordering (dead second `if`) — must be
   verified to be irrelevant identically in both, incl. `0` and `u64::MAX`.
6. Struct padding: 4 trailing padding bytes; `memcpy` copies them, member-wise
   struct assignment may or may not. Compared explicitly.
7. Buffer state: `b` pre-filled with a sentinel (detects partial/extra writes);
   guard elements past `size` (detects overruns).
8. Aliasing: `a == b`.

## Configuration table

Every row is run with **many randomized inputs** (fixed seed, deterministic
xorshift PRNG) unless the row is a fixed boundary. Every row asserts that the
**entire `a` arena and the entire `b` arena** (including guard regions and
padding bytes) are byte-identical between the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `merge_sort` | `size = 0`; both buffers non-null, pre-filled with random sentinel; expect no mutation. Randomized sentinels. | `row01_size_zero_no_mutation` | [x] |
| 2 | `merge_sort` | `size = 1`; random element; `memcpy` path only, `recurse` early-out. Randomized. | `row02_size_one` | [x] |
| 3 | `merge_sort` | `size = 2`, already ascending (`sort_bits[0] < sort_bits[1]`) — forces `i < split && pred == 1` branch at depth 1. Randomized values. | `row03_size_two_ascending` | [x] |
| 4 | `merge_sort` | `size = 2`, descending (`sort_bits[0] > sort_bits[1]`) — forces the `else` branch (`pred == 0`). Randomized. | `row04_size_two_descending` | [x] |
| 5 | `merge_sort` | `size = 2`, equal `sort_bits`, distinct random `texture_id`s — exercises the dead second `if`. Randomized. | `row05_size_two_tied_keys_dead_branch` | [x] |
| 6 | `merge_sort` | `size = 3` (odd; split = 1, unbalanced halves, depth 2). Randomized `sort_bits`. | `row06_size_three_odd_split` | [x] |
| 7 | `merge_sort` | `size = 4` (power of two; balanced tree, depth 2 — even depth, result parity differs from row 6). Randomized. | `row07_size_four_power_of_two` | [x] |
| 8 | `merge_sort` | `size = 5, 6, 7` (odd + even non-power-of-two, depth 3). Randomized. | `row08_sizes_five_six_seven` | [x] |
| 9 | `merge_sort` | `size = 8, 16, 32, 64` (powers of two, depths 3-6). Randomized. | `row09_powers_of_two` | [x] |
| 10 | `merge_sort` | every `size` in `1..=40`, fully random `sort_bits` over the whole `i32` range and random `texture_id` over the whole `u64` range; many seeds per size. Covers all split/depth/parity combinations densely. | `row10_dense_sweep_random_keys` | [x] |
| 11 | `merge_sort` | every `size` in `1..=40`, `sort_bits` drawn from a **tiny alphabet** (`0..3`) so runs of equal keys are frequent — stresses the `pred` tie path and merge stability, with random `texture_id`. | `row11_dense_sweep_tiny_alphabet` | [x] |
| 12 | `merge_sort` | every `size` in `1..=40`, **all `sort_bits` identical**, `texture_id` strictly increasing / strictly decreasing / random — proves `texture_id` is ignored identically. | `row12_all_keys_equal` | [x] |
| 13 | `merge_sort` | every `size` in `1..=40`, strictly ascending and strictly descending `sort_bits` (best/worst case for the merge branches). | `row13_sorted_and_reverse_sorted` | [x] |
| 14 | `merge_sort` | `sort_bits` drawn from the extreme set `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` and `texture_id` from `{0, 1, u64::MAX/2, u64::MAX}`, sizes `2..=24`, randomized permutations. | `row14_extreme_values` | [x] |
| 15 | `merge_sort` | large input: `size = 1000`, `4096`, `4097` — deep recursion (depth 10-13), randomized data. | `row15_large_inputs` | [x] |
| 16 | `merge_sort` | `b` arena pre-filled with a random sentinel pattern *and* guard elements after index `size-1` in both arenas — detects any write outside `[0, size)` and any difference in which buffer the final run lands in. Sizes `0..=33`. | `row16_guard_regions_and_sentinel` | [x] |
| 17 | `merge_sort` | struct **padding bytes set to a non-zero random pattern** (arena written as raw bytes, not as structs) — checks that C's `memcpy` + member/struct assignment and Rust's 16-byte copy agree on the padding bytes. Sizes `0..=17`. | `row17_nonzero_padding` | [x] |
| 18 | `merge_sort` | aliased buffers `a == b` (same pointer), sizes `0..=17`, randomized data. | `row18_aliased_buffers` | [x] |
| 19 | `merge_sort` | misaligned-relative-offset buffers: `a` and `b` are non-overlapping slices of one arena separated by a random gap, so their relative distance varies; sizes `1..=17`. | `row19_windows_varying_gap` | [x] |
| 20 | `merge_sort` | repeated invocation: call `merge_sort(a, b, size)` twice in a row on the same buffers (idempotency/state-carryover), sizes `1..=17`, randomized. | `row20_repeated_invocation` | [x] |

## Additional randomized coverage (`tests/stress_random.rs`)

On top of the per-row tests, ~322 000 randomized configurations are compared
byte-for-byte with fixed seeds. Every trial randomizes the size, the guard width,
the key distribution (full-`i32` / binary / tiny-alphabet / ascending /
descending / extremes), the `texture_id` distribution, **and all padding bytes**:

| test | trials | shape |
|---|---|---|
| `stress_small_inputs_200k_trials` | 200 000 | `size` 0..8 |
| `stress_medium_inputs_50k_trials` | 50 000 | `size` 0..40 |
| `stress_large_inputs_2k_trials` | 2 000 | `size` 0..600 |
| `stress_aliased_50k_trials` | 50 000 | `a == b`, `size` 0..16 |
| `stress_permutation_invariant` | 20 000 | also checks the winning buffer is a sorted permutation of the input |

## Divergences found and fixed during Phase B

| # | divergence | root cause | fix |
|---|---|---|---|
| 1 | Rows 17 and 19 failed: the 4 **struct padding bytes** differed at every element the merge wrote. | gcc compiles the C struct assignment `b[k] = a[i]` as a full 16-byte move (`mov 0x8(%rax),%rdx; mov (%rax),%rax; mov %rax,(%rcx); mov %rdx,0x8(%rcx)`), i.e. it copies the padding. The Rust `*b.offset(k) = *a.offset(i)` on the `#[repr(C)]` struct copied only the two initialised fields and left the destination's padding intact. Observable to any caller that inspects the buffers as raw bytes. | `src/lib.rs`: `spritebatch_internal_merge_sort_iteration` now moves the element through a `[u8; 16]` temporary (`read_unaligned` / `write_unaligned`), reproducing gcc's 16-byte move — including when `src == dst` in the aliased case. |

Note that rows 1-16, 18 and 20 all passed *before* this fix, because they write
their sprites through the `Sprite` struct and therefore leave padding zeroed.
The bug was only visible once the padding was deliberately randomized — which is
exactly why row 17 exists.

## Binary / driver

`c_src/CMakeLists.txt` builds **only** `add_library(... SHARED src/lib.c)` —
there is no `add_executable`, and `translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]` with no `[[bin]]`. **The project builds no
binary executable**, so the "compare C and Rust stdout" gate is vacuous / not
applicable.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table**. The only combination is
the default, which is identical to `--no-default-features`. `verify.sh`
enumerates features mechanically from `Cargo.toml` (it would build the full
powerset if any existed) and runs the whole suite for each.

Because the behaviour being matched (row 17's struct-padding move) is
**codegen-sensitive**, `verify.sh` additionally runs the entire suite against
BOTH the `release` and the `debug` build of the Rust cdylib, selected via the
`MERGE_SORT_RUST_SO` environment variable. That is 2 feature configurations x 2
optimisation levels = **4 verified configurations**, all passing.
