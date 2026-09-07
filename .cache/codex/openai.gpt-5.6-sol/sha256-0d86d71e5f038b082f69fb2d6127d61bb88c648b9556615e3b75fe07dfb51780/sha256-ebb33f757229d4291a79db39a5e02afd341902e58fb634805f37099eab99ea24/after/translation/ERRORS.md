# Error Surface

Mechanically inspected `../c_src/include/sieve.h` and
`../c_src/src/sieve.c` for error-return statements/macros, assertions, null
checks, range checks, min/max constants, and error enums.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|

There are no rows: the sole API is `void sieve(int)`, and the C source has no
rejection or error path. It accepts no pointers, lengths, enums, modes, or
options, and documents no restricted integer subrange.

Generic FFI-boundary audit:

- Null pointers: not applicable; there are no pointer parameters.
- Zero length: not applicable; there are no length parameters.
- Oversized length: not applicable; there are no length parameters.
- Out-of-range enum: not applicable; there are no enum parameters.
- One past documented range: not applicable; no integer subrange is documented.
- Integer zero is a valid configuration and is covered in `CONFIGS.md`.

