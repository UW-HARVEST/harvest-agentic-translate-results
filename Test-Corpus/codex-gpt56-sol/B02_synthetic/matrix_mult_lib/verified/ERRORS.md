# Error Surface

Each row corresponds to a distinct explicit rejection branch in the C source.
Allocation-failure rows require resource exhaustion or allocator fault
injection; they are not inferred from normal successful allocations.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `allocate_matrix` | `malloc(sizeof(matrix_t)) == NULL` | `NULL` | [x] |
| 2 | `allocate_matrix` | row-pointer allocation `malloc(height * sizeof(int *)) == NULL` | frees the struct and returns `NULL` | [x] |
| 3 | `allocate_matrix` | any row allocation `malloc(width * sizeof(int)) == NULL` | frees rows allocated through the failing row, row-pointer storage, and struct; returns `NULL` | [x] |
| 4 | `free_matrix` | `mat == NULL` | returns without action | [x] |
| 5 | `initialize_matrix_from_string` | `strdup(input) == NULL` | frees the allocated matrix and returns `NULL` | [x] |
| 6 | `initialize_matrix_from_string` | fewer newline-delimited nonempty row tokens than `height` | frees temporary input and matrix; returns `NULL` | [x] |
| 7 | `initialize_matrix_from_string` | a row has fewer space-delimited nonempty column tokens than `width` | frees temporary input and matrix; returns `NULL` | [x] |
| 8 | `multiply_matrices` | `mat_a->width != mat_b->height` | `NULL` | [x] |
| 9 | `matrix_to_string` | `mat == NULL` | `NULL` | [x] |
| 10 | `matrix_to_string` | result-buffer `malloc(buffer_size) == NULL` | `NULL` | [x] |
| 11 | `write_to_file` | `content == NULL` | `EINVAL` (22 on this build platform) | [x] |
| 12 | `write_to_file` | `fopen(filename, "w") == NULL` | current `errno` | [x] |
| 13 | `write_to_file` | `fprintf(file, "%s", content) < 0` | closes stream and returns current `errno` | [x] |
| 14 | `write_to_file` | `fclose(file) != 0` | current `errno` | [x] |
| 15 | `driver` | initialization of matrix A returns `NULL` | `EXIT_FAILURE` (1) | [x] |
| 16 | `driver` | initialization of matrix B returns `NULL` | frees A and returns `EXIT_FAILURE` (1) | [x] |
| 17 | `driver` | multiplication returns `NULL` | frees A and B and returns `EXIT_FAILURE` (1) | [x] |
| 18 | `driver` | result conversion returns `NULL` | frees A, B, and the result struct; returns `EXIT_FAILURE` (1) | [x] |
| 19 | `driver` | `write_to_file("matrix.txt", result) != 0` | frees all owned values and returns `EXIT_FAILURE` (1) | [x] |
