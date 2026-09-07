# Error-surface table

Mechanical scans covered `../c_src/include/driver.h` and
`../c_src/src/driver.c` for error returns, `assert`, conditionals, range
checks, null checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | Status |
|---|----------|---------------------------------------------|-------------------|--------|

There are no rejection paths. The sole public API accepts one `double` by
value and returns `void`; it has no pointers, lengths, enums, documented
ranges, or invalid bit patterns. Therefore the generic null, zero/oversized
length, and out-of-range enum boundary cases are not applicable.
