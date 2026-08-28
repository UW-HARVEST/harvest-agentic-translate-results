# Error Surface

The rows below are mechanically derived from every explicit conditional return,
range check, and null check in `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|-|
| E01 | `safe_double_to_int` | `isnan(d)` | `0` | [x] |
| E02 | `safe_double_to_int` | `isinf(d) && d > 0` | `INT_MAX` | [x] |
| E03 | `safe_double_to_int` | `isinf(d) && !(d > 0)` | `INT_MIN` | [x] |
| E04 | `safe_double_to_int` | finite `d >= (double)INT_MAX` | `INT_MAX` | [x] |
| E05 | `safe_double_to_int` | finite `d <= (double)INT_MIN` | `INT_MIN` | [x] |
| E06 | `allocate_and_compute` | `malloc(size * sizeof(DataPoint)) == NULL` | `-1` | [x] |
| E07 | `fallcalc` | `malloc(5 * sizeof(int)) == NULL` | `-1` | [x] |

There are no `assert` statements, error enums, `RETURN_ERROR` macros, explicit
pointer validation checks, or documented enum/range constraints in the C
source. The two array APIs rely on the caller to provide readable storage when
`count > 0`; dereferencing an invalid pointer is undefined behavior rather than
a C rejection result.
