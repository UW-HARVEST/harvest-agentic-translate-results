# Error Surface

The public header exposes only `memchra2(int, int, int, int)`. Its arguments
are values, cover the complete C `int` domain, and have no invalid values.
Consequently, null-pointer, length, and invalid-enum tests do not apply to the
public ABI.

The table still inventories every rejection/guard branch mechanically found
in `c_src/src/lib.c`. All listed functions are `static` in the C library, so
the conditions cannot be submitted by an external caller through the default
C shared object. Phase C uses a mechanically built test-only shared object
from the unchanged C source with `static` removed, and calls matching Rust
exports through `libloading`.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `process_buffer` | `buffer == NULL` | `-1` | [x] |
| 2 | `process_buffer` | `*buffer == '\0'` | `-1` | [x] |
| 3 | `process_strings` | `strings == NULL` | `0` | [x] |
| 4 | `process_strings` | `count <= 0` with non-null `strings` | `0` | [x] |
| 5 | `process_strings` | current `*i == NULL` | skip element; final match count excludes it | [x] |
| 6 | `process_strings` | current `**i == '\0'` | skip element; final match count excludes it | [x] |
| 7 | `safe_sum_array` | `arr == NULL` | `0` | [x] |
| 8 | `safe_sum_array` | `size == 0` with non-null `arr` | `0` | [x] |
| 9 | `interpret_as_int` | `bytes == NULL` | `0` | [x] |
| 10 | `interpret_as_int` | `len < sizeof(int)` with non-null `bytes` | `0` | [x] |
| 11 | `count_occurrences` | `text == NULL` | `0` | [x] |
| 12 | `count_occurrences` | `*text == '\0'` | `0` | [x] |
| 13 | `complex_iteration` | `data == NULL` | `-1` | [x] |
| 14 | `complex_iteration` | `count == 0` with non-null `data` | `-1` | [x] |

There are no `assert` statements, error enums, error macros, public pointer
arguments, public lengths, or documented public min/max constraints.

