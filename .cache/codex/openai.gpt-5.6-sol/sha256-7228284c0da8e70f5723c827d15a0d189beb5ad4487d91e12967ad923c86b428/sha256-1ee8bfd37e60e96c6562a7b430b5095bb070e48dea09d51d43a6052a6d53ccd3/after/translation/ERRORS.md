# Error Surface

The C source contains no error-return statements, error enums, assertions,
explicit range checks, pointer checks, length checks, or min/max constants.
The only invalid input conditions are the two invalid signed-division cases
passed directly to libc `div` at `src/driver.c:30`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `driver` | `y == 0` | calling process terminates with `SIGFPE` | [x] |
| 2 | `driver` | `x == INT_MIN && y == -1` (signed quotient overflow) | calling process terminates with `SIGFPE` | [x] |

The API has no pointers, lengths, enums, modes, or documented narrower integer
range, so no null-pointer, oversized-length, invalid-enum, or one-past-range
cases can be represented at its FFI boundary.
