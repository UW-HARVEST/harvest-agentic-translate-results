# ERRORS.md — Phase A: ERROR-SURFACE TABLE

Mechanically derived from `c_src/src/lib.c`. Every early `return`, every
sentinel/clamp return, every explicit range/null test, every `default:` reject
branch and every min/max constant in the C source gets one row.

Grep basis (`c_src/src/lib.c`):

```
48  int safe_double_to_int(double d) {
49      if (isnan(d))            return 0;                       -> row 1
53      if (isinf(d))            return d > 0 ? INT_MAX : INT_MIN; -> rows 2,3
57      if (d >= (double)INT_MAX) return INT_MAX;                 -> row 4
60      if (d <= (double)INT_MIN) return INT_MIN;                 -> row 5
64      return (int)d;
95      default: result = 0;                                      -> row 10
105     if (points == NULL)      return -1;                       -> rows 11,12
145     if (data_array == NULL)  return -1;                       -> row 17
```

There are **no** `assert`s and **no** null-pointer checks on the pointer
parameters of `process_array_reverse` / `foreach_sum` — the C dereferences them
unconditionally inside the loop, so the only *defined* NULL behaviour is with a
non-positive count (rows 6–9 / 13–16).

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|---------------------------------------------|-------------------|------|-----|
| 1  | `safe_double_to_int` | `d` is NaN — any payload: `f64::NAN`, `-NAN`, quiet NaN `0x7ff8..1`, signalling NaN `0x7ff0..1`, negative NaN `0xfff8..1` | `0` | `err_row01_nan` | [x] |
| 2  | `safe_double_to_int` | `d == +INFINITY` (`isinf` true, `d > 0`) | `INT_MAX` = 2147483647 | `err_row02_pos_inf` | [x] |
| 3  | `safe_double_to_int` | `d == -INFINITY` (`isinf` true, `d <= 0`) | `INT_MIN` = -2147483648 | `err_row03_neg_inf` | [x] |
| 4  | `safe_double_to_int` | finite `d >= (double)INT_MAX`: exactly `2147483647.0`, `2147483647.5`, `2147483648.0`, `1e300`, `f64::MAX` | `INT_MAX` | `err_row04_ge_int_max` | [x] |
| 5  | `safe_double_to_int` | finite `d <= (double)INT_MIN`: exactly `-2147483648.0`, `-2147483648.5`, `-2147483649.0`, `-1e300`, `f64::MIN` | `INT_MIN` | `err_row05_le_int_min` | [x] |
| 6  | `process_array_reverse` | `count == 0` (loop body never runs; pointer never dereferenced) | `0` | `err_row06_par_count_zero` | [x] |
| 7  | `process_array_reverse` | `count < 0` (incl. `-1`, `INT_MIN`) — loop condition false immediately | `0` | `err_row07_par_count_negative` | [x] |
| 8  | `process_array_reverse` | `end == NULL` **and** `count == 0` (only defined NULL case) | `0` | `err_row08_par_null_count_zero` | [x] |
| 9  | `process_array_reverse` | `end == NULL` **and** `count < 0` | `0` | `err_row09_par_null_count_negative` | [x] |
| 10 | `switch_fallthrough_calculator` | `operation` matches no `case` → `default:` — i.e. any value outside `{0,1,2,3,4}`: `5`, `6`, `100`, `-1`, `-5`, `INT_MAX`, `INT_MIN`, and out-of-range "enum" ints (`0x7fffffff`, `0x80000000` as int) | `0` regardless of `value` | `err_row10_switch_default` | [x] |
| 11 | `allocate_and_compute` | `size < 0` → `(size_t)size * 16` wraps to ≈2^64 → `malloc` fails → `points == NULL` | `-1` | `err_row11_alloc_negative_size` | [x] |
| 12 | `allocate_and_compute` | `size` so large that `size * sizeof(DataPoint)` cannot be satisfied (`INT_MAX` = 32 GiB, `INT_MAX-1`, `0x40000000` = 16 GiB, `0x20000000` = 8 GiB) → `malloc` returns NULL. (Note: `0x10000000` = 4 GiB *succeeds* on this host, so it is verified differential-only, not as an error, together with a 256 MiB "large but satisfiable" size.) | `-1` | `err_row12_alloc_huge_size` | [x] |
| 13 | `foreach_sum` | `count == 0` → `FOREACH` never enters the body | `0` | `err_row13_foreach_count_zero` | [x] |
| 14 | `foreach_sum` | `count < 0` (incl. `INT_MIN`) → `keep && idx < size` false immediately | `0` | `err_row14_foreach_count_negative` | [x] |
| 15 | `foreach_sum` | `array == NULL` **and** `count == 0` | `0` | `err_row15_foreach_null_count_zero` | [x] |
| 16 | `foreach_sum` | `array == NULL` **and** `count < 0` | `0` | `err_row16_foreach_null_count_negative` | [x] |
| 17 | `fallcalc` | `data_array == NULL` after `malloc(5 * sizeof(int))` → `return -1`. **Unreachable in practice** (fixed 20-byte request); documented and asserted as "C and Rust agree that this never fires" — both always return a value in `0..=511`. | `-1` (never observed) | `err_row17_fallcalc_malloc_never_fails` | [x] |
| 18 | `fallcalc` | `param4` such that `param4 % 10 + 1 <= 0`, i.e. `param4 % 10 == -1` (`-1, -11, -21, …`) → `allocate_and_compute(0, 1.5)`: `malloc(0)` is non-NULL → no `-1`, contributes `0` | value in `0..=511`, `alloc_result == 0` | `err_row18_fallcalc_alloc_size_zero` | [x] |
| 19 | `fallcalc` | `param4` such that `param4 % 10 + 1 < 0`, i.e. `param4 % 10 <= -2` (`-2 … -9`, `-12`, …) → `allocate_and_compute(negative, 1.5)` returns `-1`, which is folded into `result` | value in `0..=511`, `alloc_result == -1` | `err_row19_fallcalc_alloc_negative` | [x] |
| 20 | `fallcalc` | `param3 % 5` outside `{0..4}` (negative `param3`) → `switch_fallthrough_calculator` `default:` → `switch_result == 0` | value in `0..=511` | `err_row20_fallcalc_switch_default` | [x] |
| 21 | `fallcalc` | `floating_calc` saturates `safe_double_to_int` (e.g. `param1 = INT_MAX` → `3.7 * 2^31 > INT_MAX` → `INT_MAX`; `param1 = INT_MIN` → `INT_MIN`) | value in `0..=511` | `err_row21_fallcalc_float_saturation` | [x] |
| 22 | min/max constants | `INT_MAX` / `INT_MIN` clamp constants themselves are exercised as *returned* values, and `OCTAL_MASK_1` (0777) is asserted as the hard upper bound of every `fallcalc` result | `fallcalc` result always `& 0777` | `err_row22_masks_and_limits` | [x] |
| 23 | generic FFI boundary | out-of-range "enum" ints for the `operation` parameter passed across FFI (`i32::MIN`, `i32::MAX`, `-2147483647`, `2147483646`) plus one-past-valid-range `5` and `-1` | identical (all `0`) | `err_row23_out_of_range_enum_ints` | [x] |
| 24 | generic FFI boundary | zero and oversized lengths on both pointer APIs at once: `count ∈ {0, -1, INT_MIN, INT_MAX(with NULL, negative-guarded)}`, plus `size ∈ {0, -1, INT_MIN, INT_MAX}` for `allocate_and_compute` | identical | `err_row24_zero_and_oversized_lengths` | [x] |

All 24 rows are covered by `tests/phase_c_errors.rs` and pass against both
libraries (see the run log at the bottom of `CONFIGS.md`).
