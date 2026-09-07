# Error surface

Mechanical searches covered `c_src/include/` and `c_src/src/` for error-return
statements/macros, `assert`, null checks, range checks, min/max constants,
enums, and conditional branches.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are zero rejection paths. The sole public API is `void driver(int x)`;
every value representable by the C `int` parameter is accepted and the
function has no error return.

## Generic FFI boundary audit

- [x] Null pointers: not applicable; there are no pointer parameters.
- [x] Zero lengths: not applicable; there are no length parameters.
- [x] Oversized lengths: not applicable; there are no length parameters.
- [x] One-past-range values: not representable outside the by-value C `int`
  ABI type; `INT_MIN`, `INT_MAX`, zero, and random full-width bit patterns are
  covered by the valid-path differential test.
- [x] Out-of-range enums: not applicable; there are no enum parameters.

All zero error-surface rows are complete.
