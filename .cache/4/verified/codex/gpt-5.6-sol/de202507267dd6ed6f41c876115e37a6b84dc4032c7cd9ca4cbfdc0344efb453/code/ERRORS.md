# Error Surface

This table is derived from every explicit exceptional return, range check, and
limit in `c_src/src/lib.c`. The C source has no `assert`, error enum, null
check, `RETURN_ERROR`, or `return NULL`. Pointer arguments are dereferenced
without validation, so null pointers are outside C's defined behavior and do
not have a C result that can be compared.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `modulo_operation` | `b == 0` | returns `0` | [x] |
| 2 | `safe_double_to_int` | `d >= (double)INT32_MAX` | returns `INT32_MAX` | [x] |
| 3 | `safe_double_to_int` | `d <= (double)INT32_MIN` | returns `INT32_MIN` | [x] |
| 4 | `safe_double_to_int` | `d != d` (NaN) | returns `0` | [x] |
| 5 | `compare_results_in_array` | `idx1 >= arr->count` | returns `0` | [x] |
| 6 | `compare_results_in_array` | `idx2 >= arr->count` | returns `0` | [x] |
| 7 | `compare_results_in_array` | valid indices and `&arr->data[idx1] < &arr->data[idx2]` | returns `-1` | [x] |
| 8 | `init_result_array` | `count >= 10` | stores `arr->count = 10`; initializes only elements `0..10` | [x] |

Negative indices pass C's one-sided bounds check and then form an out-of-bounds
pointer. They are not rejection cases and cannot be called as defined C.

## Completion

- [x] Every row has a passing C-vs-Rust differential test.
