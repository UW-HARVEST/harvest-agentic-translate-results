# Error Surface

Mechanical audit covered `c_src/include/lib.h` and `c_src/src/lib.c` for:
error-return macros/statements, error enums, `assert`, explicit range checks,
null checks, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|

There are **no explicit rejection or error paths** in the C source. In
particular, `normalize` returns `void` and performs no null or range checks.
Crash-prone generic FFI boundaries (null pointers and negative/oversized
lengths) are therefore tested as process outcomes rather than invented as
error returns. Those generic boundary checks pass for both libraries: **[x]**.
