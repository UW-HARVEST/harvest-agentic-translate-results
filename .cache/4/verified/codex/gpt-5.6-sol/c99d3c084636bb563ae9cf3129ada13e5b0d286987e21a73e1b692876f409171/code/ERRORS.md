# Error Surface

Mechanically derived from every explicit special-value check, range check,
null check, and error return in `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `safe_double_to_int` | `isnan(d)` | `0` | [x] |
| 2 | `safe_double_to_int` | `isinf(d)` and `d > 0` | `INT_MAX` | [x] |
| 3 | `safe_double_to_int` | `isinf(d)` and `d <= 0` | `INT_MIN` | [x] |
| 4 | `safe_double_to_int` | finite `d >= (double)INT_MAX` | `INT_MAX` | [x] |
| 5 | `safe_double_to_int` | finite `d <= (double)INT_MIN` | `INT_MIN` | [x] |
| 6 | `allocate_and_compute` | `malloc(size * sizeof(DataPoint)) == NULL` | `-1` | [x] |
| 7 | `fallcalc` | `malloc(5 * sizeof(int)) == NULL` | `-1` | [x] |

Generic FFI boundaries to verify in addition to the explicit C checks:

| # | function(s) | boundary | expected C behavior | status |
|---|-------------|----------|---------------------|--------|
| G1 | `process_array_reverse`, `foreach_sum` | null pointer with `count <= 0` | no dereference; return `0` | [x] |
| G2 | `process_array_reverse`, `foreach_sum` | null pointer with positive count | invalid dereference; process terminates | [x] |
| G3 | `process_array_reverse`, `foreach_sum` | zero, one, and oversized valid-buffer lengths | sum exactly the requested elements | [x] |
| G4 | `switch_fallthrough_calculator` | operation one below and one above `0..4` | default branch; return `0` | [x] |
| G5 | `allocate_and_compute` | zero and oversized allocation lengths | `0` for zero; `-1` when allocation fails | [x] |

There are no C enum parameters, assertions, `RETURN_ERROR` macros, or
`return NULL` statements.
