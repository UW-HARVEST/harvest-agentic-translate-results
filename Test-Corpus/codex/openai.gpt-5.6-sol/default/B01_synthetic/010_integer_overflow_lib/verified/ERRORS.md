# Error Surface

Mechanical searches of `c_src/include/driver.h` and `c_src/src/driver.c` found
no error-return statements or macros, assertions, explicit range checks, null
checks, min/max constants, pointer or length parameters, or enum parameters.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

There are no rejection rows. Both C functions take every `char` bit pattern by
value and return `void`. Generic pointer, length, and invalid-enum boundaries
do not apply to this API.

