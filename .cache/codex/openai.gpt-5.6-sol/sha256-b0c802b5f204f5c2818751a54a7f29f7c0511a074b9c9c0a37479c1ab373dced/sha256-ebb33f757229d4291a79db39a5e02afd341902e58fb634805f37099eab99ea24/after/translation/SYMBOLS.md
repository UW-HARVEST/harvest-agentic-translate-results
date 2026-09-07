# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust `.so` export | Status |
|----------|--------|-------------------|--------|
| `allocate_matrix` | `T` | `allocate_matrix` | present |
| `free_matrix` | `T` | `free_matrix` | present |
| `initialize_matrix_from_string` | `T` | `initialize_matrix_from_string` | present |
| `multiply_matrices` | `T` | `multiply_matrices` | present |
| `matrix_to_string` | `T` | `matrix_to_string` | present |
| `write_to_file` | `T` | `write_to_file` | present |
| `driver` | `T` | `driver` | present |

The C library's undefined dynamic symbols are libc/glibc functions and weak
toolchain runtime hooks only. Missing C-defined symbols in the Rust library: 0.

- [x] Exact C-defined dynamic symbol names are all exported by Rust.
- [x] No undefined non-libc C dependency needs a Rust counterpart.
