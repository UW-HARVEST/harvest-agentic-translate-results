# Error Surface

Mechanically derived from the two explicit rejection branches in
`c_src/src/pow.c`. The public API has scalar `double` arguments, so pointer,
length, and enum boundary cases do not apply.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `my_pow` | `pow(base, exponent)` sets `errno == EDOM` | returns `-1.0` after writing the domain diagnostic to `stderr` | [x] |
| 2 | `my_pow` | `pow(base, exponent)` sets `errno == ERANGE` (including overflow, underflow, and pole cases) | returns `-1.0` after writing the range diagnostic to `stderr` | [x] |
