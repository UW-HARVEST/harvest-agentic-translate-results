# Dynamic Symbol Surface

Source library: `c_src/build/libharvest-work-IPImlt.so`

Command: `nm -D --defined-only c_src/build/libharvest-work-IPImlt.so`

| C symbol | Kind | Size / ABI | Rust export | Status |
|----------|------|------------|-------------|--------|
| `add_element` | function | `int (DynamicArray *, int)` | `add_element` | present |
| `calculate_matrix_checksum` | function | `int (void)` | `calculate_matrix_checksum` | present |
| `expand_array` | function | `int (DynamicArray *)` | `expand_array` | present |
| `free_array` | function | `void (DynamicArray *)` | `free_array` | present |
| `init_array` | function | `DynamicArray *(size_t)` | `init_array` | present |
| `matrix` | object | 48 bytes (`int[3][4]`) | `matrix` | present |
| `matrixsum` | function | `int (int, int, int, int)` | `matrixsum` | present |
| `process_flags` | function | `int (int)` | `process_flags` | present |

The C library's undefined dynamic symbols are allocator/runtime imports:
`malloc`, `realloc`, `free`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`. It has no
undefined project-library symbols.

Completion check: [x] exact defined-symbol diff is empty and Rust has no
missing C project symbol.
