# Error surface

This table is derived from every explicit rejection, range check, null-sensitive
pointer entry point, and length boundary in `../c_src/src/lib.c`. The C API has
no enums and performs no explicit null checks; null-pointer rows therefore
compare the process-level failure produced by the unchecked C dereference/call.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|---|
| 1 | `modulo_operation` | `b == 0` | returns `0` | [x] |
| 2 | `safe_double_to_int` | `d >= (double)INT32_MAX`, including `+INFINITY` | returns `INT32_MAX` | [x] |
| 3 | `safe_double_to_int` | `d <= (double)INT32_MIN`, including `-INFINITY` | returns `INT32_MIN` | [x] |
| 4 | `safe_double_to_int` | `d != d` (NaN) | returns `0` | [x] |
| 5 | `compare_results_in_array` | `idx1 >= arr->count` | returns `0` | [x] |
| 6 | `compare_results_in_array` | `idx2 >= arr->count` | returns `0` | [x] |
| 7 | `compare_results_in_array` | `idx1 == -1`, one below the valid index range; C has no lower-bound check | compares the preceding pointer and returns `-1` for valid `idx2` | [x] |
| 8 | `compare_results_in_array` | `idx2 == -1`, one below the valid index range; C has no lower-bound check | compares the preceding pointer and returns `1` for valid `idx1` | [x] |
| 9 | `init_result_array` | `count < 0`; the `count < 10` arm accepts the invalid length | stores the negative count and writes no elements | [x] |
| 10 | `init_result_array` | `count > 10`, an oversized length | caps `arr->count` at `10` and initializes exactly ten elements | [x] |
| 11 | `compare_results_in_array` | `arr == NULL` | unchecked dereference terminates the call process | [x] |
| 12 | `init_result_array` | `arr == NULL` | unchecked write terminates the call process | [x] |
| 13 | `init_result_array` | `values == NULL && count > 0` | unchecked read terminates the call process | [x] |
| 14 | `process_with_foreach` | `arr == NULL` | unchecked dereference terminates the call process | [x] |
| 15 | `process_with_foreach` | `op == NULL && arr->count > 0` | unchecked indirect call terminates the call process | [x] |
| 16 | `compute_weighted_sum` | `arr == NULL` | unchecked dereference terminates the call process | [x] |
| 17 | `compute_weighted_sum` | `arr->count < 0` | loop condition is initially false; returns `0` | [x] |
| 18 | `compute_scaled_value` → `safe_double_to_int` | `(double)base * scale_factor >= (double)INT32_MAX` | returns `INT32_MAX` | [x] |
| 19 | `compute_scaled_value` → `safe_double_to_int` | `(double)base * scale_factor <= (double)INT32_MIN` | returns `INT32_MIN` | [x] |
| 20 | `compute_scaled_value` → `safe_double_to_int` | `(double)base * scale_factor` is NaN | returns `0` | [x] |
| 21 | `init_result_array` | `values == NULL && count == 0`; null is never dereferenced | stores count `0` and returns normally | [x] |
| 22 | `process_with_foreach` | `op == NULL && arr->count == 0`; null is never called | returns `0` and leaves the array unchanged | [x] |

There are no `assert` statements, error enums, `return -1` error sentinels,
`return NULL` branches, or public enum parameters in the C source.
