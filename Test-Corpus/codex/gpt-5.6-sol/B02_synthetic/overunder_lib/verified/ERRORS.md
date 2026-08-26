# Error Surface

Mechanically derived from all `if`, `switch`, `return`, `assert`, `NULL`,
`INT_MIN`, `INT_MAX`, and `isnan` occurrences in `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `safe_double_to_int` | `d > (double)INT_MAX` | `INT_MAX` | [x] |
| 2 | `safe_double_to_int` | `d < (double)INT_MIN` | `INT_MIN` | [x] |
| 3 | `safe_double_to_int` | `isnan(d)` after the two range comparisons | `0` | [x] |
| 4 | `process_with_fallthrough` | `code` is not one of `0, 1, 2, 3, 4, 5` | `-1` | [x] |

There are no `assert` calls, explicit null checks, length arguments, enums,
error enums, or error-return macros/statements in the C source. Passing a null
source or destination to `copy_data_block` reaches `memcpy(..., 40)` and has
undefined behavior in C; the generic null-pointer probe therefore compares the
observed process termination of the two built shared objects rather than
claiming an error value that the C source does not define.
