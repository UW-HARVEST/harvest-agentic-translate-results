# Error Surface

Mechanically derived from every `return NULL` path and its guarding condition
in `c_src/src/lib.c`. The public header declares no enums, length parameters,
or min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `decode_base64` | `src == NULL` at line 46 | `NULL` | [x] |
| 2 | `decode_base64` | `src != NULL && *src == '\0'` at line 46 | `NULL` | [x] |
| 3 | `decode_base64` | `calloc(sizeof(char), strlen(src) + 14)` returns `NULL` at lines 53-55 | `NULL` | [x] |
| 4 | `decode_base64` | destination allocation succeeds, then `malloc(strlen(src) + 1)` returns `NULL` at lines 60-63 | destination is freed; returns `NULL` | [x] |

There are no assertions, error enums/codes, range checks, public enum inputs,
or explicit min/max constants. This API accepts a NUL-terminated string rather
than a pointer/length pair, so zero length is row 2 and there is no caller-
supplied length or enum value to test one past a declared range.
