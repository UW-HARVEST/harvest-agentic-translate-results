# Error Surface

Mechanical audit scope: all C files outside generated build output. The source
was searched for error-return statements/macros, assertions, null/range checks,
min/max constants, conditionals, switches, and preprocessor branches.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|

There are zero rejection rows. `driver` accepts a by-value C `int`, for which
every bit pattern is valid. `main` initializes that integer to zero and ignores
the return from `scanf`; failed conversion and EOF are therefore not reported
as errors and do not produce an error code or sentinel.

Generic FFI boundary audit:

- No public API accepts pointers, lengths, sizes, or enum values.
- There is no zero/oversized length boundary.
- There is no out-of-range enum representation.
- The complete signed `int` range, including both endpoints, is valid.

- [x] Zero explicit rejection rows are outstanding. Differential boundary
  coverage includes failed conversion, EOF, sign without digits, whitespace
  without digits, and decimal values immediately outside the C `int` range.
