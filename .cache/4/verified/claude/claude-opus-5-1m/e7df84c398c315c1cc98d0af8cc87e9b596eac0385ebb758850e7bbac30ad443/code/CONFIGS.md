# CONFIGS.md — Phase A: CONFIGURATION-SURFACE TABLE

## Build-time configuration surface

### `Cargo.toml`

```
[lib] name = "fallcalc_lib"  crate-type = ["cdylib"]
[dev-dependencies] libloading = "0.8"
```

There is **no `[features]` section**, therefore the crate has exactly **one**
valid feature combination: the empty set.

| # | feature combo | `cargo check` command | result |
|---|---------------|-----------------------|--------|
| 1 | *(none — no features declared)* | `cargo check --offline --no-default-features` | OK, 0 errors, 0 warnings |

`--all-features` and `--features <anything>` are degenerate/invalid here
(`cargo` rejects unknown features), so combination #1 is the complete
enumeration. Phases B and C are therefore run once, and additionally re-run in
`--release` (which changes `opt-level`, and `panic = "abort"` per
`[profile.release]`) to prove the code is not relying on debug-mode arithmetic
behaviour.

### `c_src/CMakeLists.txt`

```
add_library(${project_name} SHARED src/lib.c)
target_include_directories(... PUBLIC include / PRIVATE src)
```

No `option()`, no `target_compile_definitions`, no `#ifdef` in `lib.c` — the C
side has exactly one build configuration too. `CMAKE_BUILD_TYPE` is unset ⇒ the
reference `.so` is built without optimisation.

## Runtime configuration axes (derived from the C branches)

| axis | where it is branched on in `lib.c` | values the C distinguishes |
|------|-----------------------------------|----------------------------|
| A. `operation` mode selector | `switch (operation)` L82–97 | `0` (×8, +0200, &0777), `1` (+0200, &0777), `2` (&0777), `3` (×3, +0100), `4` (+0100), *default* (→0) — 6 distinct paths, note the deliberate fall-through 0→1→2 and 3→4 |
| B. `d` classification | `isnan` / `isinf` / `>= INT_MAX` / `<= INT_MIN` / truncate, L49–64 | NaN, +inf, -inf, ≥INT_MAX, ≤INT_MIN, in-range positive, in-range negative, ±0, subnormal |
| C. element count / array shape | `for (i < count)` L71, `FOREACH` L34 | `count` = negative, 0, 1, 2, many; `NULL` array with non-positive count |
| D. traversal direction | `process_array_reverse` walks **backwards** (`ptr--`) from `end` | forward (`foreach_sum`) vs reverse (`process_array_reverse`) over the same buffer |
| E. allocation size | `malloc(size * sizeof(DataPoint))` L103 | `size` < 0 (wraps → NULL), 0 (`malloc(0)` ≠ NULL), 1, small, large-but-OK, `INT_MAX` (NULL) |
| F. `multiplier` value | `points[i].coefficient = i * multiplier` L111 | 0.0, -0.0, 1.5 (the value `fallcalc` uses), negative, huge (sum saturates), NaN, ±inf, subnormal |
| G. `value` magnitude | arithmetic in `switch_fallthrough_calculator` L84–93 | 0, small +, small -, values that overflow `*8` / `*3` / `+0200` (`INT_MAX`, `INT_MIN`, near-boundary) |
| H. `param3 > 0200` flag | `if (param3 > OCTAL_FLAG)` L167 | `param3 <= 128` (no OR) vs `param3 > 128` (`result |= 0200`) |
| I. `param3 % 5` sub-mode | L158 feeds axis A | `param3 % 5 ∈ {0,1,2,3,4}` for `param3 >= 0`, `{0,-1,-2,-3,-4}` for `param3 < 0` (C truncating `%`) |
| J. `param4 % 10 + 1` sub-mode | L163 feeds axis E | `1..10` for `param4 >= 0`; `0` when `param4 % 10 == -1`; `-8..-1` otherwise |
| K. integer wrap-around | `param1 * 0100 + param2` L140, `(i+1)*010 + param1` L150 | non-overflowing vs overflowing multiplies/adds |
| L. entry-point level | public/exported API | 5 low-level entry points (`safe_double_to_int`, `process_array_reverse`, `switch_fallthrough_calculator`, `allocate_and_compute`, `foreach_sum`) + 1 composed one-shot (`fallcalc`, the only symbol in `include/lib.h`) |

## Configuration rows (cross-product, pruned to what the C distinguishes)

Every row is driven through **both** `.so` files via `libloading` and compared
byte-for-byte. "randomised" = many inputs per row from a fixed-seed xorshift64\*
PRNG (seed `0x2545F4914F6CDD1D`), see `tests/common/mod.rs`.

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `safe_double_to_int` | axis B: in-range positive doubles, randomised in `(0, 2147483647)` incl. fractional parts | `cfg_row01_sdti_in_range_positive` | [x] |
| 2  | `safe_double_to_int` | axis B: in-range negative doubles, randomised in `(-2147483648, 0)` | `cfg_row02_sdti_in_range_negative` | [x] |
| 3  | `safe_double_to_int` | axis B: `±0.0`, subnormals (`f64::MIN_POSITIVE/2`, `5e-324`), `±1e-300` (truncate to 0) | `cfg_row03_sdti_zero_and_subnormal` | [x] |
| 4  | `safe_double_to_int` | axis B: boundary sweep — every double within ±4 ULP of `INT_MAX` and `INT_MIN`, plus `±2147483646.5`, `±0.5`, `±1.0` | `cfg_row04_sdti_boundary_sweep` | [x] |
| 5  | `safe_double_to_int` | axis B: fully random 64-bit **bit patterns** reinterpreted as `f64` (hits NaN/inf/huge/subnormal/in-range at natural frequencies) | `cfg_row05_sdti_random_bitpatterns` | [x] |
| 6  | `switch_fallthrough_calculator` | axes A×G: `operation = 0` (the `×8 → +0200 → &0777` fall-through chain) × randomised `value` incl. overflow-inducing magnitudes | `cfg_row06_switch_op0` | [x] |
| 7  | `switch_fallthrough_calculator` | axes A×G: `operation = 1` (`+0200 → &0777`) × randomised `value` | `cfg_row07_switch_op1` | [x] |
| 8  | `switch_fallthrough_calculator` | axes A×G: `operation = 2` (`&0777` only) × randomised `value` | `cfg_row08_switch_op2` | [x] |
| 9  | `switch_fallthrough_calculator` | axes A×G: `operation = 3` (`×3 → +0100`, **no** mask) × randomised `value` incl. `INT_MAX`/`INT_MIN` overflow | `cfg_row09_switch_op3` | [x] |
| 10 | `switch_fallthrough_calculator` | axes A×G: `operation = 4` (`+0100`, no mask) × randomised `value` incl. `INT_MAX` overflow | `cfg_row10_switch_op4` | [x] |
| 11 | `switch_fallthrough_calculator` | axes A×G: randomised `operation` over the **whole** `i32` range × randomised `value` (mostly default path, occasionally 0..4) | `cfg_row11_switch_random_op` | [x] |
| 12 | `foreach_sum` | axis C: `count = 1` — single element, randomised value incl. extremes | `cfg_row12_foreach_single` | [x] |
| 13 | `foreach_sum` | axis C: `count = 2` and `count = 3` — randomised contents | `cfg_row13_foreach_few` | [x] |
| 14 | `foreach_sum` | axis C: `count` = 5 (the size `fallcalc` uses), 16, 64, 257, 1024 — randomised contents | `cfg_row14_foreach_many` | [x] |
| 15 | `foreach_sum` | axes C×K: contents chosen so the running `total` wraps `int` (all `INT_MAX`, all `INT_MIN`, alternating) | `cfg_row15_foreach_overflow` | [x] |
| 16 | `process_array_reverse` | axes C×D: `count = 1` from the last element — randomised buffer | `cfg_row16_reverse_single` | [x] |
| 17 | `process_array_reverse` | axes C×D: `count = n` starting at `buf + n - 1` (walks the whole buffer backwards) for `n ∈ {2,3,5,16,64,1024}` | `cfg_row17_reverse_full_buffer` | [x] |
| 18 | `process_array_reverse` | axes C×D: partial reverse window — start in the middle (`buf + k`, `count = k+1 ≤ k+1`) so only part of the buffer is read | `cfg_row18_reverse_partial_window` | [x] |
| 19 | `process_array_reverse` | axes C×D×K: overflow-inducing contents (all `INT_MIN`, all `INT_MAX`, random extremes) | `cfg_row19_reverse_overflow` | [x] |
| 20 | `foreach_sum` + `process_array_reverse` | axis D interaction: same buffer, forward sum vs reverse sum must be equal in both libs (and cross-lib identical) | `cfg_row20_forward_vs_reverse_same_buffer` | [x] |
| 21 | `allocate_and_compute` | axes E×F: `size = 0` × every `multiplier` class (0, 1.5, -1.5, NaN, ±inf, huge) → `malloc(0)` path, result 0 | `cfg_row21_alloc_size_zero_all_multipliers` | [x] |
| 22 | `allocate_and_compute` | axes E×F: `size = 1` × every `multiplier` class (`i = 0` only ⇒ `0 * multiplier`, incl. `0 * inf = NaN`, `0 * NaN = NaN`) | `cfg_row22_alloc_size_one_all_multipliers` | [x] |
| 23 | `allocate_and_compute` | axes E×F: `size ∈ 1..=10` (the exact range `fallcalc` can produce) × `multiplier = 1.5` | `cfg_row23_alloc_fallcalc_range` | [x] |
| 24 | `allocate_and_compute` | axes E×F: `size ∈ {2,3,5,16,64,1000,65536}` × randomised finite `multiplier` (in-range sums) | `cfg_row24_alloc_many_sizes_random_mult` | [x] |
| 25 | `allocate_and_compute` | axes E×F: `multiplier` large enough that `sum` exceeds `INT_MAX` / `INT_MIN` ⇒ clamp path inside `safe_double_to_int` | `cfg_row25_alloc_sum_saturates` | [x] |
| 26 | `allocate_and_compute` | axes E×F: `multiplier` NaN/±inf with `size >= 2` ⇒ `sum` becomes NaN/±inf ⇒ rows 1–3 of `ERRORS.md` reached *through* this entry point | `cfg_row26_alloc_nonfinite_sum` | [x] |
| 27 | `allocate_and_compute` | axes E×F: `multiplier` = random 64-bit bit pattern as `f64`, `size ∈ {1,2,7}` | `cfg_row27_alloc_random_bitpattern_mult` | [x] |
| 28 | `fallcalc` | axes H×I×J: exhaustive small grid `param1,param2 ∈ -3..=3`, `param3 ∈ -6..=6`, `param4 ∈ -12..=12` (covers every `param3 % 5` and `param4 % 10 + 1` sub-mode and both sides of the `param3 > 0200` flag boundary is added separately) | `cfg_row28_fallcalc_small_exhaustive_grid` | [x] |
| 29 | `fallcalc` | axis H: `param3` swept across the `0200` flag boundary (`126..=131`) × randomised other params | `cfg_row29_fallcalc_flag_boundary` | [x] |
| 30 | `fallcalc` | axis I: `param3` chosen so `param3 % 5` hits each of `0,1,2,3,4,-1,-2,-3,-4` × randomised other params | `cfg_row30_fallcalc_all_switch_submodes` | [x] |
| 31 | `fallcalc` | axis J: `param4` chosen so `param4 % 10 + 1` hits each of `1..=10`, `0`, `-1..=-8` × randomised other params | `cfg_row31_fallcalc_all_alloc_submodes` | [x] |
| 32 | `fallcalc` | axis K: `param1`/`param2` at `INT_MAX`, `INT_MIN`, `±(2^24)`, `±(2^31/0100)` → overflow in `param1 * 0100 + param2` and in `(i+1)*010 + param1` | `cfg_row32_fallcalc_overflow_params` | [x] |
| 33 | `fallcalc` | axes B×K: `param1/param2/param3` magnitudes that saturate `floating_calc` at `+INT_MAX` and `-INT_MIN` | `cfg_row33_fallcalc_float_saturation_path` | [x] |
| 34 | `fallcalc` | all axes: fully randomised `i32` quadruples over the whole range (10 000 cases) | `cfg_row34_fallcalc_random_full_range` | [x] |
| 35 | `fallcalc` | all axes: randomised quadruples drawn from a "spicy" pool of boundary constants (`0, ±1, ±5, ±10, 127..129, ±INT_MAX, ±INT_MIN, ±0100, ±0200, ±0777`) — cross-product sampling | `cfg_row35_fallcalc_boundary_pool` | [x] |
| 36 | composed pipeline | low-level call sequence replicating `fallcalc` **by hand** (`malloc`-backed buffer → `foreach_sum` → `process_array_reverse` → `switch_fallthrough_calculator` → `safe_double_to_int` → `allocate_and_compute`) and checking the manual composition equals the library's `fallcalc` in **both** libs, for randomised params | `cfg_row36_manual_pipeline_matches_fallcalc` | [x] |
| 37 | cross-library buffer sharing | Rust `foreach_sum`/`process_array_reverse` fed a buffer written by the C lib's view and vice-versa (same caller-owned memory, both `.so`s reading it) to prove no hidden state/ABI mismatch | `cfg_row37_shared_caller_owned_buffer` | [x] |

## Test-run log

See `RESULTS.md` for the recorded pass output of every phase and profile.
