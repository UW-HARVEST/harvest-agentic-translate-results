# Error Surface

Mechanical scan scope: `c_src/include/driver.h` and `c_src/src/driver.c`.

Searches covered error-return statements/macros, `assert`, null checks, range
checks, min/max constants, conditionals, and enums. The only C condition is the
private `print_hex` loop bound (`i < len`), where the sole caller always passes
`sizeof(float)`. The public API is `void driver(float)` and has no pointers,
lengths, enums, return value, rejection path, assertion, or invalid input
representation.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

Distinct C rejection branches: **0**. Consequently, Phase C has no error rows
to check and no generic pointer, length, or enum boundaries applicable to this
API.
