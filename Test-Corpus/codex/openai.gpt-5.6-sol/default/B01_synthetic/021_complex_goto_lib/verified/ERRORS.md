# Error Surface

Mechanical searches covered `c_src/include/driver.h` and
`c_src/src/driver.c` for error returns, `return`, `assert`, null checks, range
checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

There are **0 rejection branches**. `driver` returns `void`, accepts two C
`int` values, and has no pointers, lengths, enum arguments, documented input
range, return statements, assertions, error macros, or error sentinels.
Consequently, the generic null-pointer, zero/oversized-length, and invalid-enum
cases are not applicable. Zero and signed integer boundary shapes are valid
configurations and are covered in `CONFIGS.md`.
