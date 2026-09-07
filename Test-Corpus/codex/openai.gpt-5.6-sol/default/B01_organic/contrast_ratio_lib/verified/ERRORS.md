# Error-surface table

Mechanical audit of `../c_src/include/lib.h` and `../c_src/src/lib.c` found:

- no error-return macro or error enum;
- no `return -1`, `return NULL`, assertion, null check, or range rejection;
- no pointer, length, option, or enum parameter in the public API;
- all bit patterns of each `unsigned char` channel are valid inputs.

Consequently the C API has no rejection rows:

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

## Generic FFI boundary audit

| boundary class | applicability | coverage |
|---|---|---|
| null pointers | not applicable: both arguments are structs passed by value | [x] |
| zero lengths | not applicable: there is no length parameter | [x] |
| oversized lengths | not applicable: there is no length parameter | [x] |
| out-of-range enum values | not applicable: there is no enum parameter | [x] |
| channel value below minimum | impossible for `unsigned char` | [x] |
| channel value above maximum | impossible for `unsigned char` | [x] |

Phase C completion check: [x] every error-surface row is covered (zero rows).
