# Error Surface

Rows are mechanically derived from each rejection branch in `src/lib.c`.
Allocator-failure rows require deterministic allocator fault injection.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `init_array` | allocation of the `DynamicArray` object returns `NULL` | returns `NULL` | [x] |
| 2 | `init_array` | object allocation succeeds but allocation of `initial_capacity * sizeof(int)` returns `NULL` | frees the object and returns `NULL` | [x] |
| 3 | `expand_array` | `arr == NULL` | returns `0` | [x] |
| 4 | `expand_array` | `realloc(arr->data, arr->capacity * 2 * sizeof(int))` returns `NULL` | leaves fields unchanged and returns `0` | [x] |
| 5 | `add_element` | `arr == NULL` | returns `0` | [x] |
| 6 | `add_element` | `arr->size >= arr->capacity` and the nested `expand_array(arr)` returns `0` | does not append and returns `0` | [x] |
| 7 | `matrixsum` | its `init_array(2)` call returns `NULL` | returns `-1` | [x] |

No C `assert`, enum rejection, explicit range rejection, or min/max constant
exists in this source.
