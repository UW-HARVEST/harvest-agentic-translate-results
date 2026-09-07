# Error-surface table

Mechanically derived from every rejection/sentinel branch in
`../c_src/src/lib.c`. There are no assertions, enums, explicit range checks,
or min/max constants in the C source.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `encode_base64` | `src == NULL` (`if (!src)`) | returns `NULL` | [x] |
| 2 | `encode_base64` | `calloc(1, size * 4 / 3 + 4)` returns `NULL` (`if (!out)`) | returns `NULL` | [x] |
