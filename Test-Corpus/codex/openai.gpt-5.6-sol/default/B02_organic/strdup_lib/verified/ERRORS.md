# Error Surface

Mechanically derived from all null checks and error returns in
`../c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `custom_strdup` | `str == NULL` | returns `NULL` without calling `strlen` or `malloc` | [x] |
| 2 | `custom_strdup` | `str != NULL` and `malloc(strlen(str) + 1) == NULL` | returns `NULL` | [x] |

No assertions, enums, explicit numeric range checks, min/max constants, length
parameters, or other error-return statements exist in the public C surface.

Generic boundary coverage maps as follows: null pointers are row 1; a
zero-length C string is valid and covered by `CONFIGS.md` row 1; a 1 MiB C
string is covered by `CONFIGS.md` row 3. There are no length arguments,
documented numeric ranges, or enum arguments for oversized, one-past-range, or
out-of-range-enum calls.
