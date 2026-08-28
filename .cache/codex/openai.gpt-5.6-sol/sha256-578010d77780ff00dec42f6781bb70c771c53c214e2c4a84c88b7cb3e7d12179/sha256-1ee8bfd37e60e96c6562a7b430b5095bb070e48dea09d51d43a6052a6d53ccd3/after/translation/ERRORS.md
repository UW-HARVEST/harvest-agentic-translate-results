# Error Surface

Derived mechanically from all rejection branches in `../c_src/src/lib.c`.
There are no error enums, assertions, explicit range checks, or min/max
constants in the public API.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `encode_base64` | `src == NULL` at line 33 | `NULL` | [x] |
| 2 | `encode_base64` | `calloc(sizeof(char), size * 4 / 3 + 4) == NULL` at line 42 | `NULL` | [x] |

All rows pass with default features and `--no-default-features`.
