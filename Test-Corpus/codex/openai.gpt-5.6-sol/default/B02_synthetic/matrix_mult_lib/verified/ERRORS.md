# Error-Surface Table

This table is mechanically derived from every null check, failed allocation,
failed I/O operation, and failure-propagation branch in `c_src/src/*.c`.
Allocation failures require fault injection; they are still part of the C
surface and are not omitted merely because they are uncommon.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `allocate_matrix` | `malloc(sizeof(matrix_t)) == NULL` | prints with `perror`; returns `NULL` | [x] |
| 2 | `allocate_matrix` | row-pointer allocation `malloc(height * sizeof(int *)) == NULL` | prints with `perror`, frees the struct; returns `NULL` | [x] |
| 3 | `allocate_matrix` | any row allocation `malloc(width * sizeof(int)) == NULL` | prints with `perror`, frees rows through the failed row and the struct; returns `NULL` | [x] |
| 4 | `free_matrix` | `mat == NULL` | returns immediately; no error and no crash | [x] |
| 5 | `initialize_matrix_from_string` | `strdup(input) == NULL` | prints with `perror`, frees `mat`; returns `NULL` | [x] |
| 6 | `initialize_matrix_from_string` | fewer newline-delimited nonempty rows than `height` (`row_token == NULL`) | prints `Insufficient rows...`, frees temporary input and matrix; returns `NULL` | [x] |
| 7 | `initialize_matrix_from_string` | a consumed row has fewer space-delimited nonempty columns than `width` (`col_token == NULL`) | prints `Insufficient columns...`, frees temporary input and matrix; returns `NULL` | [x] |
| 8 | `multiply_matrices` | `mat_a->width != mat_b->height` | prints dimension error; returns `NULL` | [x] |
| 9 | `matrix_to_string` | `mat == NULL` | prints `Error: Matrix is NULL.`; returns `NULL` | [x] |
| 10 | `matrix_to_string` | result-buffer `malloc(buffer_size) == NULL` | prints with `perror`; returns `NULL` | [x] |
| 11 | `write_to_file` | `content == NULL` | prints null-content error; returns `EINVAL` | [x] |
| 12 | `write_to_file` | `fopen(filename, "w") == NULL` | prints open error; returns the current `errno` | [x] |
| 13 | `write_to_file` | `fprintf(file, "%s", content) < 0` | prints write error, closes file; returns the current `errno` | [x] |
| 14 | `write_to_file` | `fclose(file) != 0` | prints close error; returns the current `errno` | [x] |
| 15 | `driver` | initializing matrix A returns `NULL` | returns `EXIT_FAILURE` (1) | [x] |
| 16 | `driver` | matrix A succeeds but initializing matrix B returns `NULL` | frees A; returns `EXIT_FAILURE` (1) | [x] |
| 17 | `driver` | both inputs parse but multiplication returns `NULL` | frees A and B; returns `EXIT_FAILURE` (1) | [x] |
| 18 | `driver` | multiplication succeeds but `matrix_to_string` returns `NULL` | takes the C failure cleanup branch; returns `EXIT_FAILURE` (1) | [x] |
| 19 | `driver` | `write_to_file("matrix.txt", result) != 0` | frees all successful intermediates; returns `EXIT_FAILURE` (1) | [x] |

There are no C enums and therefore no invalid-enum-value cases. The generic
unchecked-null-pointer and dimension boundaries are exercised separately in
the differential tests, including subprocess comparisons for C-undefined
crashing calls.
