# CONFIGS.md — Phase B configuration-surface table (VALID inputs)

## Axes the C code actually branches on

Derived mechanically from `c_src/src/lib.c`. There is no global state, no
init/teardown, no option struct — every "option" is an argument value, so the
configuration axes are the *value classes* each `if`/`switch`/loop-condition
distinguishes:

| axis | values the C distinguishes | site |
|---|---|---|
| A. entry point | `safe_double_to_int`, `process_array_reverse`, `switch_fallthrough_calculator`, `allocate_and_compute`, `foreach_sum`, `fallcalc` (all 6 exported; only `fallcalc` is the "convenience wrapper" — the other 5 are the low-level entry points and are tested directly) | `nm -D` |
| B. `operation` arm | `0` (×8, +0200, &0777), `1` (+0200, &0777), `2` (&0777), `3` (×3, +0100), `4` (+0100), `default` (⇒0) | `switch_fallthrough_calculator` |
| C. double class | NaN, +Inf, −Inf, `>= (double)INT_MAX`, `<= (double)INT_MIN`, in-range positive, in-range negative, ±0.0, subnormal, fractional (truncation toward zero) | `safe_double_to_int` |
| D. count / size shape | `< 0`, `0`, `1`, `2`, small (2..10), exactly `5` (as used by `fallcalc`), large | all loops, `malloc` |
| E. pointer position | `array` = buffer start (forward walk) vs `end` = buffer *last* element (backward walk); also mid-buffer `end` so the backward walk stays in bounds | `foreach_sum` vs `process_array_reverse` |
| F. `multiplier` class | `0.0`, `1.5` (the value `fallcalc` hardcodes), negative, tiny, huge-finite, NaN, ±Inf | `allocate_and_compute` |
| G. `param3 % 5` | truncating C remainder ⇒ `{0,1,2,3,4}` for `param3 >= 0`, `{0,-1,-2,-3,-4}` for `param3 < 0` (the negative ones hit `default`) | `fallcalc` |
| H. `param4 % 10 + 1` | `1..10` (`param4 >= 0`), `0` (`param4 % 10 == -1`), `-8..-1` (`param4 % 10 <= -2` ⇒ alloc failure) | `fallcalc` |
| I. `param3 > 0200` flag | `param3 < 128`, `param3 == 128` (no flag, strict `>`), `param3 > 128` (flag set) | `fallcalc` |
| J. overflow | signed-overflowing `param1 * 0100`, `value * 010`, `value * 3`, `sum` accumulation | wrapping arithmetic |
| K. feature set | none — `Cargo.toml` has no `[features]`; default == `--no-default-features` | `Cargo.toml` |

Every row below is checked by a **randomized, fixed-seed (SplitMix64, seed
`0x5EED_1234_ABCD_0001`) property test** with many inputs per row (not a single
hand-picked value), comparing the C `.so` and Rust `.so` through `dlsym`.

## Rows

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|-------------------------------------------|--------|
| 1 | `safe_double_to_int` | in-range positive integral doubles, randomized over `0.0 ..= 2147483646.0` | [x] |
| 2 | `safe_double_to_int` | in-range negative integral doubles, randomized over `-2147483647.0 ..= -1.0` | [x] |
| 3 | `safe_double_to_int` | in-range **fractional** doubles (truncation toward zero, both signs, incl. `±0.5`, `±0.999`) | [x] |
| 4 | `safe_double_to_int` | uniformly random 64-bit bit patterns reinterpreted as `f64` (hits NaN/Inf/subnormal/huge classes at random) | [x] |
| 5 | `safe_double_to_int` | the exact boundary ladder `nextafter` around `(double)INT_MAX` and `(double)INT_MIN`, ±few ULP | [x] |
| 6 | `switch_fallthrough_calculator` | `operation == 0`, randomized `value` over full `i32` range (×010, +0200, &0777 fallthrough chain) | [x] |
| 7 | `switch_fallthrough_calculator` | `operation == 1`, randomized full-range `value` | [x] |
| 8 | `switch_fallthrough_calculator` | `operation == 2`, randomized full-range `value` | [x] |
| 9 | `switch_fallthrough_calculator` | `operation == 3`, randomized full-range `value` (×3 then +0100, **unmasked** result) | [x] |
| 10 | `switch_fallthrough_calculator` | `operation == 4`, randomized full-range `value` | [x] |
| 11 | `switch_fallthrough_calculator` | randomized `operation` over the full `i32` range × randomized `value` (mostly `default`, plus random hits on 0..4) | [x] |
| 12 | `foreach_sum` | `count == 1`, randomized element value | [x] |
| 13 | `foreach_sum` | `count == 5` (the shape `fallcalc` uses), randomized elements | [x] |
| 14 | `foreach_sum` | `count` random in `2..=64`, randomized full-range elements (sum wraps) | [x] |
| 15 | `foreach_sum` | `count` random in `2..=64`, all elements near `INT_MAX`/`INT_MIN` so the running total signed-overflows repeatedly | [x] |
| 16 | `foreach_sum` | `count == 1` but pointer aimed at the *last* element of a larger buffer (offset ≠ 0) | [x] |
| 17 | `process_array_reverse` | `count == 1`, `end` = buffer start, randomized value | [x] |
| 18 | `process_array_reverse` | `count == 5`, `end` = `buf + 4` (exactly the `fallcalc` shape), randomized elements | [x] |
| 19 | `process_array_reverse` | `count` random in `2..=64`, `end` = `buf + count - 1`, randomized full-range elements | [x] |
| 20 | `process_array_reverse` | `end` = interior element `buf + k` with `count = k + 1 <= k+1` (partial backward walk, stays in bounds), randomized `k` | [x] |
| 21 | `process_array_reverse` | overflow shape: `count` in `2..=64`, elements near `INT_MAX`/`INT_MIN` | [x] |
| 22 | `foreach_sum` + `process_array_reverse` | same buffer driven through BOTH low-level entry points in sequence (composed pipeline as `fallcalc` does it), randomized buffers | [x] |
| 23 | `allocate_and_compute` | `size == 1`, randomized finite `multiplier` (only `points[0].value == 0` contributes ⇒ 0) | [x] |
| 24 | `allocate_and_compute` | `size == 2`, randomized finite `multiplier` | [x] |
| 25 | `allocate_and_compute` | `size` random in `1..=10` (the range `fallcalc` produces), `multiplier == 1.5` (the value `fallcalc` hardcodes) | [x] |
| 26 | `allocate_and_compute` | `size` random in `1..=10`, `multiplier == 0.0` and `multiplier` negative | [x] |
| 27 | `allocate_and_compute` | `size` random in `1..=4096`, randomized moderate `multiplier` (`sum` grows large, may saturate) | [x] |
| 28 | `allocate_and_compute` | `size` random in `2..=64`, `multiplier` randomized *huge* (`1e250..1e308`) ⇒ `sum` saturates to `INT_MAX`/`INT_MIN` via the isinf/`>=INT_MAX` guards | [x] |
| 29 | `allocate_and_compute` | `size` random in `2..=64`, `multiplier` = random tiny/subnormal ⇒ `sum` truncates to 0 | [x] |
| 30 | `allocate_and_compute` | `size` random in `1..=10` × `multiplier` drawn from random 64-bit bit patterns (finite and non-finite mixed) | [x] |
| 31 | `fallcalc` | all four params random in `0..=100` (small non-negative: `param3 % 5` in `0..4`, `param4 % 10 + 1` in `1..10`, flag mostly off) | [x] |
| 32 | `fallcalc` | `param3` swept `0..=260` (crosses the `> 0200` flag boundary and every `% 5` arm), other params randomized | [x] |
| 33 | `fallcalc` | `param4` swept `-30..=30` (covers `size` ∈ `-8..10`, incl. the `0` and negative-⇒-`-1` cases), other params randomized | [x] |
| 34 | `fallcalc` | `param1`, `param2` randomized over full `i32` (base_value and switch arms overflow), `param3`,`param4` small non-negative | [x] |
| 35 | `fallcalc` | all four params randomized over the FULL `i32` range (5000 iterations, cross-product of every axis at random) | [x] |
| 36 | `fallcalc` | params drawn from a biased "interesting values" pool (`0, ±1, ±2, 4, 5, 9, 10, 127, 128, 129, 511, 512, INT_MAX, INT_MIN, INT_MAX-1, INT_MIN+1, ±10^k`) — full 4-way cross-product sampling | [x] |
| 37 | all 6 exported symbols | one combined randomized driver run that calls every symbol in the same process, in `fallcalc`'s internal call order, on the same random inputs (detects state/ordering divergence) | [x] |
| 38 | all 6 | default feature set == `--no-default-features` (no `[features]` in `Cargo.toml`); entire suite re-run under both flags | [x] |

## Cross-check: 2,000,000-iteration native differential sweep

Beyond the Rust test suite, a standalone C driver (`dlopen` on both libraries)
ran 2,000,000 random 4-tuples through `fallcalc`, plus `switch_fallthrough_calculator`,
`safe_double_to_int` over random 64-bit patterns, and `allocate_and_compute` over
random sizes/multipliers:

| comparison | mismatches |
|---|---|
| C (cmake default build) vs **Rust** | **0** |
| C `-O0` vs C `-O2` | 0 |
| C `-O0` vs C `-O3 -march=native` | 3053 (±1 on `fallcalc`) |
| C `-O3 -march=native -ffp-contract=off` vs **Rust** | **0** |

The `-march=native` rows are a **C-compiler codegen artifact, not a translation
bug**: gcc contracts `param1*3.7 + param2*2.3 - param3*0.5` into an FMA, which
changes the rounding of `floating_calc` by 1 ULP and therefore `converted` by 1.
Disabling contraction makes the optimized C agree with Rust exactly. The
ground-truth build (`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`, no
`-march=native`) matches the Rust `.so` bit-for-bit.
