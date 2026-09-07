# Error Surface

Mechanically inspected `../c_src/include/driver.h` and
`../c_src/src/driver.c` for error returns, error macros, assertions, null
checks, range checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | Verified |
|---|----------|----------------------------------------------|-------------------|----------|

There are no rejection paths. Both exported functions accept a `char` by
value, return `void`, and perform no validation. Pointer, length, enum, and
option error boundaries do not exist in this API.

Phase C status: complete (zero mechanically discovered rows).
