# Error Surface

The only C rejection expression is the four-part condition in
`parse_val` (`c_src/src/main.c:62`). Each failed operand is listed
independently. `parse_val` is static, so its result is observed through the
exported `main`: rejection prints `An error occurred\n` and `main` returns 0.

| # | function | trigger (the exact invalid input/condition) | expected C result | covered |
|---|----------|---------------------------------------------|-------------------|---------|
| 1 | `parse_val` via `main` | `endp == str`: `strtol` consumes no decimal digits (including empty input/EOF, whitespace-only, sign-only, and a non-digit prefix) | `parse_val` returns `false`; `main` prints `An error occurred\n` and returns 0 | [x] |
| 2 | `parse_val` via `main` | `errno != 0`: base-10 input overflows or underflows C `long` | `parse_val` returns `false`; `main` prints `An error occurred\n` and returns 0 | [x] |
| 3 | `parse_val` via `main` | `tmp < INT_MIN`: converted C `long` is below `-2147483648` without overflowing `long` | `parse_val` returns `false`; `main` prints `An error occurred\n` and returns 0 | [x] |
| 4 | `parse_val` via `main` | `tmp > INT_MAX`: converted C `long` is above `2147483647` without overflowing `long` | `parse_val` returns `false`; `main` prints `An error occurred\n` and returns 0 | [x] |
| 5 | `run` | generic FFI boundary: `the_house == NULL`; C has no null check and dereferences it in `print_house` | process terminates from the same memory-fault signal | [x] |

There are no C assertions, error enums, error-return macros, length
parameters, range options, or out-of-range enum inputs. Consequently the
generic zero/oversized-length and invalid-enum cases are not applicable.
