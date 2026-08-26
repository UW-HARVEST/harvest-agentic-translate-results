# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

| C symbol | Rust symbol | Status |
|----------|-------------|--------|
| `allocate_matrix` | `allocate_matrix` | present |
| `driver` | `driver` | present |
| `free_matrix` | `free_matrix` | present |
| `initialize_matrix_from_string` | `initialize_matrix_from_string` | present |
| `matrix_to_string` | `matrix_to_string` | present |
| `multiply_matrices` | `multiply_matrices` | present |
| `write_to_file` | `write_to_file` | present |

Missing C symbols in Rust: **0**

Undefined non-system symbols in Rust: **0**
