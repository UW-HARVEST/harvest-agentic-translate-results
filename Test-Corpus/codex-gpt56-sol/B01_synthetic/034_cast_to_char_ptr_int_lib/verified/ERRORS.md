# Error Surface

Mechanical source scan covered `c_src/include/driver.h` and
`c_src/src/driver.c` for error returns, null checks, assertions, range checks,
conditionals, switches, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

There are no rows: `driver(int x)` accepts every value representable by C
`int`, returns `void`, and contains no rejection or error branch. The generic
pointer, length, and enum boundary categories do not apply to this API.

