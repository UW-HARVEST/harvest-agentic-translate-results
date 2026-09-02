# ERRORS.md — Error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. Every `return` that is not the
single normal tail return, every explicit comparison guard, every null check,
every min/max constant, and every loop guard that suppresses a dereference is
one row. There are **no** `assert`s, no error enums, and no `errno` use in the
C source (verified: `grep -n 'assert' c_src/src/lib.c` → no matches).

The C source has 14 `return` statements in total (`grep -c` on `return`), of
which 8 are early/guard returns. Those 8 plus the implicit guards below make up
the table.

Legend for "expected C result": the exact value the C function returns, not
merely "it fails".

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ok |
|---|----------|----------------------------------------------|-------------------|------|----|
| E1 | `safe_double_to_int` | `isnan(d)` — quiet NaN (`0.0/0.0`) | `0` | `err_e1_e3_nan` | [x] |
| E2 | `safe_double_to_int` | `isnan(d)` — negative-sign NaN (`-(0.0/0.0)`) | `0` (sign ignored) | `err_e1_e3_nan` | [x] |
| E3 | `safe_double_to_int` | `isnan(d)` — signalling NaN bit pattern `0x7FF0000000000001` | `0` | `err_e1_e3_nan` | [x] |
| E4 | `safe_double_to_int` | `isinf(d)` and `d > 0` → `+INFINITY` | `INT_MAX` = `2147483647` | `err_e4_e5_inf` | [x] |
| E5 | `safe_double_to_int` | `isinf(d)` and `!(d > 0)` → `-INFINITY` | `INT_MIN` = `-2147483648` | `err_e4_e5_inf` | [x] |
| E6 | `safe_double_to_int` | finite and `d >= (double)INT_MAX` (i.e. `d >= 2147483647.0`, boundary **inclusive**) | `INT_MAX` | `err_e6_e7_saturate` | [x] |
| E7 | `safe_double_to_int` | finite and `d <= (double)INT_MIN` (i.e. `d <= -2147483648.0`, boundary **inclusive**) | `INT_MIN` | `err_e6_e7_saturate` | [x] |
| E8 | `allocate_and_compute` | `malloc(size * sizeof(DataPoint))` returns `NULL`; reachable because negative `size` is converted to `size_t`, producing a request of `2^64 - 16*|size|` bytes | `-1` | `err_e8_alloc_fail` | [x] |
| E9 | `allocate_and_compute` | huge positive `size` whose `size*16` byte request `malloc` refuses. Measured on this host: requests of 4 GiB and below succeed, 8 GiB and above are refused, so the test uses `size >= 1<<29` (8 GiB) up to `INT_MAX` (32 GiB) | `-1` | `err_e9_alloc_fail_big` | [x] |
| E10 | `fallcalc` | `malloc(5 * sizeof(int))` returns `NULL` | `-1` (not reachable in practice; 20-byte request) | `err_e10_unreachable` (documented, asserted non-`-1`) | [x] |
| E11 | `switch_fallthrough_calculator` | `operation` matches no `case` label — **negative** value, e.g. `-1`, `-4` (reachable from `fallcalc` because C `%` truncates toward zero) | `0` | `err_e11_e12_default` | [x] |
| E12 | `switch_fallthrough_calculator` | `operation` matches no `case` label — value **one past** the last label, `5`, and beyond (`6`, `INT_MAX`, `INT_MIN`) | `0` | `err_e11_e12_default` | [x] |
| E13 | `process_array_reverse` | `count <= 0`: `for (i = 0; i < count; …)` guard is false on entry, so `*ptr` is never dereferenced — a `NULL` `end` is therefore accepted | `0` | `err_e13_reverse_nonpos` | [x] |
| E14 | `process_array_reverse` | `count == INT_MIN` (most negative count) | `0` | `err_e13_reverse_nonpos` | [x] |
| E15 | `foreach_sum` | `count <= 0`: `FOREACH`'s `keep && idx < size` guard is false on entry, so `(array)[idx]` is never evaluated — a `NULL` `array` is therefore accepted | `0` | `err_e15_foreach_nonpos` | [x] |
| E16 | `foreach_sum` | `count == INT_MIN` | `0` | `err_e15_foreach_nonpos` | [x] |
| E17 | `allocate_and_compute` | `size == 0`: `malloc(0)` returns a non-`NULL` unique pointer on glibc, so the `NULL` guard does **not** fire; both loops are skipped, `sum` stays `0.0` | `0` (**not** `-1`) | `err_e17_size_zero` | [x] |
| E18 | `allocate_and_compute` | `multiplier` is `NaN` → every `coefficient` is `NaN` → `sum` is `NaN` → falls into E1 | `0` | `err_e18_mult_nan` | [x] |
| E19 | `allocate_and_compute` | `multiplier` is `+INFINITY`. For **every** `size >= 1` the `i == 0` element has `value == 0` and `coefficient == 0.0 * inf == NaN`, so the first term of `sum` is `0 * NaN == NaN` and `sum` stays NaN → falls into E1. It never saturates. (Verified against the C: the initial expectation of `INT_MAX` was wrong.) | `0` for every `size >= 0` | `err_e19_mult_inf` | [x] |
| E20 | `allocate_and_compute` | `multiplier` is `-INFINITY` — same mechanism as E19, `0.0 * -inf == NaN` | `0` for every `size >= 0` | `err_e19_mult_inf` | [x] |
| E21 | `allocate_and_compute` | `multiplier` = `±DBL_MAX` with `size >= 2`. Unlike E19 this *does* saturate: the `i == 0` term is `0 * DBL_MAX == 0.0` (finite, not NaN), then the `i == 1` term `8 * DBL_MAX` overflows to `±inf` → E4 / E5. With `size == 1` the only term is `0.0`, giving `0` | `INT_MAX` / `INT_MIN`; `0` when `size <= 1` | `err_e21_mult_dblmax` | [x] |
| E22 | `fallcalc` | `param4` such that `param4 % 10 + 1 <= 0` (C `%` truncates toward zero, so `param4 % 10 ∈ [-9, 9]`; any `param4` with a negative last digit, e.g. `-1 … -9`, `-19`, `INT_MIN`) makes the inner `allocate_and_compute` take E8/E17 | E8 path contributes `-1` to `result`; E17 contributes `0`; result still masked with `0777` | `err_e22_fallcalc_neg_size` | [x] |
| E23 | `fallcalc` | `param3 % 5` lands on the `default` arm (`param3` negative with `param3 % 5 != 0`) so `switch_result` is `0` (E11) | folded into `result`, masked `& 0777` | `err_e23_fallcalc_default_arm` | [x] |
| E24 | `fallcalc` | `floating_calc` saturates: extreme `param1`/`param2`/`param3` push `param1*3.7 + param2*2.3 - param3*0.5` past `INT_MAX` / below `INT_MIN` → E6 / E7 | `converted` = `INT_MAX` / `INT_MIN`, folded in and masked | `err_e24_fallcalc_saturate` | [x] |

## Generic FFI boundary cases (covered even though not table rows)

| # | condition | covered by |
|---|-----------|------------|
| G1 | `NULL` pointer + `count == 0` to `process_array_reverse` / `foreach_sum` (the only null-safe calls the C makes) | `err_e13_reverse_nonpos`, `err_e15_foreach_nonpos` |
| G2 | zero length | `err_e13_reverse_nonpos`, `err_e15_foreach_nonpos`, `err_e17_size_zero` |
| G3 | oversized length | `err_e9_alloc_fail_big`, `err_e8_alloc_fail` |
| G4 | one step past the valid `operation` range in both directions (`-1` and `5`) | `err_e11_e12_default` |
| G5 | out-of-range "enum" value across FFI: `switch_fallthrough_calculator`'s `operation` is an `int` switch, so any `int` is a real input — `INT_MIN`, `INT_MAX`, `0x8000_0000` reinterpreted, random ints | `err_e11_e12_default`, `err_g5_operation_fuzz` |
| G6 | `INT_MIN` / `INT_MAX` for every integer parameter of every entry point | `err_g6_extreme_ints` |
| G7 | signed-overflow arithmetic (`value * 8`, `value * 3`, `+ 128`, `+ 64`, `param1 * 64 + param2`) with operands near `INT_MIN`/`INT_MAX` — C is UB here, gcc at `-O0` wraps, Rust uses `wrapping_*` | `err_g7_overflow_wrap`, `cfg_*` rows |
| G8 | `-0.0`, subnormals, and the exact `±2147483647.0` / `±2147483648.0` boundary doubles | `err_e6_e7_saturate`, `cfg_c1_safe_double_to_int_special` |
