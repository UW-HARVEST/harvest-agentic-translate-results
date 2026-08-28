# Error Surface

The C API has no error enum and no negative-error return convention. The rows
below are every explicit rejection, clamping, or invalid-input branch found by
searching `src/lib.c` for conditionals, returns, assertions, and bounds
constants. There are no assertions or null checks in the C source.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] E1 | `modulo_operation` | divisor `b == 0` | returns `0` |
| [x] E2 | `safe_double_to_int` | `d >= (double)INT32_MAX`, including positive infinity | returns `INT32_MAX` |
| [x] E3 | `safe_double_to_int` | `d <= (double)INT32_MIN`, including negative infinity | returns `INT32_MIN` |
| [x] E4 | `safe_double_to_int` | `d != d` (NaN) | returns `0` |
| [x] E5 | `compare_results_in_array` | `idx1 >= arr->count` | returns `0` |
| [x] E6 | `compare_results_in_array` | `idx2 >= arr->count` while `idx1 < arr->count` | returns `0` |
| [x] E7 | `init_result_array` | requested `count >= 10` | clamps `arr->count` to `10`; initializes only ten elements |

## Unchecked FFI Preconditions

The C implementation dereferences pointer arguments without checking them.
Consequently, null `arr`, null `values` when a positive count is initialized,
null callbacks, negative indices that pass the upper-bound-only check, and
array counts above ten in processing functions have undefined behavior rather
than a defined error result. Differential tests exercise defined null/length
boundaries and isolate representative undefined null calls in child processes
so a fault cannot terminate the test harness.
