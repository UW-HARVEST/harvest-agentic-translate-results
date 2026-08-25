# Error Surface

Derived from every `assert`, allocation failure, allocation null check, and
`return NULL` branch in `c_src/src/lib.c`. The API has no length arguments,
enums, or numeric range parameters.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| E01 | `w_utf8_drop` | `string == NULL` | `assert(string != NULL)` terminates with `SIGABRT` | [x] |
| E02 | `w_utf8_filter` | `string == NULL` | `assert(string != NULL)` terminates with `SIGABRT` | [x] |
| E03 | `w_utf8_filter` | input contains an invalid byte and the initial `malloc(strlen(string) + 1)` returns `NULL` | returns `NULL` | [x] |
| E04 | `w_utf8_filter` | `replacement == true`, an invalid byte is reached with `repl < 3`, and `realloc(copy, size + 4096)` returns `NULL` | returns `NULL` | [x] |
| E05 | `w_utf8_filter` | input is wholly valid and `strdup(string)` returns `NULL` | returns `NULL` through `copy` | [x] |

Generic FFI boundaries: E01 and E02 cover null pointers. An empty C string
covers zero input length in C01. There is no caller-supplied length, enum, or
documented numeric range, so oversized lengths and out-of-range enum values are
not representable API inputs.
