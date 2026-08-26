# Error Surface

Derived from every rejection branch in `c_src/src/pow.c`. The public API has no
pointers, lengths, enums, or explicit input range constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|-|
| 1 | `my_pow` | `pow(base, exponent)` leaves `errno == EDOM` | Write the domain-error message to `stderr`; return `-1.0` | [x] |
| 2 | `my_pow` | `pow(base, exponent)` leaves `errno == ERANGE` (overflow or underflow) | Write the range-error message to `stderr`; return `-1.0` | [x] |
