# CONFIGS.md — Configuration-surface table (Phase A → gate for Phase B)

Derived mechanically from the C sources, headers and `CMakeLists.txt`.

## Build-time configuration axes

`c_src/CMakeLists.txt` declares:

* `add_library(driver SHARED src/driver.c)` — one target, one translation unit
* include dirs only; **no** `option()`, **no** `target_compile_definitions`,
  **no** `CMAKE_BUILD_TYPE` branches, **no** generator expressions

`c_src/src/driver.c` / `c_src/include/driver.h` contain **no** `#ifdef` other
than the `DRIVER_H_` include guard.

→ **exactly one build configuration.** No conditional compilation to mirror.

`Cargo.toml` has **no `[features]` section**, so the Rust side likewise has
exactly one configuration. The full enumeration of valid feature combinations is:

| # | feature combination | cargo invocation | status |
|---|---------------------|------------------|--------|
| 1 | `{}` (default, which is empty) | `cargo check` / `cargo test` | ✅ checks & tests clean |
| 2 | `{}` (explicitly no defaults — identical set) | `cargo check --no-default-features` / `cargo test --no-default-features` | ✅ checks & tests clean |

There is no third combination: with no declared features and no `default` list,
the power set of features is `{ {} }`. No `#[cfg(feature = "...")]` gating is
required or possible.

The one `cfg` axis that *does* exist in `src/lib.rs` is
`#[cfg(target_arch = "x86_64")]` vs `#[cfg(not(target_arch = "x86_64"))]` for
`c_div`. That is a target axis, not a feature axis; the host is x86-64, so the
`idiv` path is the one under test (`cfg_x86_64_idiv_path_is_active`
documents which arm is live).

## Runtime configuration axes

The complete public API is `include/driver.h`:

```c
void driver(int x, int y);
```

One entry point, no initialisation, no handle/context, no setters, no global
state, no option flags, no modes, no byte-order or format selectors. `driver`
*is* both the lowest-level and the only entry point — there is no convenience
wrapper to mistake for the real API.

The axes the code actually distinguishes are therefore purely **input shape**,
via the two branch-free operations it performs:

* `div(x, y)` → glibc: `quot = x / y`, `rem = x % y`, plus the fix-up branch
  `if (numer >= 0 && result.rem < 0) { ++quot; rem -= denom; }`
* `printf("quotient: %d, remainder: %d\n", quot, rem)` → `%d` formatting

Axis A — sign of `x`: negative, zero, positive
Axis B — sign of `y`: negative, positive (`y == 0` ⇒ `ERRORS.md` row 1)
Axis C — magnitude relation: `|x| < |y|` (quot 0), `|x| == |y|` (quot ±1), `|x| > |y|`
Axis D — divisibility: `x % y == 0` (rem 0) vs `x % y != 0`
Axis E — boundary values: `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`
Axis F — `%d` output shape: single digit, many digits, `-` sign, widest output (`-2147483648`)

## Configuration-surface table

One row per meaningful combination the C treats differently (cross-product of
A×B×C×D pruned to distinct code paths, plus the boundary/format rows).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | `x > 0`, `y > 0`, `|x| > |y|`, not divisible → quot > 0, rem > 0 | [x] |
| 2 | `driver` | `x > 0`, `y > 0`, `|x| > |y|`, exactly divisible → quot > 0, rem == 0 | [x] |
| 3 | `driver` | `x < 0`, `y > 0`, `|x| > |y|`, not divisible → quot < 0, rem < 0 (truncation toward zero) | [x] |
| 4 | `driver` | `x < 0`, `y > 0`, `|x| > |y|`, exactly divisible → quot < 0, rem == 0 | [x] |
| 5 | `driver` | `x > 0`, `y < 0`, `|x| > |y|`, not divisible → quot < 0, rem > 0 | [x] |
| 6 | `driver` | `x > 0`, `y < 0`, `|x| > |y|`, exactly divisible → quot < 0, rem == 0 | [x] |
| 7 | `driver` | `x < 0`, `y < 0`, `|x| > |y|`, not divisible → quot > 0, rem < 0 | [x] |
| 8 | `driver` | `x < 0`, `y < 0`, `|x| > |y|`, exactly divisible → quot > 0, rem == 0 | [x] |
| 9 | `driver` | `x == 0`, `y != 0` (both signs of `y`) → quot == 0, rem == 0 | [x] |
| 10 | `driver` | `|x| < |y|` (all four sign combinations) → quot == 0, rem == x | [x] |
| 11 | `driver` | `|x| == |y|` (all four sign combinations) → quot == ±1, rem == 0 | [x] |
| 12 | `driver` | `x >= 0` with `y` of either sign — probes glibc's dead `numer >= 0 && rem < 0` fix-up branch; Rust must agree that it never fires | [x] |
| 13 | `driver` | `y == 1` (identity) and `y == -1` with `x != INT_MIN` (negation) | [x] |
| 14 | `driver` | `x == INT_MIN` with `y ∈ {1, 2, -2, INT_MIN, INT_MAX}` — widest negative `%d` output, extreme quotients | [x] |
| 15 | `driver` | `x == INT_MAX` with `y ∈ {1, -1, 2, -2, INT_MAX, INT_MIN}` — widest positive `%d` output | [x] |
| 16 | `driver` | `x == INT_MIN`/`INT_MAX` as *divisor* with small dividends → quot == 0, rem == x | [x] |
| 17 | `driver` | full boundary cross-product: `x, y ∈ {INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}`, all 49 pairs (the 8 trapping pairs routed through the worker-child harness) | [x] |
| 18 | `driver` | randomized full-range 32-bit `x` and `y` (`y != 0`, excluding the `INT_MIN/-1` trap), fixed-seed PRNG, 20 000 pairs | [x] |
| 19 | `driver` | randomized *small* magnitudes (`x, y ∈ [-64, 64]`, `y != 0`), fixed seed — dense coverage of sign/divisibility interactions | [x] |
| 20 | `driver` | repeated / interleaved calls in one process (C then Rust then C …) — verifies no hidden state and identical `stdout` buffering behaviour across a stream of calls | [x] |
| 21 | `driver` | many calls without an intervening flush, then one flush — verifies both sides append to the shared `stdout` `FILE` identically (buffering parity, not just per-call parity) | [x] |

All 21 rows are exercised against **both** feature combinations from the table
above.

## Row → test mapping (Phase B results)

Every row is covered by a test in `tests/differential_valid.rs` that loads both
`.so`s through `libloading` and compares the `stdout` bytes.

| row(s) | test | inputs exercised |
|---|---|---|
| 1 | `row01_pos_pos_inexact` | 500 randomized |
| 2 | `row02_pos_pos_exact` | 500 randomized |
| 3 | `row03_neg_pos_inexact` | 500 randomized |
| 4 | `row04_neg_pos_exact` | 500 randomized |
| 5 | `row05_pos_neg_inexact` | 500 randomized |
| 6 | `row06_pos_neg_exact` | 500 randomized |
| 7 | `row07_neg_neg_inexact` | 500 randomized |
| 8 | `row08_neg_neg_exact` | 500 randomized |
| 1–8 | `rows01_08_cover_all_quotient_remainder_sign_combinations` | 8 hand-checked expected `(quot, rem)` values |
| 9 | `row09_zero_dividend` | 406 (6 fixed + 400 randomized divisors) |
| 10 | `row10_dividend_smaller_than_divisor` | 800 randomized (200 per quadrant) |
| 11 | `row11_equal_magnitudes` | ~805 randomized + extremes |
| 12 | `row12_nonnegative_dividend_fixup_branch_never_fires` | 1005 randomized, plus the `rem >= 0` and `quot*y + rem == x` invariants |
| 13 | `row13_unit_divisors` | ~816 randomized + fixed |
| 14 | `row14_int_min_dividend` | 12 divisors + 3 exact-byte assertions |
| 15 | `row15_int_max_dividend` | 13 divisors + 2 exact-byte assertions |
| 16 | `row16_extreme_divisors` | 640 (4 extreme divisors × (10 fixed + 150 randomized)) |
| 17 | `row17_full_boundary_matrix` | all 49 boundary pairs; 41 compared, 8 trap-compared |
| 18 | `row18_random_full_range` | 20 000 randomized full-range pairs |
| 19 | `row19_random_small_magnitudes` | 4 161 exhaustive (\|x\|,\|y\| ≤ 32) + 3 000 randomized |
| 20 | `row20_interleaved_calls_are_stateless` | 600 pairs × both call orders in one process, + repeat-call check |
| 21 | `row21_unflushed_multi_call_stream` | 800-call buffered stream vs unbuffered stream, + 400-line mixed stream |
| format | `output_format_is_byte_exact` | 10 pairs asserted against literal expected bytes |
| — | `harness_detects_differences` | negative control: proves the harness can see a difference |
| cfg | `cfg_x86_64_idiv_path_is_active` | records which `c_div` arm is compiled in |

Total: **26 tests**, all passing. Roughly 37 000 differential `driver` calls per
profile.

## Validation that these rows are not vacuous (mutation testing)

The Rust source was temporarily mutated and the suite re-run; `src/lib.rs` was
restored byte-identically afterwards. Every behaviour-changing mutation was
caught by the rows that should catch it:

| mutation | caught by |
|---|---|
| A — `"quotient: "` → `"Quotient: "` in the format string | `output_format_is_byte_exact` + every comparison row |
| B — disable the `idiv` arm so the portable `wrapping_div`/`wrapping_rem` arm is used (values identical, but both `SIGFPE` traps vanish) | `test_err_row1_divide_by_zero_sigfpe`, `test_err_row2_int_min_div_neg_one_sigfpe`, `test_err_row1_output_before_trap_matches`, `test_err_row2_traps_with_garbage_upper_halves`, `row17_full_boundary_matrix`, `test_full_int_boundary_matrix` |
| C — Euclidean remainder instead of C truncation toward zero | rows 3, 7, 10, 14, 16, 17, 18, 19, 20, 21 + format/extremes tests |
| E — quotient off by one only when `x < -1e9 && y > 1e6` (a narrow, value-dependent bug) | rows 3, 4, 10, 11, 14, 16, 17, 18, 20, 21 — i.e. the randomized sweeps found it |
| D — make glibc's `if (numer >= 0 && rem < 0)` fix-up branch live | *nothing, correctly*: on x86-64 `idiv` truncates toward zero so `rem >= 0` whenever `numer >= 0`. The branch is provably dead, which is exactly what row 12 asserts. Not a real mutation. |
