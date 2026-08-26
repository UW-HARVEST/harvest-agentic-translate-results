# Error Surface

This table comes from every explicit conditional rejection/suppression branch
in `c_src/src/driver.c`. The source has no error-return macros, non-void error
returns, assertions, enums, or additional null/range checks.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] E1 | `printLine` | `line == NULL` | Return `void` without writing any bytes to stdout. |
| [x] E2 | `driver` | `data >= 100` (the false side of `if (data < 100)`) | Skip `strncpy` and `dest[data]`; call `printLine` with the initially empty destination and write exactly `"\n"` to stdout. |

`driver(data < 0)` is not assigned an expected result: C converts `data` to a
huge `size_t` for `strncpy` and then indexes before `dest`, so the C program has
undefined behavior. It is neither an explicit C rejection nor a valid
differential-test oracle.
