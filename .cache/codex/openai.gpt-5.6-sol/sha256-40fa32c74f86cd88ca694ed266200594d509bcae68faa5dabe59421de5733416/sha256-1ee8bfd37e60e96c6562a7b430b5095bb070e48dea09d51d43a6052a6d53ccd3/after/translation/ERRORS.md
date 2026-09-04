# Error-surface table

This table is derived from every error return and explicit null check in
`c_src/src/lib.c`. There are no assertions, enums, min/max constants, or
explicit numeric range checks in the C source.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| E1 | `init_array` | allocation of the `DynamicArray` object returns `NULL` (`if (!arr)`) | return `NULL` | [x] |
| E2 | `init_array` | object allocation succeeds but allocation of `initial_capacity * sizeof(int)` returns `NULL` (`if (!arr->data)`) | free the object and return `NULL` | [x] |
| E3 | `expand_array` | `arr == NULL` | return `0` | [x] |
| E4 | `expand_array` | `realloc(arr->data, arr->capacity * 2 * sizeof(int))` returns `NULL` | leave fields unchanged and return `0` | [x] |
| E5 | `add_element` | `arr == NULL` | return `0` | [x] |
| E6 | `add_element` | `arr->size >= arr->capacity` and the nested `expand_array(arr)` returns `0` | do not append and return `0` | [x] |
| E7 | `free_array` | `arr == NULL` (the explicit `if (arr)` null guard) | no-op and return normally | [x] |
| E8 | `matrixsum` | its internal `init_array(2)` returns `NULL` | return `-1` | [x] |

Generic FFI boundaries not represented by C enums:

- `size_t` zero is accepted by `init_array`; its allocator-dependent shape is
  covered in `CONFIGS.md`.
- Oversized `size_t` values exercise E2, E4, and E6.
- `process_flags` accepts every `int`; unknown/high bits and negative values are
  ignored except for the four recognized low bits.
- There are no enum parameters, so an out-of-range enum test is not applicable.
