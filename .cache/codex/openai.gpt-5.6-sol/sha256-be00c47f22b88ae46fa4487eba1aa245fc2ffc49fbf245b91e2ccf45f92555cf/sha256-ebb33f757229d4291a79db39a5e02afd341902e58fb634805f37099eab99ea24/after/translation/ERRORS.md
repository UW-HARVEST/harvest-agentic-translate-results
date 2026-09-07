# Error-Surface Table

Mechanically derived from all `if` and error returns in `src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `custom_strdup` | `str == NULL` | returns `NULL` | [x] |
| 2 | `custom_strdup` | `malloc(strlen(str) + 1) == NULL` | returns `NULL` | [x] |

There are no assertions, enums, explicit numeric range checks, min/max
constants, lengths, or additional error-return statements in the C source.
