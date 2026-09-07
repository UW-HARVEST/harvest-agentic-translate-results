# Error surface

Mechanically derived from every rejecting branch in `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `decode_base64` | `src == NULL` | returns `NULL` | [x] |
| 2 | `decode_base64` | `src != NULL` and `*src == '\0'` | returns `NULL` | [x] |
| 3 | `decode_base64` | `calloc(sizeof(char), strlen(src) + 1 + 13)` returns `NULL` | returns `NULL` | [x] |
| 4 | `decode_base64` | destination allocation succeeds, then `malloc(strlen(src) + 1)` returns `NULL` | frees the destination and returns `NULL` | [x] |

No assertions, enums, explicit numeric range checks, length parameters, or
documented min/max constants exist in the public C API.
