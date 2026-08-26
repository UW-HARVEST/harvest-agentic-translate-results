# Error Surface

Mechanically derived from the compound rejection at
`c_src/src/driver.c:64-69`, plus the mandatory generic FFI null-boundary
checks. `parse_val` is static, so its observable result is tested through
`driver`.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|-|
| E1 | `driver` / `parse_val` | `endp == str`: `strtol` consumes no decimal digits (empty, whitespace-only, or nonnumeric input) | prints `An error occurred\n`; returns `void` | [x] |
| E2 | `driver` / `parse_val` | `errno != 0`: base-10 value overflows or underflows `long` | prints `An error occurred\n`; returns `void` | [x] |
| E3 | `driver` / `parse_val` | `errno == 0 && tmp < INT_MIN` | prints `An error occurred\n`; returns `void` | [x] |
| E4 | `driver` / `parse_val` | `errno == 0 && tmp > INT_MAX` | prints `An error occurred\n`; returns `void` | [x] |
| E5 | `driver` | `in == NULL` (generic pointer boundary; C has no null check) | process terminates with `SIGSEGV` | [x] |
| E6 | `run` | `the_house == NULL` (generic pointer boundary; C has no null check) | process terminates with `SIGSEGV` | [x] |

There are no length parameters, public enum parameters, error enums, asserts,
error-return macros, `return -1`, or `return NULL` statements. Empty input is
the applicable zero-length boundary (E1); oversized decimal input is the
applicable oversized-input boundary (E2); one-past-range values are E3 and E4.
