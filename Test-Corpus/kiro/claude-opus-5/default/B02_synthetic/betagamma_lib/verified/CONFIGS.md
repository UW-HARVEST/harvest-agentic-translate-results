# CONFIGS.md — Phase A configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the
branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

There are no runtime option structs, no mode flags, no `#ifdef`s and no
byte-order handling in this library. The axes are therefore the *data shapes*
and *values* the code special-cases:

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| `A1` entry point | `create_block`, `allocate_block`, `free_block`, `compute_hash`, `betagamma` | all 5 exported symbols; `betagamma` is the only one in `include/lib.h`, the other four are the **low-level** entry points and are driven directly |
| `A2` `create_block` name length | `0`, `1`, mid, `31` (max non-overflowing) | `strcpy` into `char[32]`, line 42 |
| `A3` `create_block` flags | all 256 `uint8_t` values | field is copied verbatim, line 44 |
| `A4` `create_block` id | `0`, `±1`, `INT_MIN`, `INT_MAX`, random | field copied verbatim, line 41 |
| `A5` `allocate_block` count | `0`, `1`, `2`, small (5–14, the range `betagamma` uses), large (`1<<16`) | `calloc` + the `for` loop, lines 52–62 |
| `A6` `allocate_block` init_value | `0`, negative, `INT_MIN`, `INT_MAX` (wraps), random | `init_value + i` computed in `size_t`, truncated to `int`, line 61 |
| `A7` `compute_hash` `data` ordering | `d1 < d2`, `d1 > d2`, `d1 == d2` | 3-way branch, lines 79–83 |
| `A8` `compute_hash` struct-pointer ordering | `p1 < p2`, `p1 > p2`, `p1 == p2` | 3-way branch, lines 85–89 → 9 combos, all reachable with stack-built structs |
| `A9` `betagamma` `param1 % 10` | all 10 non-negative residues (block_size 5–14) and all 10 negative residues (block_size 5 down to −4) | `(param1 % 10) + 5`, line 126 |
| `A10` `betagamma` param2/3/4 | `0`, extremes, random full-`i32` (drives `flag_contribution` overflow + `(sum1-sum2)/10` sign) | lines 106–121, 143–150 |
| `A11` `betagamma` fixed block table | flags `0xAA`, `0xCC`, `0xF0` — the four mask tests `0x0F/0xF0/0xAA/0x55` select a **different subset of params per block**: block 1 (`0xAA`) adds `param1+param2+param3`; block 2 (`0xCC`) and block 3 (`0xF0`) add all four, since `0xCC & 0x55 == 0x44` and `0xF0 & 0x55 == 0x50` are both non-zero. So `flag_contribution` is weighted `1·(p1+p2+p3) + 2·(p1+p2+p3+p4) + 3·(p2+p3+p4)` (block 3 fails the `0x0F` test). This asymmetry is fixed by the source, not an input axis, but every row below exercises it. | lines 94–123 |

## Rows (pruned cross-product of the axes the C actually distinguishes)

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|------------------------------------------|------|-----|
| C1 | `create_block` | name length 0 (`""`), flags swept 0–255, random ids incl. `INT_MIN`/`INT_MAX` | `cfg_c1_create_block_empty_name` | [x] |
| C2 | `create_block` | name length 1, all 256 flags, random ids | `cfg_c2_create_block_len1_name` | [x] |
| C3 | `create_block` | random name lengths 2–30, random bytes (incl. high-bit / non-ASCII), random flags + ids | `cfg_c3_create_block_random_names` | [x] |
| C4 | `create_block` | name length 31 = boundary that exactly fills `char[32]` with its NUL | `cfg_c4_create_block_len31_name` | [x] |
| C5 | `allocate_block` + `free_block` | `count = 0`; `init_value` random incl. extremes | `cfg_c5_allocate_zero_count` | [x] |
| C6 | `allocate_block` + `free_block` | `count = 1`; `init_value` random incl. `INT_MAX`/`INT_MIN` (wrap on `+i`) | `cfg_c6_allocate_count_one` | [x] |
| C7 | `allocate_block` + `free_block` | `count = 2..14` (the whole range `betagamma` can request); random `init_value` | `cfg_c7_allocate_small_counts` | [x] |
| C8 | `allocate_block` + `free_block` | `count = 65536` (many, forces mmap'd/large bin); `init_value = INT_MAX-3` so the buffer wraps mid-array | `cfg_c8_allocate_large_count_wrapping` | [x] |
| C9 | `compute_hash` | `d1 < d2` × each of `p1 < p2`, `p1 > p2`, `p1 == p2` (stack-built `MemoryBlock`s with synthesised `data` values) | `cfg_c9_compute_hash_data_less` | [x] |
| C10 | `compute_hash` | `d1 > d2` × each of the 3 struct-pointer orderings | `cfg_c10_compute_hash_data_greater` | [x] |
| C11 | `compute_hash` | `d1 == d2` × each of the 3 struct-pointer orderings (incl. `data == NULL` on both, which is read but not dereferenced) | `cfg_c11_compute_hash_data_equal` | [x] |
| C12 | `compute_hash` | randomized full-`usize` `data` addresses (0, 1, `usize::MAX`, random) — checks the comparison is **unsigned**, as C pointer relationals are | `cfg_c12_compute_hash_random_addresses` | [x] |
| C13 | `compute_hash` | real `allocate_block` output (the composed pipeline `betagamma` uses), both argument orders | `cfg_c13_compute_hash_real_allocations` | [x] |
| C14 | `betagamma` | `param1 % 10 == 0..9`, `param1 >= 0` → block_size 5..14; params 2–4 random | `cfg_c14_betagamma_positive_residues` | [x] |
| C15 | `betagamma` | `param1 < 0` with `|param1| % 10 ∈ {0..5}` → block_size 5,4,3,2,1,0 (all valid); params 2–4 random | `cfg_c15_betagamma_negative_valid_residues` | [x] |
| C16 | `betagamma` | all four params at every combination of `{INT_MIN, -1, 0, 1, INT_MAX}` (5^4 = 625) — drives `flag_contribution` overflow, `sum1-sum2` overflow, and `/10` truncation toward zero for negative dividends | `cfg_c16_betagamma_extreme_grid` | [x] |
| C17 | `betagamma` | fully randomized full-range `i32` quadruples (seeded), 20 000 cases | `cfg_c17_betagamma_random_fullrange` | [x] |
| C18 | `betagamma` | `param2` chosen so `sum1 - sum2` is exactly divisible by 10 vs. off by ±1..9, at each block_size — pins the truncating division | `cfg_c18_betagamma_division_boundaries` | [x] |
| C19 | composed pipeline | `allocate_block` → `compute_hash` → sum loops → `free_block` driven manually through the `.so` exports in the same order `betagamma` does, and cross-checked against `betagamma`'s own return | `cfg_c19_manual_pipeline_matches_betagamma` | [x] |
| C20 | all 5 entry points | interleaved call sequence (allocate/free churn between `betagamma` calls) so the two libraries see comparable heap evolution; verifies the address-dependent `compute_hash` contribution is stable | `cfg_c20_interleaved_heap_churn` | [x] |

## Harness notes (why the tests are shaped this way)

**Heap-address dependence.** `compute_hash` branches on the raw values of
`mb1`, `mb2`, `mb1->data` and `mb2->data`, so `betagamma`'s return value is a
function of *(inputs, global heap state)* — not of inputs alone. Calling C and
then Rust in one process compares them under two *different* heap states: the
first call perturbs the allocator for the second. Observed directly at the start
of verification — `betagamma(0,0,0,0)` gave C=464 / Rust=474, a delta of exactly
10, i.e. the `mb1 < mb2` vs `mb1 > mb2` term — while the same inputs in two
*fresh* processes both gave 464.

The fix is `fork()`: two children forked from the same instruction inherit
byte-identical heaps. One child runs the whole C batch, the other the whole Rust
batch, and because both implementations issue the same `malloc`/`calloc`/`free`
sequence with the same sizes, their heaps stay in lockstep for the entire batch.
This makes "the Rust performs an identical allocation sequence" part of what is
verified — mutation M20 (changing `calloc`'s element size from 4 to 8) and M17
(skipping the inner `free`) are both caught this way, purely through their
effect on subsequent addresses.

**Uninitialized bytes.** C's `create_block` declares `DataBlock block;` and
writes only three fields, so the 3 trailing padding bytes *and* every `name[]`
byte past the copied NUL hold stack garbage. Comparisons use `defined()`, which
compares `id`, `flags`, and `name` only up to and including the NUL. Comparing
the full 40 bytes would compare uninitialized memory and fail nondeterministically.

**Stale-`.so` guard.** `cargo test` does **not** rebuild a
`crate-type = ["cdylib"]` artifact, because integration tests `dlopen` the `.so`
rather than linking it. An early mutation run was silently vacuous for this
reason — all 7 injected bugs "passed" against a 13-minute-old library.
`tests/common/mod.rs` now refuses to run if the `.so` is older than any file
under `src/`, and `scripts/verify_all.sh` always builds before testing.

## Validation of the tests themselves

`scripts/mutation_test.sh` injects 20 deliberate bugs into `src/lib.rs`
(signed-vs-unsigned pointer comparison, floor-vs-truncating division, sign-
extended `flags`, dropped mask branches, saturating instead of wrapping
`init_value + i`, a null guard the C does not have, `NULL` for `count == 0`,
zero- instead of sign-extended `block_size`, altered hash constants, a skipped
`strcpy`, a skipped inner `free`, an inverted pointer-equality guard, a wrong
`calloc` element size, …) and confirms the suite catches each one.

**Result: 20/20 non-equivalent mutations caught.** Three further mutations are
*provably semantically equivalent* to the original and survive by necessity —
no test can distinguish them:

* mask `0x0F` → `0x0E`: no fixed flag byte (`0xAA`, `0xCC`, `0xF0`) has bit 0 set.
* block 2 flags `0xCC` → `0xCD`: both test non-zero against all four masks.
* `mem1->size` → `mem2->size` as a sum-loop bound: both blocks are allocated
  with the same `block_size`, so the two sizes are always equal.

Each is paired with a non-equivalent variant of the same expression
(`M12b`, `M13b`, `M19b`) that *is* caught, confirming the code path is covered.
