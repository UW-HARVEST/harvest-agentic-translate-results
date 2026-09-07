# Error Surface

Derived mechanically by searching `c_src/include` and `c_src/src` for error
returns, null checks, range checks, assertions, enums, and min/max constants.
The C library has no error codes, error-return statements, assertions, enums,
length parameters, or numeric range checks. Its sole explicit rejection is the
null guard in `printLine`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` | returns `void` without writing any bytes to stdout [x] |

Generic FFI boundaries: only `printLine` accepts an argument. Zero and
oversized lengths and out-of-range enum values are inapplicable because the API
has no length or enum parameters.
