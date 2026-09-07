# CONFIGS.md — Phase B configuration-surface table

## Axes the C actually branches on

`c_src/src/lib.c` has no runtime options, no global state, no flags, no
`#ifdef`, no `switch`, and no compile-time configuration; `translation/Cargo.toml`
declares **no `[features]` section**, so there is exactly one feature
combination (the empty default). The configuration surface is therefore entirely
made of *input shapes*. Grepping every `if` / `else if` / `?:` in the C source
yields these axes:

| axis | source condition | distinct states |
|------|------------------|-----------------|
| A. divisor zero-ness | `if (v2 == 0)` (line 4) | `v2 == 0`, `v2 != 0` |
| B. dividend class | `if (v1 >= 0)` / `else if (v1 != (-0x7fffffff - 1))` | `v1 >= 0`, `INT_MIN < v1 < 0`, `v1 == INT_MIN` |
| C. divisor class | `if (v2 >= 0)` / `else if (v2 != (-0x7fffffff - 1))` (appears 3× — once per dividend class) | `v2 > 0`, `INT_MIN < v2 < 0`, `v2 == INT_MIN` |
| D. remainder sign | `if (r >= 0)` at the tail | `r >= 0` (exact, or a branch that can only produce non-negative `r`), `r < 0` |
| E. correction sign | `q + (v2 > 0 ? -1 : 1)` | `v2 > 0` → `-1`, `v2 < 0` → `+1` (fully determined by axis C) |
| F. magnitude shape | not branched on, but selects `q == 0` vs `q != 0` and exercises different divider code | `\|v1\| < \|v2\|`, `\|v1\| == \|v2\|`, `\|v1\| > \|v2\|`, `v1 == 0` |

Axis D is *not* free: several B×C leaves can only ever produce `r >= 0`
(the code assigns `r = v1` with `v1 >= 0`, or `r = 0`, or `r = v1 - INT_MIN > 0`),
so the cross-product is pruned to the combinations the code can actually reach.

## Public entry points

The full public API is the single lowest-level entry point
`int div_euclid(int v1, int v2)` (`c_src/include/lib.h:1`). There is no
convenience wrapper and no higher layer, so "exercising the low-level entry
points directly" and "driving it as a consumer does" are the same call. Both
sides are always invoked through their `.so` exports via `libloading`.

## Rows (pruned cross-product of B × C × D × F)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C01 | `div_euclid` | A: `v2 == 0`; B: all three dividend classes; F: all — early return path | [x] |
| C02 | `div_euclid` | B: `v1 >= 0`, C: `v2 > 0`, F: `v1 < v2` (quotient 0) — early `return v1/v2`, `r` never assigned | [x] |
| C03 | `div_euclid` | B: `v1 >= 0`, C: `v2 > 0`, F: `v1 == v2` | [x] |
| C04 | `div_euclid` | B: `v1 >= 0`, C: `v2 > 0`, F: `v1 > v2`, exactly divisible | [x] |
| C05 | `div_euclid` | B: `v1 >= 0`, C: `v2 > 0`, F: `v1 > v2`, NOT divisible | [x] |
| C06 | `div_euclid` | B: `v1 == 0`, C: `v2 > 0` | [x] |
| C07 | `div_euclid` | B: `v1 >= 0`, C: `INT_MIN < v2 < 0`, F: `\|v1\| < \|v2\|` → `q = 0`, `r = v1 >= 0` | [x] |
| C08 | `div_euclid` | B: `v1 >= 0`, C: `INT_MIN < v2 < 0`, F: exactly divisible → `r == 0`, `return q` | [x] |
| C09 | `div_euclid` | B: `v1 >= 0`, C: `INT_MIN < v2 < 0`, F: NOT divisible → `r > 0`, still `return q` (D can never be `<0` here) | [x] |
| C10 | `div_euclid` | B: `v1 == 0`, C: `INT_MIN < v2 < 0` | [x] |
| C11 | `div_euclid` | B: `v1 > 0`, C: `v2 == INT_MIN` → `q = 0, r = v1`, `r >= 0` | [x] |
| C12 | `div_euclid` | B: `v1 == 0`, C: `v2 == INT_MIN` → `q = 0, r = 0` | [x] |
| C13 | `div_euclid` | B: `INT_MIN < v1 < 0`, C: `v2 > 0`, D: `r == 0` (exactly divisible) → `return q` | [x] |
| C14 | `div_euclid` | B: `INT_MIN < v1 < 0`, C: `v2 > 0`, D: `r < 0` → `return q - 1` (E: `v2 > 0`) | [x] |
| C15 | `div_euclid` | B: `INT_MIN < v1 < 0`, C: `v2 > 0`, F: `\|v1\| < v2` → `q = 0`, `r < 0` → `-1` | [x] |
| C16 | `div_euclid` | B: `INT_MIN < v1 < 0`, C: `INT_MIN < v2 < 0`, D: `r == 0` → `return q` | [x] |
| C17 | `div_euclid` | B: `INT_MIN < v1 < 0`, C: `INT_MIN < v2 < 0`, D: `r < 0` → `return q + 1` (E: `v2 < 0`) | [x] |
| C18 | `div_euclid` | B: `INT_MIN < v1 < 0`, C: `INT_MIN < v2 < 0`, F: `\|v1\| < \|v2\|` → `q = 0`, `r < 0` → `+1` | [x] |
| C19 | `div_euclid` | B: `INT_MIN < v1 < 0`, C: `v2 == INT_MIN` → `q = 1, r = v1 - 1*INT_MIN > 0` → `return 1` | [x] |
| C20 | `div_euclid` | B: `v1 == INT_MIN`, C: `v2 > 0`, D: `r == 0` (`v2` divides `INT_MIN`, e.g. powers of two) → `return q` | [x] |
| C21 | `div_euclid` | B: `v1 == INT_MIN`, C: `v2 > 0`, D: `r < 0` → `return q - 1` | [x] |
| C22 | `div_euclid` | B: `v1 == INT_MIN`, C: `v2 == 1` (boundary: `-(v1+v2) = INT_MAX`) | [x] |
| C23 | `div_euclid` | B: `v1 == INT_MIN`, C: `v2 == INT_MAX` (boundary: `-(v1+v2) = 1`) | [x] |
| C24 | `div_euclid` | B: `v1 == INT_MIN`, C: `INT_MIN < v2 < 0`, D: `r == 0` → `return q` | [x] |
| C25 | `div_euclid` | B: `v1 == INT_MIN`, C: `INT_MIN < v2 < 0`, D: `r < 0` → `return q + 1` | [x] |
| C26 | `div_euclid` | B: `v1 == INT_MIN`, C: `v2 == -1` (boundary: `-(v1-v2) = INT_MAX`, quotient overflows `int`) | [x] |
| C27 | `div_euclid` | B: `v1 == INT_MIN`, C: `v2 == INT_MIN + 1` (boundary: `-(v1-v2) = 1`) | [x] |
| C28 | `div_euclid` | B: `v1 == INT_MIN`, C: `v2 == INT_MIN` → `q = 1, r = 0` → `return 1` | [x] |
| C29 | `div_euclid` | Full unrestricted domain: uniformly random `(v1, v2)` over all of `i32 × i32`, fixed seed | [x] |
| C30 | `div_euclid` | Exhaustive dense neighbourhood: every `(v1, v2)` in `[-256, 256]²` (263 169 pairs) | [x] |
| C31 | `div_euclid` | Exhaustive boundary cross-product: every `(v1, v2)` from the extreme/near-extreme value set (`INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`, powers of two ±1, …) | [x] |
| C32 | `div_euclid` | `v1 == INT_MAX` / `v1 == INT_MAX - 1` against every divisor class | [x] |
| C33 | `div_euclid` | Random inputs biased toward `\|v1\|`/`\|v2\|` near the `i32` extremes (high-bit-heavy generator) | [x] |
| C34 | `div_euclid` | **Exhaustive**: `v1 ∈ {INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX}` × **every one of the 2^32 divisors** (2.58×10^10 comparisons) | [x] |
| C35 | `div_euclid` | **Exhaustive**: **every one of the 2^32 dividends** × `v2 ∈ {INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX}` (2.58×10^10 comparisons) | [x] |

## Binary executable

Neither build produces a driver binary: `c_src/CMakeLists.txt` has only
`add_library(... SHARED src/lib.c)` (no `add_executable`), and
`translation/Cargo.toml` has only `[lib] crate-type = ["cdylib"]` with no
`[[bin]]` target and no `src/main.rs`. There is therefore no stdout to compare
in Phase B.

## Feature combinations

`translation/Cargo.toml` declares no `[features]`, so the complete set of
combinations is `{ default (empty) }` plus the degenerate
`--no-default-features`, both of which compile the identical single code path.
Both are run by `run_all.sh`.
