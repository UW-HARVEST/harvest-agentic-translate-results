# CONFIGS.md — Phase A configuration-surface table

Mechanically derived from `c_src/src/staticloop.c` and
`c_src/include/staticloop.h`.

## Axes the C code actually branches on

Enumerated by grepping the C for every `if` / `switch` / `#ifdef` / loop bound
and every runtime option the public header exposes:

| axis | source of the axis | distinct values the C distinguishes |
|---|---|---|
| **A. entry point** | `staticloop.h` declares exactly two | `static_sum` (low-level), `driver` (composed wrapper that calls `static_sum` 10×) |
| **B. runtime options / flags** | grep for `#ifdef`, `#if`, `switch`, option setters in the header: **none** | there are no flags, no modes, no build-time configuration. The `for (int i = 0; i < 10; i++)` bound is a hard-coded literal `10`, not tunable. |
| **C. accumulator state on entry** | the function-scope `static int sum` | `sum == 0` (fresh library), `sum > 0`, `sum < 0`, `sum == INT_MAX`, `sum == INT_MIN`, `sum` near a wrap boundary |
| **D. `update` / `stride` value shape** | the only parameter; feeds `sum += update` and `i * stride` | `0`, `+1`, `-1`, small positive, small negative, `INT_MAX`, `INT_MIN`, `INT_MAX/9` (accumulator overflows but products do not), values that make `i * stride` overflow, arbitrary random `i32` |
| **E. call-sequence length** | statefulness of `sum` | zero calls (fresh), one call, many calls (accumulation), long sweeps |
| **F. entry-point interleaving** | both entry points write the same `sum` | `static_sum`-only, `driver`-only, `static_sum` then `driver`, `driver` then `static_sum`, finely alternated |
| **G. observable channel** | `driver` uses `printf("%d\n", ...)`; `static_sum` uses its return value | return value (`int`), stdout bytes, both simultaneously |
| **H. library instance freshness** | `static` storage duration is per loaded object | freshly `dlopen`ed copy (`sum == 0`) vs. an instance already mutated by earlier calls |

Axis **B is empty**: this library exposes no options, so the configuration
cross-product is A × C × D × E × F × G × H. Pruned to the combinations the code
actually treats differently, that gives the rows below.

## Configuration table

Every row is exercised with many randomized inputs (fixed seed
`0x5EED_1234_ABCD_0001`, SplitMix64) where the row admits a value range, not a
single hand-picked constant.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `static_sum` | fresh instance, single call, `update == 0` → must return `0` | [x] |
| 2 | `static_sum` | fresh instance, single call, `update == 1` | [x] |
| 3 | `static_sum` | fresh instance, single call, `update == -1` | [x] |
| 4 | `static_sum` | fresh instance, single call, randomized small positive `update` (1..=1000) | [x] |
| 5 | `static_sum` | fresh instance, single call, randomized small negative `update` (-1000..=-1) | [x] |
| 6 | `static_sum` | fresh instance, single call, randomized full-range `i32` `update` | [x] |
| 7 | `static_sum` | fresh instance, single call, `update == INT_MAX` | [x] |
| 8 | `static_sum` | fresh instance, single call, `update == INT_MIN` | [x] |
| 9 | `static_sum` | fresh instance, many calls (n = 2), accumulation of two randomized values | [x] |
| 10 | `static_sum` | fresh instance, many calls (n = 10), all-positive randomized sequence, `sum` stays in range | [x] |
| 11 | `static_sum` | fresh instance, many calls (n = 10), all-negative randomized sequence | [x] |
| 12 | `static_sum` | fresh instance, many calls (n = 256), mixed-sign randomized sequence crossing `0` repeatedly | [x] |
| 13 | `static_sum` | fresh instance, many calls (n = 1000), randomized full-range `i32` sequence → `sum` wraps many times | [x] |
| 14 | `static_sum` | pre-driven state: `sum` walked to exactly `INT_MAX`, then randomized `update` | [x] |
| 15 | `static_sum` | pre-driven state: `sum` walked to exactly `INT_MIN`, then randomized `update` | [x] |
| 16 | `static_sum` | pre-driven state: `sum` positive, then large negative `update` (crosses `0` downward) | [x] |
| 17 | `static_sum` | pre-driven state: `sum` negative, then large positive `update` (crosses `0` upward) | [x] |
| 18 | `static_sum` | argument-width shape: value passed in a 64-bit register with high bits set (FFI truncation) | [x] |
| 19 | `driver` | fresh instance, `stride == 0` → 10 lines, all `0`; return value channel unused | [x] |
| 20 | `driver` | fresh instance, `stride == 1` → canonical triangular sums `0 1 3 6 … 45` | [x] |
| 21 | `driver` | fresh instance, `stride == -1` → `0 -1 -3 … -45` | [x] |
| 22 | `driver` | fresh instance, randomized small positive `stride` (1..=1000) | [x] |
| 23 | `driver` | fresh instance, randomized small negative `stride` (-1000..=-1) | [x] |
| 24 | `driver` | fresh instance, randomized full-range `i32` `stride` → both `i * stride` and `sum` wrap | [x] |
| 25 | `driver` | fresh instance, `stride == INT_MAX` (product overflows for every `i >= 2`) | [x] |
| 26 | `driver` | fresh instance, `stride == INT_MIN` (product overflows for every `i >= 2`) | [x] |
| 27 | `driver` | fresh instance, `stride == INT_MAX / 9` and `INT_MIN / 9`, `/10` (products fit, accumulator overflows) | [x] |
| 28 | `driver` | fresh instance, `stride` a power of two near the wrap boundary (`1 << 28`, `1 << 30`) | [x] |
| 29 | `driver` | pre-driven instance (`sum != 0` from prior `static_sum` calls), randomized `stride` — stdout must reflect the carried-in `sum` | [x] |
| 30 | `driver` | repeated `driver` calls on the SAME instance (n = 8), randomized strides — accumulator carries across whole loops | [x] |
| 31 | `driver` + `static_sum` | interleaved: `static_sum` then `driver` then `static_sum`, randomized values — return values AND stdout both compared | [x] |
| 32 | `driver` + `static_sum` | finely alternated long sequence (n = 200) of randomly chosen entry points and randomized arguments; every return value and every stdout byte compared | [x] |
| 33 | `driver` | stdout byte-exactness: no trailing-newline, no width/padding, no locale-grouping differences across 10 lines for a randomized `stride` | [x] |
| 34 | both | full-`i32` randomized sweep (n = 20000) of `static_sum` on one shared instance pair, comparing every single return value | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the crate has
exactly one configuration: the default (empty) feature set. `check_features.sh`
in the crate root enumerates features from `Cargo.toml` and re-runs the whole
suite for every combination; with zero declared features that is the single
default build. Both the `dev` and `release` profiles are exercised, since
`release` sets `panic = "abort"` and disables the debug overflow checks that
could otherwise mask a wrapping-arithmetic mistranslation.
