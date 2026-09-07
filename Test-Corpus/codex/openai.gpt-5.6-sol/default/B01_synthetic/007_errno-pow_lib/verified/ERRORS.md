# Error Surface

Mechanically derived from `../c_src/src/pow.c` by enumerating every explicit
error branch and error return. The API has no pointer, length, enum, option,
assertion, or explicit numeric-boundary inputs.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `my_pow` | The call to `pow(base, exponent)` sets `errno == EDOM` | Print `Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n` to `stderr`; return `-1.0` | [x] |
| 2 | `my_pow` | The call to `pow(base, exponent)` sets `errno == ERANGE` | Print `Range error: pow(%.2f, %.2f) caused overflow or underflow.\n` to `stderr`; return `-1.0` | [x] |
