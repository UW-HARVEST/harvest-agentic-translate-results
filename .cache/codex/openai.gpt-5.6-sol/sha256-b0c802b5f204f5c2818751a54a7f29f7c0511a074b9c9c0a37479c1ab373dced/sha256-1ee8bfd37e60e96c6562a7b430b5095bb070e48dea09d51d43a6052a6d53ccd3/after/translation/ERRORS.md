# Error Surface

Each row corresponds to a distinct rejection branch in the C source. Allocation
failure rows use dimensions that request an unallocatable object on the test
platform, or a fault-injection allocator where no input alone can select the
branch.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `allocate_matrix` | `malloc(sizeof(matrix_t)) == NULL` | prints with `perror`; returns `NULL` | [x] |
| 2 | `allocate_matrix` | allocation of `height * sizeof(int*)` returns `NULL` | prints with `perror`, frees struct; returns `NULL` | [x] |
| 3 | `allocate_matrix` | allocation of any `width * sizeof(int)` row returns `NULL` | prints with `perror`, frees allocated rows and struct; returns `NULL` | [x] |
| 4 | `initialize_matrix_from_string` | `strdup(input) == NULL` | prints with `perror`, frees matrix; returns `NULL` | [x] |
| 5 | `initialize_matrix_from_string` | fewer newline-delimited nonempty rows than `height` | prints `Insufficient rows...`, frees state; returns `NULL` | [x] |
| 6 | `initialize_matrix_from_string` | a required row has fewer space-delimited nonempty columns than `width` | prints `Insufficient columns...`, frees state; returns `NULL` | [x] |
| 7 | `multiply_matrices` | `mat_a->width != mat_b->height` | prints dimension error; returns `NULL` | [x] |
| 8 | `matrix_to_string` | `mat == NULL` | prints `Error: Matrix is NULL.`; returns `NULL` | [x] |
| 9 | `matrix_to_string` | allocation of computed `buffer_size` returns `NULL` | prints with `perror`; returns `NULL` | [x] |
| 10 | `write_to_file` | `content == NULL` | prints `Error: Content is NULL.`; returns `EINVAL` | [x] |
| 11 | `write_to_file` | `fopen(filename, "w") == NULL` | prints opening error; returns the exact `errno` | [x] |
| 12 | `write_to_file` | `fprintf(file, "%s", content) < 0` | prints writing error, closes file; returns the exact `errno` | [x] |
| 13 | `write_to_file` | `fclose(file) != 0` | prints closing error; returns the exact `errno` | [x] |
| 14 | `driver` | first `initialize_matrix_from_string` returns `NULL` | returns `EXIT_FAILURE` (`1`) | [x] |
| 15 | `driver` | second `initialize_matrix_from_string` returns `NULL` | frees first matrix; returns `EXIT_FAILURE` (`1`) | [x] |
| 16 | `driver` | `multiply_matrices` returns `NULL` | frees both inputs; returns `EXIT_FAILURE` (`1`) | [x] |
| 17 | `driver` | `matrix_to_string` returns `NULL` | frees inputs and result struct; returns `EXIT_FAILURE` (`1`) | [x] |
| 18 | `driver` | `write_to_file` returns nonzero | frees all state; returns `EXIT_FAILURE` (`1`) | [x] |

There are no C `assert` statements, enums, `switch` statements, explicit
numeric range checks, or documented min/max constants.
