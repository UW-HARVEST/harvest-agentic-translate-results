# Error Surface

Mechanical scan scope: `../c_src/include/lib.h` and `../c_src/src/lib.c`.
The source contains no `RETURN_ERROR`, `assert`, error enum, explicit input
range check, or input-pointer null check. It has one rejection path.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| E01 | `find_value_in_buffer` | `memchr(buffer, (char)search_val, size)` returns `NULL` because the target byte does not occur in the first `size` bytes (including `size == 0`) | `-1` | [x] |

## Unchecked Inputs

The C implementation does not reject a null `buffer` with positive `size`;
dereferencing that input is outside the C API's defined domain. It also does
not reject oversized lengths, negative buffer-generation sizes, out-of-range
integers, or any integer value as an enum because the API defines no enums.
Safe boundary forms are covered in `CONFIGS.md`.
