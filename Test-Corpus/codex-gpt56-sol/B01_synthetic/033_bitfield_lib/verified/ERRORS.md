# Error Surface

Mechanical searches covered `c_src/include/driver.h`,
`c_src/src/driver.c`, and `c_src/CMakeLists.txt` for error returns, `return`
statements, assertions, null/range checks, error constants, conditionals, and
switches.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

The C source contains no explicit rejection or error path, so there are no
error-surface rows to check.

## Generic FFI Boundaries

These are required boundary probes even though the C implementation does not
explicitly reject them.

| # | function | boundary input | expected C result | status |
|---|----------|----------------|-------------------|--------|
| G1 | `print_foo` | null `foo` pointer | process terminates with `SIGSEGV` when the fields are read | [x] |
| G2 | `driver` | noncanonical raw `_Bool` ABI byte `2`, one past the valid C `bool` value range | accepted; stored one-bit field is `2 & 1`, producing `0` | [x] |
| G3 | `driver` | noncanonical raw `_Bool` ABI byte `255` | accepted; stored one-bit field is `255 & 1`, producing `1` | [x] |

There are no lengths, enums, nullable pointers on `driver`, or documented
numeric ranges narrower than the C parameter types. Zero, integer extrema, and
bit-field overflow/truncation are valid inputs and are covered by
`CONFIGS.md`.
