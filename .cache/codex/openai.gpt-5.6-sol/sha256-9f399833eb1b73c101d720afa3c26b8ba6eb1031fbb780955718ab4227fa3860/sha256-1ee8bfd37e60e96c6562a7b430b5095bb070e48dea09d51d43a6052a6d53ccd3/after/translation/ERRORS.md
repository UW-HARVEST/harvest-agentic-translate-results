# Error Surface

Derived from every `return NULL` and its guarding condition in
`../c_src/src/lib.c`. The API performs no explicit null-pointer, range, enum,
or length validation and contains no assertions or error enums.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|-|
| 1 | `searchAndReplace` | `inx_start > 0` and the initial `malloc(inx_start + 1)` returns `NULL` | `NULL` | [x] |
| 2 | `searchAndReplace` | the replacement-copy `realloc(tmp, total_bytes_allocated + value_len)` returns `NULL` | `NULL` | [x] |
| 3 | `searchAndReplace` | a later match has a nonempty gap and the gap-copy `realloc(tmp, total_bytes_allocated + gap)` returns `NULL` | `NULL` | [x] |
| 4 | `searchAndReplace` | the last match has a nonempty suffix and the suffix-copy `realloc(tmp, total_bytes_allocated + orig_len - from)` returns `NULL` | `NULL` | [x] |
| 5 | `searchAndReplace` | no match exists and `strdup(orig)` returns `NULL` | `NULL` returned directly from `strdup` | [x] |

Null `orig`, `search`, or `value` pointers reach `strlen` before any check and
therefore have undefined behavior in C. There are no length parameters, enum
parameters, numeric ranges, or documented min/max constants in the public API.
