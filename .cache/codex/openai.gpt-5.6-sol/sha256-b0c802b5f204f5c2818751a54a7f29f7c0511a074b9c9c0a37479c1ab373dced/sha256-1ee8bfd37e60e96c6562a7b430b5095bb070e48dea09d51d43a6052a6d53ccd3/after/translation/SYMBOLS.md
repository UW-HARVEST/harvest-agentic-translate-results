# Dynamic Symbol Surface

Source: `nm -D --defined-only ../c_src/build/libdriver.so`.

| # | symbol | C type | Rust export |
|---|--------|--------|-------------|
| 1 | `allocate_matrix` | `T` | present |
| 2 | `driver` | `T` | present |
| 3 | `free_matrix` | `T` | present |
| 4 | `initialize_matrix_from_string` | `T` | present |
| 5 | `matrix_to_string` | `T` | present |
| 6 | `multiply_matrices` | `T` | present |
| 7 | `write_to_file` | `T` | present |

Missing C symbols in Rust: **0**.

The C library's undefined dynamic symbols are libc/runtime imports
(`__errno_location`, `atoi`, `fclose`, `fopen`, `fprintf`, `free`, `fwrite`,
`malloc`, `perror`, `snprintf`, `stderr`, `strcat`, `strdup`, `strerror`,
`strlen`, and `strtok_r`) plus weak toolchain hooks. They are not library API
exports.
