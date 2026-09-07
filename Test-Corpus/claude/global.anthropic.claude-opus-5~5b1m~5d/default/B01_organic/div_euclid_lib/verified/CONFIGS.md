# CONFIGS.md — Phase B configuration surface table

Derived mechanically from the branch structure the C actually takes in
`c_src/src/lib.c`. There is exactly **one** public entry point and it is also the
lowest-level one (`c_src/include/lib.h` declares only `int div_euclid(int, int)`),
so "low-level vs convenience wrapper" collapses: every row below drives the real
entry point directly through the `.so`.

## Axes the C branches on

Enumerated from the `if` / `else if` / `?:` conditions in the source (there are no
`#ifdef`s, no global state, no runtime option/mode/flag setters, no build-time
options, and no `[features]` in `Cargo.toml`):

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| A1 `v2` vs zero | `v2 == 0`, `v2 != 0` | line 4 |
| A2 sign of `v1` | `v1 >= 0`, `v1 < 0` | line 8 |
| A3 `v1` vs `INT_MIN` | `v1 == INT_MIN`, `v1 != INT_MIN` (only reachable when `v1 < 0`) | line 15 |
| A4 sign of `v2` | `v2 >= 0` (i.e. `v2 > 0` after A1), `v2 < 0` | lines 9, 16, 22 |
| A5 `v2` vs `INT_MIN` | `v2 == INT_MIN`, `v2 != INT_MIN` (only reachable when `v2 < 0`) | lines 11, 18, 24 |
| A6 sign of computed `r` | `r >= 0` (return `q`), `r < 0` (adjust) | line 28 |
| A7 sign of `v2` in epilogue | `v2 > 0` ⇒ `-1`, `v2 < 0` ⇒ `+1` | line 31 |
| A8 divisibility | `v1 % v2 == 0` vs `!= 0` — decides A6 on most paths | lines 12/17/19/23/25 |
| A9 magnitude shape | `|v1| < |v2|` (quotient 0), `|v1| == |v2|`, `|v1| > |v2|`; `v2 == ±1`; boundary values `INT_MAX`, `INT_MIN+1`, `0`, `±1`, `±2` | value-dependent, exercised across all paths |

## Enumerated code paths (the pruned cross product)

Ten mutually exclusive assignment paths exist; `P1` returns early and skips the
epilogue, the other nine flow into the A6/A7 epilogue.

| path | condition | body |
|------|-----------|------|
| P0 | `v2 == 0` | `return 0` |
| P1 | `v1 >= 0`, `v2 > 0` | `return v1 / v2` (early return, no epilogue) |
| P2 | `v1 >= 0`, `v2 < 0`, `v2 != INT_MIN` | `q = -(v1 / -v2)`, `r = v1 % -v2` |
| P3 | `v1 >= 0`, `v2 == INT_MIN` | `q = 0`, `r = v1` |
| P4 | `v1 < 0`, `v1 != INT_MIN`, `v2 > 0` | `q = -((-v1)/v2)`, `r = -((-v1)%v2)` |
| P5 | `v1 < 0`, `v1 != INT_MIN`, `v2 < 0`, `v2 != INT_MIN` | `q = (-v1)/(-v2)`, `r = -((-v1)%(-v2))` |
| P6 | `v1 < 0`, `v1 != INT_MIN`, `v2 == INT_MIN` | `q = 1`, `r = v1 - q*v2` |
| P7 | `v1 == INT_MIN`, `v2 > 0` | `q = -((-(v1+v2))/v2) - 1`, `r = -((-(v1+v2))%v2)` |
| P8 | `v1 == INT_MIN`, `v2 < 0`, `v2 != INT_MIN` | `q = ((-(v1-v2))/(-v2)) + 1`, `r = -((-(v1-v2))%(-v2))` |
| P9 | `v1 == INT_MIN`, `v2 == INT_MIN` | `q = 1`, `r = 0` |

## Configuration table

One row per meaningful combination of path × epilogue branch × input shape. Every
row is driven with **many randomized inputs** (fixed seed `0x5EED_1234`,
SplitMix64) constrained to that row's region, plus that row's boundary values.
All rows call both `.so`s and compare the returned `int` byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1  | `div_euclid` | P0: `v2 == 0`, `v1` randomized over full `i32` (+ boundary `v1`s) | [x] |
| C2  | `div_euclid` | P1: `v1 > 0`, `v2 > 0`, `v1 < v2` (quotient 0), randomized | [x] |
| C3  | `div_euclid` | P1: `v1 > 0`, `v2 > 0`, `v1 > v2`, exactly divisible (`r == 0`), randomized | [x] |
| C4  | `div_euclid` | P1: `v1 > 0`, `v2 > 0`, `v1 > v2`, not divisible, randomized | [x] |
| C5  | `div_euclid` | P1: `v1 == 0`, `v2 > 0`; and `v1 == v2`; and `v2 == 1`; and `v1 == INT_MAX` / `v2 == INT_MAX` | [x] |
| C6  | `div_euclid` | P2: `v1 > 0`, `v2 < 0`, `v2 != INT_MIN`, `r == 0` (divisible) → epilogue A6=`r>=0` | [x] |
| C7  | `div_euclid` | P2: `v1 > 0`, `v2 < 0`, `v2 != INT_MIN`, `r > 0` (not divisible) → epilogue A6=`r>=0` | [x] |
| C8  | `div_euclid` | P2: `v1 == 0` or `|v1| < |v2|`, `v2 < 0` (quotient 0); and `v2 == -1`; and `v1 == INT_MAX`, `v2 == INT_MIN+1` | [x] |
| C9  | `div_euclid` | P3: `v1 >= 0`, `v2 == INT_MIN`; `v1` randomized in `[0, INT_MAX]` + `{0, 1, INT_MAX}` | [x] |
| C10 | `div_euclid` | P4: `v1 < 0` (`!= INT_MIN`), `v2 > 0`, divisible → `r == 0`, epilogue returns `q` | [x] |
| C11 | `div_euclid` | P4: `v1 < 0` (`!= INT_MIN`), `v2 > 0`, not divisible → `r < 0`, epilogue A7 `v2>0` ⇒ `q-1` | [x] |
| C12 | `div_euclid` | P4: `|v1| < v2` (quotient 0, `r < 0`); `v2 == 1`; `v1 == INT_MIN+1`; `v2 == INT_MAX` | [x] |
| C13 | `div_euclid` | P5: `v1 < 0` (`!= INT_MIN`), `v2 < 0` (`!= INT_MIN`), divisible → `r == 0` ⇒ `q` | [x] |
| C14 | `div_euclid` | P5: `v1 < 0` (`!= INT_MIN`), `v2 < 0` (`!= INT_MIN`), not divisible → `r < 0`, epilogue A7 `v2<0` ⇒ `q+1` | [x] |
| C15 | `div_euclid` | P5: `|v1| < |v2|` (quotient 0); `v2 == -1` (⇒ `q == -v1`, near-overflow); `v1 == INT_MIN+1`, `v2 == INT_MIN+1` | [x] |
| C16 | `div_euclid` | P6: `v1 < 0` (`!= INT_MIN`), `v2 == INT_MIN`; `v1` randomized in `[INT_MIN+1, -1]` + `{-1, -2, INT_MIN+1}` | [x] |
| C17 | `div_euclid` | P7: `v1 == INT_MIN`, `v2 > 0`, divisible (`r == 0`, power-of-two and general divisors) | [x] |
| C18 | `div_euclid` | P7: `v1 == INT_MIN`, `v2 > 0`, not divisible → `r < 0` ⇒ `q - 1` | [x] |
| C19 | `div_euclid` | P7: `v1 == INT_MIN`, `v2 ∈ {1, 2, 3, INT_MAX-1, INT_MAX}` (boundary divisors) | [x] |
| C20 | `div_euclid` | P8: `v1 == INT_MIN`, `v2 < 0` (`!= INT_MIN`), divisible | [x] |
| C21 | `div_euclid` | P8: `v1 == INT_MIN`, `v2 < 0` (`!= INT_MIN`), not divisible | [x] |
| C22 | `div_euclid` | P8: `v1 == INT_MIN`, `v2 ∈ {-1, -2, -3, INT_MIN+1, INT_MIN+2}` (boundary divisors incl. the `INT_MIN / -1` trap input) | [x] |
| C23 | `div_euclid` | P9: `v1 == INT_MIN`, `v2 == INT_MIN` (single point) | [x] |
| C24 | `div_euclid` | Unconstrained: both operands uniformly random over the full `i32` range, large sample count (path mix decided by data) | [x] |
| C25 | `div_euclid` | Unconstrained: both operands drawn from a boundary-biased distribution (extremes, powers of two, ±1 neighbourhoods of `0`/`INT_MIN`/`INT_MAX`) | [x] |
| C26 | `div_euclid` | Full cross product of the 9-value boundary grid `{INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, INT_MAX-1, INT_MAX}` × itself (81 pairs) | [x] |
| C27 | `div_euclid` | Exhaustive small-value cross product: `v1, v2 ∈ [-300, 300]` (361 201 pairs, covers all sign/divisibility/quotient-0 combinations densely) | [x] |
| C28 | `div_euclid` | Exhaustive sweep of **every** `i32` value of `v2` with `v1` pinned to each of `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX}` (subsumes out-of-range "enum-like" ints) | [x] |
| C29 | `div_euclid` | Exhaustive sweep of **every** `i32` value of `v1` with `v2` pinned to each of `{INT_MIN, INT_MIN+1, -3, -2, -1, 0, 1, 2, 3, INT_MAX}` | [x] |
| C30 | `div_euclid` | Stride sweep over the whole 2-D space: a coprime trajectory visiting 40 M distinct `(v1, v2)` pairs, plus a coarse true grid (~512 points/axis) so both axes vary independently | [x] |
| C31 | `div_euclid` | Long randomized soak: 5 M uniform pairs + 1 M boundary-biased pairs | [x] |

## Where each row lives

* C1–C26 → `tests/configs.rs` (one `#[test]` per row, names `c1_…` … `c26_…`)
* C27–C31 → `tests/exhaustive.rs`

Actual measured coverage of the heavy rows (release `.so`, one run, 287 s):
C28 = 6 pins × 2^32 values and C29 = 10 pins × 2^32 values, i.e. **68.7 billion
`(v1, v2)` pairs** compared C-vs-Rust with **0 divergences**. `EXHAUSTIVE_STRIDE=<n>`
reduces these to a strided subset for quick runs; the default is `1` (exhaustive).

## Non-vacuity of these rows

`tests/mutation_check.sh` is the negative control: it generates six broken copies
of `src/lib.rs`, loads each as the "Rust" `.so`, and requires the suite to fail.
Result: mutants 1–5 (P8 quotient adjustment, epilogue sign, P3 `q`, the `v2 == 0`
sentinel, P6 remainder) are each caught by 4–12 tests. Mutant 6 replaces the whole
algorithm with `i32::wrapping_div_euclid` and is *correctly* not caught — the C
function is provably exactly wrapping Euclidean division with `0` for `v2 == 0`,
confirmed against that independent oracle over 23.7 M pairs
(`independent_oracle_agrees_with_both` in `tests/differential.rs`).

## Binary / driver

`c_src/CMakeLists.txt` defines only `add_library(... SHARED src/lib.c)` — **no
executable target** — and `translation/Cargo.toml` has only `[lib]` with
`crate-type = ["cdylib"]` (no `[[bin]]`, no `src/main.rs`). There is no binary
whose stdout could be compared, so that checklist item is not applicable.

## Feature combinations

`translation/Cargo.toml` contains no `[features]` table, so the complete set of
feature combinations is `{default}` = `{no-default-features}`. `tests/feature_sweep.sh`
enumerates the features from `Cargo.toml` and re-runs the whole suite for each
combination it finds, so this is verified mechanically rather than assumed.
