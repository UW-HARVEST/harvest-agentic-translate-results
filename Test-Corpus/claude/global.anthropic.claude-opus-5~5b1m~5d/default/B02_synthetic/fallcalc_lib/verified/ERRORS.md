# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection / early-return / sentinel / implicit
saturation branch in `c_src/src/lib.c`. There are no `assert`s, no error enums
and no `errno` use in the C; the rejection mechanisms are:

* `return -1` on `malloc` failure (2 sites: `allocate_and_compute`, `fallcalc`)
* the saturating / NaN guards in `safe_double_to_int` (5 guarded returns)
* the `default:` arm of `switch_fallthrough_calculator` (`result = 0`)
* implicit "do nothing" guards: loop conditions `i < count` / `idx < size`
  reject non-positive counts by never entering the body (return 0)
* `INT_MAX` / `INT_MIN` / `0777` / `0100` / `0200` / `010` constants

| # | function | trigger (exact invalid input/condition) | expected C result | test | status |
|---|----------|------------------------------------------|-------------------|------|--------|
| 1 | `safe_double_to_int` | `isnan(d)` — quiet NaN | `0` | `err_01_nan` | [x] |
| 2 | `safe_double_to_int` | `isnan(d)` — negative / signalling NaN bit patterns | `0` | `err_01_nan` | [x] |
| 3 | `safe_double_to_int` | `isinf(d) && d > 0` (`+INFINITY`) | `INT_MAX` = 2147483647 | `err_02_pos_inf` | [x] |
| 4 | `safe_double_to_int` | `isinf(d) && d < 0` (`-INFINITY`) | `INT_MIN` = -2147483648 | `err_03_neg_inf` | [x] |
| 5 | `safe_double_to_int` | finite `d >= (double)INT_MAX` (== 2147483647.0, one step past, 1e300, DBL_MAX) | `INT_MAX` | `err_04_ge_int_max` | [x] |
| 6 | `safe_double_to_int` | finite `d <= (double)INT_MIN` (== -2147483648.0, one step past, -1e300, -DBL_MAX) | `INT_MIN` | `err_05_le_int_min` | [x] |
| 7 | `safe_double_to_int` | one step *inside* each bound (`nextafter(INT_MAX,0)`, `nextafter(INT_MIN,0)`) — must NOT saturate | truncated cast, `2147483646` / `-2147483647` | `err_06_just_inside_bounds` | [x] |
| 8 | `safe_double_to_int` | `-0.0`, subnormals (`DBL_MIN`, `5e-324`) | `0` | `err_07_zero_and_subnormal` | [x] |
| 9 | `allocate_and_compute` | `size < 0` ⇒ `(size_t)size * 16` is astronomically large ⇒ `malloc` returns `NULL` | `-1` | `err_08_alloc_negative_size` | [x] |
| 10 | `allocate_and_compute` | `size` such that `size * sizeof(DataPoint)` overflows/exhausts memory (e.g. `INT_MAX`, `INT_MAX/2`) ⇒ `malloc` `NULL` | `-1` | `err_09_alloc_huge_size` | [x] |
| 11 | `allocate_and_compute` | `size == 0` ⇒ `malloc(0)` returns a non-NULL unique pointer, both loops skipped, `sum == 0.0` | `0` (**not** `-1`) | `err_10_alloc_zero_size` | [x] |
| 12 | `allocate_and_compute` | `multiplier` = NaN ⇒ `sum` becomes NaN (for `size >= 2`) ⇒ `safe_double_to_int` NaN guard | `0` | `err_11_alloc_nan_multiplier` | [x] |
| 13 | `allocate_and_compute` | `multiplier` = ±Inf, any `size >= 1`. **Verified against C:** `points[0].coefficient = 0.0 * inf = NaN` and `points[0].value = 0`, so the first term `0 * NaN = NaN` poisons `sum` for every size — the `isnan` guard fires, **not** the `isinf` one | `0` (**not** `INT_MAX`/`INT_MIN`) | `err_12_alloc_inf_multiplier` | [x] |
| 14 | `allocate_and_compute` | `size == 1` with any multiplier: only `points[0]` exists and `points[0].value == 0`, so the product is `0 * anything` (`0`, or `NaN` for non-finite) | `0` for every multiplier | `err_12_alloc_inf_multiplier`, `cfg_23` | [x] |
| 15 | `allocate_and_compute` | huge finite `multiplier` (`1e300`) with `size >= 2` ⇒ `sum` overflows to `+Inf` | `INT_MAX` | `err_13_alloc_overflow_to_inf` | [x] |
| 16 | `process_array_reverse` | `count == 0` — loop never entered, pointer never dereferenced (accepts even a garbage/NULL `end`) | `0` | `err_14_reverse_zero_count` | [x] |
| 17 | `process_array_reverse` | `count < 0` (incl. `INT_MIN`) — `i < count` false immediately, no deref | `0` | `err_15_reverse_negative_count` | [x] |
| 18 | `process_array_reverse` | `end == NULL` **and** `count <= 0` — must not crash, must return 0 | `0` | `err_16_reverse_null_ptr_zero_count` | [x] |
| 19 | `foreach_sum` | `count == 0` — `FOREACH` expands to `idx < size` false ⇒ body skipped | `0` | `err_17_foreach_zero_count` | [x] |
| 20 | `foreach_sum` | `count < 0` (incl. `INT_MIN`) | `0` | `err_18_foreach_negative_count` | [x] |
| 21 | `foreach_sum` | `array == NULL` **and** `count <= 0` | `0` | `err_19_foreach_null_ptr_zero_count` | [x] |
| 22 | `switch_fallthrough_calculator` | `operation` outside `0..=4` — i.e. an out-of-range "enum" value crossing FFI (`5`, `6`, `-1`, `-5`, `INT_MIN`, `INT_MAX`, `1<<31` wrap) hits `default:` | `0` | `err_20_switch_default_arm` | [x] |
| 23 | `switch_fallthrough_calculator` | `value == INT_MAX` / `INT_MIN` with `operation == 0` ⇒ signed overflow in `result *= 010` then `+= 0200` then `& 0777` | same masked low 9 bits as C (gcc wraps) | `err_21_switch_overflow_values` | [x] |
| 24 | `switch_fallthrough_calculator` | `value == INT_MAX` with `operation == 3` ⇒ `result *= 3` overflows, then `+= 0100`, **no mask** ⇒ full 32-bit wrapped value escapes | wrapped `int` | `err_21_switch_overflow_values` | [x] |
| 25 | `fallcalc` | `param3 % 5` lands outside `0..=4` because C `%` is truncating ⇒ negative `param3` ⇒ `default:` arm ⇒ `switch_result == 0` | see differential | `err_22_fallcalc_negative_param3` | [x] |
| 26 | `fallcalc` | `param4 % 10 + 1 <= -1`, i.e. `param4 % 10 <= -2` (negative `param4`) ⇒ `allocate_and_compute` gets a negative size ⇒ inner `malloc` `NULL` ⇒ `alloc_result == -1` (propagated into the sum, **not** returned as an error) | `(… + -1) & 0777` | `err_23_fallcalc_negative_param4` | [x] |
| 27 | `fallcalc` | `param4 % 10 == -1` (`param4 ∈ {-1,-11,-21,…}`) ⇒ size `0` ⇒ `malloc(0)` ⇒ `alloc_result == 0` (boundary between rows 26 and the happy path) | `(… + 0) & 0777` | `err_23_fallcalc_negative_param4` | [x] |
| 28 | `fallcalc` | `param1 * 0100 + param2` signed-overflows (`param1` near `INT_MAX`/`INT_MIN`) | wrapped, then `& 0777` | `err_24_fallcalc_overflow_params` | [x] |
| 29 | `fallcalc` | `param3 > 0200` ⇒ `result \|= 0200` is applied *before* the `& 0777` mask | bit 7 forced set | `err_25_fallcalc_flag_boundary` | [x] |
| 30 | `fallcalc` | `param3 == 0200` exactly (`128`) — strict `>` ⇒ flag NOT applied (off-by-one boundary) | no bit-7 force | `err_25_fallcalc_flag_boundary` | [x] |
| 31 | `fallcalc` | `param3 == INT_MIN` ⇒ `INT_MIN % 5 == -3` (well-defined in C), and `INT_MIN > 0200` is false | see differential | `err_26_fallcalc_extremes` | [x] |
| 32 | `fallcalc` | all four params at `INT_MIN`/`INT_MAX`/`0`/`-1` (cross-product of extremes, 256 combos) | see differential | `err_26_fallcalc_extremes` | [x] |
| 33 | `fallcalc` | `data_array == NULL` ⇒ `return -1`. Unreachable in practice (`malloc(5*4)` never fails), documented as dead-but-present branch; asserted indirectly — `fallcalc` never returns `-1` because the final `& 0777` cannot yield `-1` | never `-1` | `err_27_fallcalc_never_returns_minus_one` | [x] |
| 34 | all functions | return value is always in `INT_MIN..=INT_MAX` and `fallcalc`'s is always in `0..=511` (post-mask invariant) | in-range | `err_28_fallcalc_range_invariant` | [x] |
