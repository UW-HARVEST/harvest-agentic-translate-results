# Exported-symbol parity

Source of truth:

```text
nm -D --defined-only ../c_src/build/libharvest-work-z2k1C9.so
```

The C library has seven exported functions and one exported data object. The
Rust column was measured from `target/release/libmatrixsum_lib.so`.

| C symbol | kind | Rust export | status |
|----------|------|-------------|--------|
| `add_element` | function | `add_element` | [x] |
| `calculate_matrix_checksum` | function | `calculate_matrix_checksum` | [x] |
| `expand_array` | function | `expand_array` | [x] |
| `free_array` | function | `free_array` | [x] |
| `init_array` | function | `init_array` | [x] |
| `matrix` | data object, 48 bytes | `matrix` | [x] |
| `matrixsum` | function | `matrixsum` | [x] |
| `process_flags` | function | `process_flags` | [x] |

Missing C exports in Rust: **0**.

The C library's undefined dynamic references are `malloc`, `realloc`, `free`,
and the weak runtime reference `__cxa_finalize`; all are libc/runtime imports,
not missing library API symbols.
