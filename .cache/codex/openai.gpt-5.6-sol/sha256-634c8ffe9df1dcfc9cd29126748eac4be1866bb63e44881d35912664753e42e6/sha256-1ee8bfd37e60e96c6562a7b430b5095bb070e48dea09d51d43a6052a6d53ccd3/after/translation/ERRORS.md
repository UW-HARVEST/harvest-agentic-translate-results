# Error Surface

The C source has no error macro, assertion, error enum, null check, or
length/count argument. The rows below are the complete set of explicit
range/special-value rejection branches and sentinel-return branches.

| # | function | trigger (the exact invalid input/condition) | expected C result | Status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `safe_double_to_int` | `d > (double)INT_MAX`, including positive infinity | `INT_MAX` | [x] |
| 2 | `safe_double_to_int` | `d < (double)INT_MIN`, including negative infinity | `INT_MIN` | [x] |
| 3 | `safe_double_to_int` | `isnan(d)` after both range comparisons are false | `0` | [x] |
| 4 | `process_with_fallthrough` | `code` is not one of `0`, `1`, `2`, `3`, `4`, or `5` | `-1` | [x] |

`copy_data_block` passes both pointers directly to `memcpy` for 40 bytes.
Null, dangling, undersized, or overlapping storage is not rejected by C and
has undefined behavior, so there is no C error result to put in this table.
The generic null-pointer boundary is tested out of process by comparing
termination behavior.

The remaining functions take only fixed-width scalar arguments. They have no
lengths, enums, pointer arguments, documented ranges, or explicit rejection
paths.
