# Error Surface

Mechanical searches covered `return`, `assert`, null checks, range checks,
error macros/enums, and min/max constants in `c_src/include/lib.h` and
`c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|

There are **0 rejection paths**. `encode_quant` accepts six by-value C `int`
arguments and always returns a C `int`; it has no pointer, length, enum,
documented range, assertion, error code, or sentinel surface. Consequently,
generic null-pointer, zero/oversized-length, and out-of-range-enum cases are
not applicable. Scalar zero, extrema, and bit-boundary values are valid inputs
and are covered by the valid-path differential tests in `CONFIGS.md`.
