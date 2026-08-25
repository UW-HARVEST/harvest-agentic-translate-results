# Error Surface

Mechanically derived from every `if` guarding a null pointer or failed
allocation and every error/sentinel return in `c_src/src/lib.c`. There are no
assertions, enums, explicit numeric range checks, or min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| E01 | `init_array` | `malloc(sizeof(DynamicArray))` returns `NULL` | returns `NULL` | [x] |
| E02 | `init_array` | the data `malloc(initial_capacity * sizeof(int))` returns `NULL` after the structure allocation succeeds | frees the structure and returns `NULL` | [x] |
| E03 | `expand_array` | `arr == NULL` | returns `0` | [x] |
| E04 | `expand_array` | `realloc(arr->data, arr->capacity * 2 * sizeof(int))` returns `NULL` | leaves the structure fields unchanged and returns `0` | [x] |
| E05 | `add_element` | `arr == NULL` | returns `0` | [x] |
| E06 | `add_element` | `arr->size >= arr->capacity` and its `expand_array(arr)` call returns `0` | leaves `size` unchanged and returns `0` | [x] |
| E07 | `free_array` | `arr == NULL` | no operation; returns `void` | [x] |
| E08 | `matrixsum` | `init_array(2)` returns `NULL` | returns `-1` | [x] |

## Generic FFI Boundaries

These are required boundary probes even though the C source does not define
additional rejection branches for them.

| # | API boundary | probe | expected comparison | tested |
|---|--------------|-------|---------------------|--------|
| B01 | `init_array` length | zero capacity | identical pointer-nullness and structure fields | [x] |
| B02 | `init_array` length | `SIZE_MAX` capacity | identical `NULL` allocation result | [x] |
| B03 | `init_array` length arithmetic | `SIZE_MAX / sizeof(int) + 1`, whose C `size_t` byte count wraps | identical pointer-nullness and structure fields | [x] |
| B04 | `expand_array` length | zero capacity, causing `realloc(ptr, 0)` | identical return sentinel and resulting fields | [x] |
| B05 | all pointer-taking exports | null pointers to `expand_array`, `add_element`, and `free_array` | exact C sentinel/no-op behavior | [x] |

There are no enum parameters or documented finite numeric ranges, so there is
no out-of-range enum or one-past-documented-range probe to construct.
