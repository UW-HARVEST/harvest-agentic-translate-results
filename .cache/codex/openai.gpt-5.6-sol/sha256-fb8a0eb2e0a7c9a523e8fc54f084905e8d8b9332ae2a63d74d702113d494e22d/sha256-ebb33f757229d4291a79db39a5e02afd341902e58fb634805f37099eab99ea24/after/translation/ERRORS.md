# Error surface

Mechanically derived by inspecting every `if`, null check, range check,
error-return statement, assertion, and min/max constant in
`../c_src/include/driver.h` and `../c_src/src/driver.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| ERR-001 | `printLine` | `line == NULL` | Return `void` without writing any bytes to stdout. [x] |
| ERR-002 | `driver` | `data >= 100` | Skip `strncpy` and indexed terminator write; call `printLine` with the initially empty destination and write exactly `"\n"` to stdout. [x] |

There are no error-return macros/statements, error enums, assertions, or
min/max constants in the C source.

`driver(data < 0)` is not rejected by C: it converts `data` to an oversized
`size_t` for `strncpy` and also forms a negative array index. That execution
has no defined C result, so it is not an error-surface row with a stable
expected code or sentinel.
