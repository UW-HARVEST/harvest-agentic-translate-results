# Dynamic symbol surface

Mechanically derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-c3ErL0.so
nm -D --defined-only target/release/libmatrixsum_lib.so
```

| # | symbol | C kind | Rust kind | status |
|---|--------|--------|-----------|--------|
| 1 | `add_element` | `T` | `T` | [x] |
| 2 | `calculate_matrix_checksum` | `T` | `T` | [x] |
| 3 | `expand_array` | `T` | `T` | [x] |
| 4 | `free_array` | `T` | `T` | [x] |
| 5 | `init_array` | `T` | `T` | [x] |
| 6 | `matrix` | `D` | `D` | [x] |
| 7 | `matrixsum` | `T` | `T` | [x] |
| 8 | `process_flags` | `T` | `T` | [x] |

The C DSO's undefined entries are the libc allocation functions `malloc`,
`realloc`, and `free`, plus weak ELF/toolchain hooks. They are imports, not
library exports. The Rust DSO has no missing project-defined symbol.

