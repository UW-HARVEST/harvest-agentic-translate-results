# Error Surface

Mechanical search covered `return`, `NULL`, `assert`, conditionals, enums,
range checks, and min/max constants in `c_src/src/main.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `printLine` | `line == NULL` | returns `void` without producing output | [x] |

There are no error-return macros, error enums, assertions, range checks,
length parameters, or enum parameters in the C source.
