# Error surface

This table is derived from every input-rejection branch, fallback branch,
allocation null check, and error return in `../c_src/src/lib.c`. The source has
no assertions, error enums, or named minimum/maximum constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| E01 | `shift_array` | `positions <= 0` (so `positions > 0 && positions < size` is false) | return `void`; array is byte-for-byte unchanged | [x] |
| E02 | `shift_array` | `positions > 0 && positions >= size` | return `void`; array is byte-for-byte unchanged | [x] |
| E03 | `compare_allocations` | first `malloc(sizeof(int))` returns `NULL`, second succeeds | free both values and return `-1` | [x] |
| E04 | `compare_allocations` | first allocation succeeds, second `malloc(sizeof(int))` returns `NULL` | free both values and return `-1` | [x] |
| E05 | `compare_allocations` | both `malloc(sizeof(int))` calls return `NULL` | free both null values and return `-1` | [x] |
| E06 | `apply_bitmask` | `operation < 0`, which matches no `case` | return the input `value` unchanged | [x] |
| E07 | `apply_bitmask` | `operation > 3`, which matches no `case` | return the input `value` unchanged | [x] |
| E08 | `arity` | low 8 bits of `len` are `0` (`unsigned char len < 2`) | return `-1` without reading `params` | [x] |
| E09 | `arity` | low 8 bits of `len` are `1` (`unsigned char len < 2`) | return `-1` without reading `params` | [x] |

Generic FFI boundary cases not represented as C rejection branches are tested
separately: null pointers on inactive paths, null pointers on dereferencing
paths (matching process termination), zero/negative/oversized lengths, one past
the effective arity, and extreme out-of-range operation values.
