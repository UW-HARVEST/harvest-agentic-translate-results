# Error surface

Mechanically derived from every `if` guarding a rejection/no-op and every
error sentinel in `../c_src/src/lib.c`. Rows 9-10 are the additional generic
zero/oversized-length boundary checks required for the FFI API.

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|-----------------------------------------|-------------------|--------|
| 1 | `init_array` | allocation of the `DynamicArray` object returns `NULL` | returns `NULL` | [x] |
| 2 | `init_array` | object allocation succeeds, but allocation of `initial_capacity * sizeof(int)` returns `NULL` | frees the object and returns `NULL` | [x] |
| 3 | `expand_array` | `arr == NULL` | returns `0` | [x] |
| 4 | `expand_array` | `realloc(arr->data, arr->capacity * 2 * sizeof(int))` returns `NULL` | returns `0`; original allocation and fields remain valid | [x] |
| 5 | `add_element` | `arr == NULL` | returns `0` | [x] |
| 6 | `add_element` | `arr->size >= arr->capacity` and the nested `expand_array(arr)` returns `0` | returns `0`; no element is appended | [x] |
| 7 | `free_array` | `arr == NULL` | returns normally without freeing or dereferencing anything | [x] |
| 8 | `matrixsum` | its `init_array(2)` call returns `NULL` | returns `-1` | [x] |
| 9 | `init_array` | generic zero-length boundary: `initial_capacity == 0` | returns the same null/non-null sentinel as C; on this glibc build it is non-null with size/capacity zero | [x] |
| 10 | `init_array` | generic oversized-length boundary: `initial_capacity == SIZE_MAX` | data allocation fails and the function returns `NULL` | [x] |

There are no enum-typed parameters, explicit range checks, assertions, error
enums, or documented integer min/max ranges in this C API. All pointer-taking
entry points' null-pointer cases are represented above.

