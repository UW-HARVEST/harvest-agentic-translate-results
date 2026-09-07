# Error-Surface Table

Mechanically derived from null checks, range checks, failure sentinels, and
failure branches in `c_src/src/lib.c`. There are no assertions or enum-typed
parameters in the C API.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `is_string_empty` | `str == NULL` | `1` | [x] |
| 2 | `find_char_in_buffer` | `buffer == NULL` (including `size == 0` and nonzero sizes) | `NULL` | [x] |
| 3 | `create_buffer` | `initial == NULL` | `NULL` | [x] |
| 4 | `create_buffer` | `malloc(strlen(initial) + 1) == NULL` | `NULL` | [x] |
| 5 | `validate_uint16_range` | `value < 0` (boundary: `-1`) | `0` | [x] |
| 6 | `validate_uint16_range` | `value > UINT16_MAX` (boundary: `65536`) | `0` | [x] |
| 7 | `apply_operation` | `op == NULL` | `-1` | [x] |
| 8 | `charinbuf` mode `0` | `value < 0`, so `validate_uint16_range(value) == 0` | `-1` | [x] |
| 9 | `charinbuf` mode `0` | `value > UINT16_MAX`, so `validate_uint16_range(value) == 0` | `-1` | [x] |
| 10 | `charinbuf` mode `2` | `create_buffer("Testing malloc and free") == NULL` | `-1` | [x] |
| 11 | `charinbuf` mode `4` | `create_buffer("Search for character X in this buffer") == NULL` | `0` (the initialized result is retained) | [x] |
| 12 | `charinbuf` mode `4` | `find_char_in_buffer(...) == NULL` | `-1` | [x] |
| 13 | `charinbuf` | `mode` is not `0`, `1`, `2`, `3`, or `4` | `-1` | [x] |

Generic FFI boundaries additionally covered by tests: zero-length buffers,
lengths at the allocated-buffer boundary, null pointers, signed integer
extremes, and mode values immediately outside the valid range. The API has no
public enum parameters.
