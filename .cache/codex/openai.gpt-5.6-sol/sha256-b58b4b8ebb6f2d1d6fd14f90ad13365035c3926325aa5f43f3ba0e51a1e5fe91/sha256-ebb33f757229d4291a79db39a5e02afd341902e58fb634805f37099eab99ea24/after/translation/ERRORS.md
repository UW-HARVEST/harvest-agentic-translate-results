# Error Surface

Mechanically derived by searching `c_src/include` and `c_src/src` for error
returns, assertions, null/range checks, and min/max constants. The C source has
no error-return macros, error enums, assertions, explicit range checks, or
min/max constants. Its sole input rejection is the null check below.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` | Return `void` without writing any bytes to stdout. [x] |

Generic FFI boundaries:

- `driver(int)`: every representable C `int` is accepted; there is no invalid
  enum value or out-of-range integer branch. Zero selects `bad`; every nonzero
  value selects `good`.
- `bad()` and `good()`: no inputs.
- No API takes a length, so zero/oversized-length cases do not exist.
