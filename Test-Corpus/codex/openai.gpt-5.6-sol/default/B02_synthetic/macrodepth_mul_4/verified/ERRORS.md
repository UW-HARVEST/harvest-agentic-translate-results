# Error surface

Mechanically searched `mdcore.c` and `mdmacros.h` for error returns, null
checks, assertions, explicit range rejection, error enums, and min/max checks.
The shared-library API has no rejection or error paths.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|-|

The `use_generated` switch does not reject out-of-range `n`: every value other
than `0` through `6` takes `default` and returns the selected operation's
initial accumulator. Those inputs are valid and are covered in `CONFIGS.md`.

The only rejection in the complete C source tree is in the separate executable
entry point `main`: `argc < 3` prints usage and returns `2`. `main` is not part
of `libmdcore.so` or its FFI surface, so it is outside the shared-library
differential table.

Generic FFI boundary audit: the exported functions take only by-value C
`int`s. There are no pointer, length, or enum parameters, so null pointers,
zero/oversized lengths, and invalid enum discriminants are not applicable.

Phase C status: complete; there are no shared-library rejection rows.
