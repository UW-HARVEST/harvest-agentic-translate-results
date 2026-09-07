# Error surface

The C source contains no `assert`, error enum, `RETURN_ERROR`, explicit pointer
validation, or documented enum type. Rows 1-7 are the mechanically found
exception/range/allocation branches. Rows 8-17 are the mandatory generic FFI
boundaries; where C performs no rejection, the expected result records that
exact behavior.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|-|
| 1 | `safe_double_to_int` | `isnan(d)` | `0` | [x] |
| 2 | `safe_double_to_int` | `isinf(d)` and `d > 0` | `INT_MAX` | [x] |
| 3 | `safe_double_to_int` | `isinf(d)` and `d <= 0` | `INT_MIN` | [x] |
| 4 | `safe_double_to_int` | finite `d >= (double)INT_MAX` | `INT_MAX` | [x] |
| 5 | `safe_double_to_int` | finite `d <= (double)INT_MIN` | `INT_MIN` | [x] |
| 6 | `allocate_and_compute` | `malloc(size * sizeof(DataPoint)) == NULL` | `-1` | [x] |
| 7 | `fallcalc` | `malloc(5 * sizeof(int)) == NULL` | `-1` | [x] |
| 8 | `switch_fallthrough_calculator` | `operation < 0` (out-of-range operation value) | default branch returns `0` | [x] |
| 9 | `switch_fallthrough_calculator` | `operation > 4` (out-of-range operation value) | default branch returns `0` | [x] |
| 10 | `process_array_reverse` | `end == NULL`, `count == 0` | loop is skipped; returns `0` | [x] |
| 11 | `process_array_reverse` | `end == NULL`, `count < 0` | loop is skipped; returns `0` | [x] |
| 12 | `process_array_reverse` | `end == NULL`, `count > 0` | no C rejection; invalid dereference terminates the isolated caller | [x] |
| 13 | `foreach_sum` | `array == NULL`, `count == 0` | macro loop is skipped; returns `0` | [x] |
| 14 | `foreach_sum` | `array == NULL`, `count < 0` | macro loop is skipped; returns `0` | [x] |
| 15 | `foreach_sum` | `array == NULL`, `count > 0` | no C rejection; invalid dereference terminates the isolated caller | [x] |
| 16 | `process_array_reverse` | `count` is one larger than the readable reverse span | no C rejection; guard-page dereference terminates the isolated caller | [x] |
| 17 | `foreach_sum` | `count` is one larger than the readable forward span | no C rejection; guard-page dereference terminates the isolated caller | [x] |

No public function accepts a C enum, so there is no enum representation to
pass an invalid discriminant through. The integer `operation` switch is covered
on both sides of its accepted `0..=4` case range.
