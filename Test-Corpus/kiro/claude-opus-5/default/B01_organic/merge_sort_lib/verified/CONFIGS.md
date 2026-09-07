# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the branches the C code actually takes.

## Axes the C code branches on

There are **no** runtime options, modes, flags, or `#ifdef`s: `grep -c
'#if\|#ifdef\|#define' c_src/src/lib.c c_src/include/lib.h` finds none, and
`merge_sort(a, b, size)` has no options parameter. The entire configuration
surface is therefore *input shape*, on the following axes, each read off a real
branch in the source:

| axis | values the C distinguishes | branch that distinguishes them |
|---|---|---|
| **A. `size`** | `0`, `1` (base case) vs `>= 2` (recurses); even vs odd (`split = (lo+hi)/2` splits unevenly on odd runs); exact powers of two vs not; deep recursion | `lib.c:33` `if (hi - lo <= 1)`, `lib.c:35` `split = (lo + hi) / 2` |
| **B. `sort_bits` ordering** | already ascending, already descending, all equal, few distinct values (many ties), fully random | `lib.c:7` `a->sort_bits <= b->sort_bits` |
| **C. `sort_bits` magnitude** | non-negative only, negative only, mixed sign, `INT_MIN`/`INT_MAX` present | `lib.c:7` — signed compare |
| **D. `texture_id`** | all equal, all distinct ascending, all distinct **descending while `sort_bits` tie** (probes the dead tiebreak), `0`/`u64::MAX` extremes | `lib.c:9` — the dead branch |
| **E. run-exhaustion order** | left run exhausts first (`i >= split`), right run exhausts first (`j >= hi`), both alternate | `lib.c:19-21` `i < split && (j >= hi \|\| leq(...))` |
| **F. scratch buffer `b`** | pre-zeroed, pre-filled with distinct garbage | `merge_sort` overwrites `b` via `memcpy`, then `b` is read *and* written by alternating recursion levels — `b`'s final contents are part of the output |
| **G. struct padding** | padding zeroed, padding filled with non-zero garbage | the 16-byte `memcpy` and the 16-byte struct assignment both carry padding |

## Entry points

`merge_sort` is the **only** public entry point (`SYMBOLS.md`); the three
`spritebatch_internal_*` functions are `static` and unreachable through the
`.so`. They are exercised transitively, and each row below names which internal
path it drives. There is no convenience-wrapper / low-level split to worry
about: `merge_sort` *is* the lowest-level exported call.

There is no binary/driver target in `c_src/CMakeLists.txt` (only
`add_library(... SHARED ...)`) and none in `translation/Cargo.toml` (`crate-type
= ["cdylib"]`, no `[[bin]]`), so the "compare stdout of C and Rust binaries"
requirement is **not applicable**.

## Configuration table

Every row is run with **many randomized inputs** (`RNG_SEED = 0x5EED_1234_ABCD_F00D`,
a fixed-seed SplitMix64/xorshift so runs are reproducible), and asserts that
**both** the sorted buffer `a` *and* the scratch buffer `b` are byte-identical
between C and Rust — all `16 * size` bytes, padding included.

| # | entry point(s) | configuration (options set + input shape) | internal path driven | [x] |
|---|---|---|---|---|
| 1 | `merge_sort` | `size = 0`; `b` garbage-filled | base case only, no merge | [x] |
| 2 | `merge_sort` | `size = 1`; `b` garbage-filled | base case only, `memcpy` of 1 elem | [x] |
| 3 | `merge_sort` | `size = 2`; random `sort_bits` | one `iteration`, both runs length 1 | [x] |
| 4 | `merge_sort` | `size = 3` (odd → uneven split 1/2) | nested recursion, uneven runs | [x] |
| 5 | `merge_sort` | `size = 4` (power of two, even splits) | balanced recursion | [x] |
| 6 | `merge_sort` | `size = 5,6,7` (odd/even non-power-of-two) | uneven splits at several levels | [x] |
| 7 | `merge_sort` | `size = 8, 16, 32, 64, 128, 256` (powers of two) | fully balanced, buffer roles alternate cleanly | [x] |
| 8 | `merge_sort` | `size = 9, 17, 33, 63, 65, 127, 129, 255, 257` (±1 off powers of two) | worst-case uneven splits / deepest ragged recursion | [x] |
| 9 | `merge_sort` | `size = 1000`, random | deep recursion (~10 levels) | [x] |
| 10 | `merge_sort` | `size = 4096`, random | deep recursion (12 levels), large `memcpy` | [x] |
| 11 | `merge_sort` | random `size` in `1..=512`, random data (200 iterations) | broad property sweep over axes A+B | [x] |
| 12 | `merge_sort` | already **ascending** `sort_bits` (sorted input), sizes 2..=64 | `leq` always true → left run always taken until exhausted (axis E: left-exhausts-first) | [x] |
| 13 | `merge_sort` | already **descending** `sort_bits` (reverse sorted), sizes 2..=64 | `leq` false on first compare → right run drains first (axis E: right-exhausts-first) | [x] |
| 14 | `merge_sort` | **all `sort_bits` equal**, distinct `texture_id`s ascending | every compare hits `lib.c:7` true; probes stability + dead branch (axis B+D) | [x] |
| 15 | `merge_sort` | **all `sort_bits` equal**, `texture_id`s **descending** | the exact input the dead `texture_id` tiebreak would have reordered; C must leave order as-is (axis D) | [x] |
| 16 | `merge_sort` | few distinct `sort_bits` (2 / 3 / 5 buckets) → **many ties**, random `texture_id` | heavy tie traffic through `lib.c:7`/`lib.c:9` (axis B+D) | [x] |
| 17 | `merge_sort` | `sort_bits` **all negative** | signed compare on negatives (axis C) | [x] |
| 18 | `merge_sort` | `sort_bits` **mixed sign**, full `i32` range random | signed compare across zero (axis C) | [x] |
| 19 | `merge_sort` | `sort_bits` drawn only from `{INT_MIN, -1, 0, 1, INT_MAX}` | signed boundary values, guaranteed ties (axis C) | [x] |
| 20 | `merge_sort` | `texture_id` drawn only from `{0, 1, u64::MAX/2, u64::MAX-1, u64::MAX}`, `sort_bits` all equal | unsigned extremes in the dead branch (axis D) | [x] |
| 21 | `merge_sort` | scratch `b` **pre-zeroed** vs `b` **pre-filled with a distinct pattern**, same `a`; sizes 0..=32 | `b`'s final contents differ only where the algorithm writes; catches any missing write (axis F) | [x] |
| 22 | `merge_sort` | struct **padding bytes non-zero** (0xAA pattern) in `a`, `b` padding 0x55; sizes 1..=32 | 16-byte copy must carry padding through both `memcpy` and struct assignment (axis G) | [x] |
| 23 | `merge_sort` | padding zeroed everywhere (baseline control for row 22) | same, padding must stay zero | [x] |
| 24 | `merge_sort` | **repeated / idempotent** invocation: call `merge_sort` twice on the same buffers | second call re-`memcpy`s the already-sorted `a`; both libs must agree on both buffers after each call | [x] |
| 25 | `merge_sort` | full cross-product sweep: every `size` in `0..=80` × each of the 6 data shapes (ascending, descending, all-equal, few-buckets, random, boundary-values), padding garbage on | pruned cross-product of axes A×B×C×G — the interaction sweep | [x] |
| 26 | `merge_sort` | `a` and `b` **adjacent in one allocation** (`b` immediately follows `a`), sizes 1..=64 | non-overlapping but contiguous; catches an off-by-one write past `size` in either buffer (axes A+F) | [x] |

## Verification record

All 26 rows pass. Every row is exercised through `libloading` against both
`.so`s — the Rust side is always reached via its `#[no_mangle] extern "C"`
export, never as a direct Rust call.

* `tests/phase_b_valid_paths.rs` — 26 tests, one per row, all passing in both
  `cargo test` (debug) and `cargo test --release`.
* Fixed RNG seed `0x5EED_1234_ABCD_F00D`, so every reported input is
  reproducible.
* Each row asserts **both** output buffers byte-for-byte over all `16 * size`
  bytes, padding included — not just the sorted array.
* Also re-run green against the C compiled at `-O0`, `-O1`, `-O2`, `-O3` and
  `-Os` (via `HARVEST_C_SO=<path> cargo test --release`), which confirms the
  16-byte struct copy is what gcc emits at every optimisation level, not an
  artefact of the unoptimised default CMake build.

### Suite adequacy (`./mutation_check.sh`)

Passing tests only prove something if the tests can fail. 26 single-edit mutants
were injected into `translation/src/lib.rs`, one per branch and boundary in the
C algorithm (comparison direction and signedness, struct-copy width, both merge
loop bounds, the `j >= hi` short-circuit, both index increments, the recursion
base case, the split computation, the buffer-role alternation, and the entry
`memcpy`). **All 26 were killed.** The script refuses mutations that only touch
comments, which is how two initially "surviving" mutants turned out to be
no-ops rather than blind spots.
