# Error Surface

This table is derived from every null check, range rejection, failure return,
and invalid `switch` branch in `c_src/src/lib.c`. There are no assertions or
error enums.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] 1 | `is_string_empty` | `str == NULL` | `1` |
| [x] 2 | `find_char_in_buffer` | `buffer == NULL` (for any `size` and `target`) | `NULL` |
| [x] 3 | `create_buffer` | `initial == NULL` | `NULL` |
| [x] 4 | `create_buffer` | `malloc(strlen(initial) + 1) == NULL` | `NULL` |
| [x] 5 | `validate_uint16_range` | `value < 0` | `0` |
| [x] 6 | `validate_uint16_range` | `value > UINT16_MAX` (`65535`) | `0` |
| [x] 7 | `apply_operation` | `op == NULL` | `-1` |
| [x] 8 | `charinbuf` mode `0` | `value < 0`, so `validate_uint16_range(value) == 0` | `-1` |
| [x] 9 | `charinbuf` mode `0` | `value > UINT16_MAX`, so `validate_uint16_range(value) == 0` | `-1` |
| [x] 10 | `charinbuf` mode `2` | internal `create_buffer("Testing malloc and free") == NULL` | `-1` |
| [x] 11 | `charinbuf` mode `4` | internal `create_buffer("Search for character X in this buffer") == NULL` | `0` (the initialized `result`) |
| [x] 12 | `charinbuf` mode `4` | allocation succeeds but internal `find_char_in_buffer(..., 'X') == NULL` | `-1` |
| [x] 13 | `charinbuf` | `mode` is not `0`, `1`, `2`, `3`, or `4` | `-1` |
