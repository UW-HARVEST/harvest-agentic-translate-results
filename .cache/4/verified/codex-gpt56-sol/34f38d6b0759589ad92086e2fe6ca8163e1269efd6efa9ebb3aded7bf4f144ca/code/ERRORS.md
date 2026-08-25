# Error Surface

Mechanical search covered every statement and condition in
`c_src/src/main.c`, including searches for error returns, `assert`, null
checks, range checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|

There are no rows: the C source contains no rejection or error branch.
`driver` accepts a scalar `float`, so pointer, length, and enum boundary cases
do not apply. `main` ignores the return value of `scanf`; conversion failure or
EOF leaves its initialized value at positive zero, prints `00000000` on the
target little-endian platform, and returns `0`. Those observable input shapes
are covered in `CONFIGS.md`.

- [x] Complete: the error table is empty, and the applicable conversion-failure
  and EOF boundaries pass differential tests.
