# Error Surface

Mechanically derived by inspecting every conditional, return, assertion, null
check, range check, enum, and min/max constant in `c_src/include/driver.h` and
`c_src/src/driver.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | Verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `printLine` | `line == NULL` | Returns normally without writing any bytes to stdout | [x] |

There are no length parameters, range checks, enums, error-return statements,
assertions, or min/max constants in the public C API. Consequently there are
no zero/oversized-length, one-past-range, or invalid-enum cases to construct.
