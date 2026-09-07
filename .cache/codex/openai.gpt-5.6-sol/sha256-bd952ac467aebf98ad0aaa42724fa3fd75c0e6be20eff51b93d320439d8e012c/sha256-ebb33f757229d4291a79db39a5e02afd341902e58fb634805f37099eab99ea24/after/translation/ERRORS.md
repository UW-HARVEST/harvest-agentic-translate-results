# Error surface

The only public entry point is `memchra2(int, int, int, int)`. It accepts four
by-value C integers, has no error return contract, and contains no public input
rejection. Consequently, null pointers, lengths, and enum discriminants cannot
be supplied through this ABI.

The table below is the complete mechanical inventory of rejection/skip checks
in `c_src/src/lib.c`. Every row belongs to a `static` helper and is unreachable
through the production dynamic ABI: `nm -D --defined-only` exports only
`memchra2`. Phase C uses test-only C and Rust adapter shared libraries to expose
the private helpers, loads both adapters with `libloading`, and compares every
exact result. It also verifies that the helpers remain absent from both
production dynamic symbol tables and compares all public boundary values.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] E01 | `process_buffer` | `buffer == NULL` | returns `-1` |
| [x] E02 | `process_buffer` | `*buffer == '\0'` | returns `-1` |
| [x] E03 | `process_strings` | `strings == NULL` | returns `0` |
| [x] E04 | `process_strings` | `count <= 0` | returns `0` |
| [x] E05 | `process_strings` | current element `*i == NULL` | skips that element and continues |
| [x] E06 | `process_strings` | current element `**i == '\0'` | skips that element and continues |
| [x] E07 | `safe_sum_array` | `arr == NULL` | returns `0` |
| [x] E08 | `safe_sum_array` | `size == 0` | returns `0` |
| [x] E09 | `interpret_as_int` | `bytes == NULL` | returns `0` |
| [x] E10 | `interpret_as_int` | `len < sizeof(int)` | returns `0` |
| [x] E11 | `count_occurrences` | `text == NULL` | returns `0` |
| [x] E12 | `count_occurrences` | `*text == '\0'` | returns `0` |
| [x] E13 | `complex_iteration` | `data == NULL` | returns `-1` |
| [x] E14 | `complex_iteration` | `count == 0` | returns `-1` |

No `assert`, error enum, `RETURN_ERROR`, public range check, public minimum or
maximum, pointer parameter, length parameter, or enum parameter exists.
