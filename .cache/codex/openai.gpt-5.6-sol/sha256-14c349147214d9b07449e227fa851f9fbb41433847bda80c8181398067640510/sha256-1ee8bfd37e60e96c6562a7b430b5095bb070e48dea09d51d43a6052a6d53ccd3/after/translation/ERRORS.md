# Error Surface

Mechanically derived from every null check and error return in
`../c_src/src/lib.c`. There are no assertions, enums, range constants, or
length parameters in the public API.

| # | function | trigger (the exact invalid input/condition) | expected C result | Covered |
|---|----------|---------------------------------------------|-------------------|---------|
| 1 | `decode_base64` | `src == NULL` | `NULL` | [x] |
| 2 | `decode_base64` | `src != NULL && *src == '\0'` | `NULL` | [x] |
| 3 | `decode_base64` | `calloc(sizeof(char), strlen(src) + 14)` returns `NULL` | `NULL` | [x] |
| 4 | `decode_base64` | destination allocation succeeds, then `malloc(strlen(src) + 1)` returns `NULL` | frees destination and returns `NULL` | [x] |

Generic FFI boundaries:

| # | function | boundary | C handling | Covered |
|---|----------|----------|------------|---------|
| 5 | `decode_base64` | zero-length input | Same as row 2 | [x] |
| 6 | `decode_base64` | oversized/long NUL-terminated input | No rejection or maximum; decode normally | [x] |
| 7 | `decode_base64` | out-of-range enum value | Not applicable: the API has no enum parameter | [x] |
