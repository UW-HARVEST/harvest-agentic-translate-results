# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/release/libmatrixsum_lib.so
```

| C symbol | kind | Rust export | status |
|----------|------|-------------|--------|
| `add_element` | function (`T`) | `add_element` | present |
| `calculate_matrix_checksum` | function (`T`) | `calculate_matrix_checksum` | present |
| `expand_array` | function (`T`) | `expand_array` | present |
| `free_array` | function (`T`) | `free_array` | present |
| `init_array` | function (`T`) | `init_array` | present |
| `matrixsum` | function (`T`) | `matrixsum` | present |
| `process_flags` | function (`T`) | `process_flags` | present |
| `matrix` | writable data (`D`) | `matrix` | present |

Missing C-defined symbols in Rust: **0**.

The C library's undefined symbols are the libc allocator functions `malloc`,
`realloc`, and `free`, plus weak ELF runtime hooks. It has no unresolved
project-defined symbol.
