# Error surface

Mechanical searches covered `RETURN_ERROR`, negative and null returns,
assertions, explicit `if`/`switch` checks, range comparisons, and min/max
constants in `c_src/include` and `c_src/src`. The C source contains no explicit
input rejection or error-return path.

The following row records the generic pointer boundary required by the
verification protocol. It is not a checked C error: the C implementation
dereferences the pointer directly, so the observed contract for this build is
process termination by `SIGSEGV`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `print_foo` | `foo == NULL` | [x] no return; child process terminates with `SIGSEGV` |

There are no length parameters, array counts, documented numeric ranges, or
enum parameters in this API, so zero/oversized lengths and invalid enum
discriminants are not applicable.
