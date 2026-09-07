# Error Surface

Mechanical scans covered `c_src/include/` and `c_src/src/` for
`RETURN_ERROR`, `return -1`, `return NULL`, error enums, assertions,
conditionals, comparisons, null checks, and min/max constants.

The C source contains no rejection branches, error returns, assertions, range
checks, null checks, enums, length parameters, or min/max constants. Therefore
the source-derived error-surface table has zero rows.

| # | function | trigger (the exact invalid input/condition) | expected C result | Status |
|---|----------|----------------------------------------------|-------------------|--------|

## Generic ABI boundary obligations

These are required independently of source-side rejection branches.

| # | function | boundary | expected C behavior | Status |
|---|----------|----------|---------------------|--------|
| G1 | `next_double` | `rnd == NULL` | no rejection exists; dereferencing the invalid pointer terminates the isolated caller process on this platform | [x] |

Zero/oversized lengths and out-of-range enum values are not applicable: this
API accepts neither a length nor an enum.
