# Error Surface

Derived from every rejecting branch in `c_src/src/main.c`. `main` is the only
public entry point.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `main` | `argc != 3` | writes `Usage: %s base exponent\n` to stderr and returns `1` | [x] |
| 2 | `main` | after `strtod(argv[1], ...)`, `errno == ERANGE` | writes `Range error while converting base '%s'\n` to stderr and returns `1` | [x] |
| 3 | `main` | base conversion did not set `ERANGE`, but `*endptr1 != '\0'` | writes `Invalid numeric input for base: '%s'\n` to stderr and returns `1` | [x] |
| 4 | `main` | after `strtod(argv[2], ...)`, `errno == ERANGE` | writes `Range error while converting exponent '%s'\n` to stderr and returns `1` | [x] |
| 5 | `main` | exponent conversion did not set `ERANGE`, but `*endptr2 != '\0'` | writes `Invalid numeric input for exponent: '%s'\n` to stderr and returns `1` | [x] |
| 6 | `main` | after `pow(base, exponent)`, `errno == EDOM` | writes `Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n` to stderr and returns `1` | [x] |
| 7 | `main` | after `pow(base, exponent)`, `errno == ERANGE` | writes `Range error: pow(%.2f, %.2f) caused overflow or underflow.\n` to stderr and returns `1` | [x] |

## Generic FFI boundaries

The C API has no enum, buffer-length, or explicit min/max parameter. Its only
size-like value is `argc`; tests must cover zero, negative, and `INT_MAX`.
Tests must also compare C and Rust behavior for a null `argv`, a null element
in `argv`, and extra arguments. Crashing cases must run in isolated child
processes so that their terminating signals can be compared.
