# Error Surface

Mechanically derived from the public declaration in `c_src/include/lib.h` and
the `default` branch of `get_predict_func` in `c_src/src/lib.c`. The public API
has no pointer, length, struct, or enum arguments, no assertions, and no
`RETURN_ERROR`, `return -1`, or `return NULL` paths.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `get_predict_func` | `pfcn < 0` or `pfcn > 11` (the `default` branch, including 12–15 and all other `int` values) | returns `0` | [x] |
