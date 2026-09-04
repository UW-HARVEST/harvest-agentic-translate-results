# Error-surface table

Mechanically derived from `../c_src/src/lib.c` assertions, null-return paths,
and allocation calls whose result is returned directly.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `w_utf8_drop` | `string == NULL` | `assert(string != NULL)` terminates with `SIGABRT` while assertions are enabled |
| 2 | `w_utf8_filter` | `string == NULL` | `assert(string != NULL)` terminates with `SIGABRT` while assertions are enabled |
| 3 | `w_utf8_filter` | input is entirely valid through its terminating NUL and `strdup(string)` fails | returns `NULL` (the `strdup` result is returned directly) |
| 4 | `w_utf8_filter` | input contains an invalid byte and initial `malloc(strlen(string) + 1)` fails | returns `NULL` |
| 5 | `w_utf8_filter` | input contains an invalid byte, `replacement == true`, `repl < 3`, and `realloc(copy, size + 4096)` fails | returns `NULL` |

## Differential verification

- [x] Row 1: C and Rust terminate with `SIGABRT`.
- [x] Row 2: C and Rust terminate with `SIGABRT`.
- [x] Row 3: forced `strdup` failure returns `NULL` from both libraries.
- [x] Row 4: forced `malloc` failure returns `NULL` from both libraries.
- [x] Row 5: forced `realloc` failure returns `NULL` from both libraries.

## Generic FFI boundaries

The API has no explicit length parameter and no enum parameter. Zero-length
input is the valid empty C string and is covered in `CONFIGS.md`. Long strings
and byte values immediately outside each accepted UTF-8 boundary are also
covered there because malformed UTF-8 is data to filter, not an API error.
