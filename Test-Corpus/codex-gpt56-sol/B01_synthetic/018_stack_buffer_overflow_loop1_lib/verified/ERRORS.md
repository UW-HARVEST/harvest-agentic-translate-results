# Error Surface

Mechanical search covered `return`, `assert`, `NULL`, min/max constants,
range checks, and conditional branches in `c_src/include/driver.h` and
`c_src/src/driver.c`.

The C API has no error-return statements, error enums, assertions, lengths,
range checks, or min/max constants. The sole explicit null check is an accepted
no-op rather than an error return, but it is included because null checks are
part of the required error surface.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` | [x] returns `void` without writing output |

Generic FFI boundaries that do not exist in this API: lengths and enums.
