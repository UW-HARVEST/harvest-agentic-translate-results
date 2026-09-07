# Error Surface

Mechanically inspected `../c_src/include/driver.h` and
`../c_src/src/driver.c` for error-return macros/statements, assertions,
explicit range checks, null checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are no rejection branches in the C source.

The sole public API is `void driver(float x)`. It accepts its scalar argument
by value and has no pointer, length, enum, option, or documented range, so the
generic null-pointer, zero/oversized-length, and out-of-range-enum probes are
not applicable.

Phase C status: **[x] complete (0 applicable rows)**.
