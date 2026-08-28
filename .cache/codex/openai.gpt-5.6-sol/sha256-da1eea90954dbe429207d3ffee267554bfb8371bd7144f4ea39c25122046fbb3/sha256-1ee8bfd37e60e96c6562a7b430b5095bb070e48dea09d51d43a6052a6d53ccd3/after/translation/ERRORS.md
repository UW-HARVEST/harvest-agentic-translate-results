# Error Surface

Mechanically derived from the null checks, range checks, error sentinels, and
failure branches in `../c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| - [x] 1 | `is_string_empty` | `str == NULL` | `1` |
| - [x] 2 | `find_char_in_buffer` | `buffer == NULL` (including zero or oversized `size`) | `NULL` |
| - [x] 3 | `create_buffer` | `initial == NULL` | `NULL` |
| - [x] 4 | `create_buffer` | `malloc(strlen(initial) + 1) == NULL` | `NULL` |
| - [x] 5 | `validate_uint16_range` | `value < 0` | `0` |
| - [x] 6 | `validate_uint16_range` | `value > UINT16_MAX` | `0` |
| - [x] 7 | `apply_operation` | `op == NULL` | `-1` |
| - [x] 8 | `charinbuf` | `mode == 0 && value < 0` | `-1` |
| - [x] 9 | `charinbuf` | `mode == 0 && value > UINT16_MAX` | `-1` |
| - [x] 10 | `charinbuf` | `mode == 2 && create_buffer(...) == NULL` | `-1` |
| - [x] 11 | `charinbuf` | `mode` is outside `0..=4` (including one-step-past and arbitrary `int`) | `-1` |

There are no `assert` statements, error enums, error macros, or compile-time
min/max configuration branches in the C source. In mode 4, allocation failure
is a distinct branch but is not an error result: the initialized result `0` is
returned. That valid environmental branch is tracked in `CONFIGS.md`.
