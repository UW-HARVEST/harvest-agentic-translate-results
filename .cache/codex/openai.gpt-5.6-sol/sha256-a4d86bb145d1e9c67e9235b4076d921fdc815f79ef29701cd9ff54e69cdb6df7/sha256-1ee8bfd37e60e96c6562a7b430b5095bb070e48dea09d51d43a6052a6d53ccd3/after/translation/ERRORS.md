# Error surface

Rows are derived from every `assert` and allocation-failure result in
`../c_src/src/lib.c`. The API has no length parameters, enums, error enums,
range checks, or min/max input constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `w_utf8_drop` | `string == NULL` (`assert` at line 40) | process terminates with `SIGABRT` | [x] |
| 2 | `w_utf8_filter` | `string == NULL` (`assert` at line 60) | process terminates with `SIGABRT` | [x] |
| 3 | `w_utf8_filter` | input is entirely valid and `strdup(string)` cannot allocate | returns `NULL` | [x] |
| 4 | `w_utf8_filter` | input contains invalid UTF-8 and initial `malloc(strlen(string) + 1)` returns `NULL` | returns `NULL` | [x] |
| 5 | `w_utf8_filter` | `replacement == true`, an invalid byte is reached, and `realloc(copy, size + 4096)` returns `NULL` | returns `NULL` | [x] |

An empty C string is the zero-length boundary and is valid. There is no
caller-provided length or enum, so no one-past length/enum rejection exists.
