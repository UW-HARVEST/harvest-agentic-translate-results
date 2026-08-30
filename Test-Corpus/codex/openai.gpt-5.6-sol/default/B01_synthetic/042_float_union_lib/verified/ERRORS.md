# Error Surface

Mechanical searches of `../c_src/include/driver.h` and
`../c_src/src/driver.c` found no error-return statements, assertions, range
checks, null checks, enums, lengths, or min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

`driver` accepts one `double` by value. Every binary64 bit pattern is accepted,
so the generic pointer, length, and out-of-range enum boundaries do not apply.
