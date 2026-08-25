# Error Surface

Mechanical searches covered `c_src/include/staticloop.h` and
`c_src/src/staticloop.c` for error returns, `NULL`, assertions, conditionals,
range checks, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

The C public API has no rejection branches or invalid input representations:
both entry points accept one `int` by value. There are no pointers, lengths,
enums, error returns, assertions, explicit ranges, or min/max constants.
Consequently, the mechanically derived error-surface table has zero rows.
Phase C status: **complete** (there are no rejection paths to exercise).

Generic FFI boundary applicability:

- Null pointers: not applicable; neither function accepts a pointer.
- Zero or oversized lengths: not applicable; neither function accepts a length.
- Out-of-range enums: not applicable; neither function accepts an enum.
- Integer boundaries: valid inputs, covered in `CONFIGS.md`.
