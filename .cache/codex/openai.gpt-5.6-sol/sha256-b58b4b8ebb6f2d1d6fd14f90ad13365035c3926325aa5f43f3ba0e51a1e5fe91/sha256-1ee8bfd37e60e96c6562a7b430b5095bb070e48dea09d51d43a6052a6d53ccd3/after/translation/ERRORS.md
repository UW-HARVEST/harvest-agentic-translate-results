# Error Surface

Mechanical review covered `return` statements, error macros, `assert`, null
checks, range checks, enums, and min/max constants in `../c_src/include` and
`../c_src/src`. The only explicit rejection is the null check in `printLine`.
The API has no lengths, enums, error-return macros, assertions, or range
constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | Verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `printLine` | `line == NULL` | Return `void` without writing output | [x] |

Generic FFI boundaries:

- `printLine`: null is row 1. A non-null pointer that does not identify a
  NUL-terminated readable C string is outside the C function's defined input
  domain.
- `bad` and `good`: no parameters.
- `driver`: accepts the full C `int` range; zero and nonzero values are valid.
- No API accepts a length or enum, so zero/oversized lengths and invalid enum
  discriminants do not apply.
