# Configuration Surface

The API exposes no option structs, modes, flags, enums, conditional compilation,
or Cargo features. Its runtime branches are driven by matrix dimensions,
tokenized input shape, integer text/value shape, multiplication compatibility,
file content/path state, and the composed `driver` stages.

| # | entry point(s) | configuration (options set + input shape) | passed |
|---|----------------|--------------------------------------------|--------|
| 1 | `allocate_matrix`, `free_matrix` | zero height; width zero/one/many (row loop skipped) | [x] |
| 2 | `allocate_matrix`, `free_matrix` | positive height; zero width (`malloc(0)` row for each height) | [x] |
| 3 | `allocate_matrix`, `free_matrix` | one-by-one matrix | [x] |
| 4 | `allocate_matrix`, `free_matrix` | rectangular positive dimensions with many rows and columns | [x] |
| 5 | `free_matrix` | `NULL` matrix (explicit no-op branch) | [x] |
| 6 | `initialize_matrix_from_string`, `free_matrix` | zero height; empty/nonempty input; width zero/positive | [x] |
| 7 | `initialize_matrix_from_string`, `free_matrix` | positive height and zero width; enough nonempty rows | [x] |
| 8 | `initialize_matrix_from_string`, `free_matrix` | one-by-one decimal integer | [x] |
| 9 | `initialize_matrix_from_string`, `free_matrix` | many rows/columns with exact token counts | [x] |
| 10 | `initialize_matrix_from_string`, `free_matrix` | extra row and column tokens (extras ignored) | [x] |
| 11 | `initialize_matrix_from_string`, `free_matrix` | repeated spaces/newlines (delimiter runs collapsed by `strtok_r`) | [x] |
| 12 | `initialize_matrix_from_string`, `free_matrix` | tokens accepted by `atoi`: signs, leading whitespace, numeric prefixes with suffixes, and nonnumeric text | [x] |
| 13 | `multiply_matrices`, `matrix_to_string`, `free_matrix` | compatible 1x1 operands | [x] |
| 14 | `multiply_matrices`, `matrix_to_string`, `free_matrix` | compatible rectangular operands with one inner element | [x] |
| 15 | `multiply_matrices`, `matrix_to_string`, `free_matrix` | compatible rectangular operands with many inner elements | [x] |
| 16 | `multiply_matrices`, `matrix_to_string`, `free_matrix` | compatible operands with zero inner dimension (result cells remain zero) | [x] |
| 17 | `multiply_matrices`, `matrix_to_string`, `free_matrix` | compatible operands producing zero rows or zero columns | [x] |
| 18 | `matrix_to_string` | non-NULL matrix with zero height (empty string) | [x] |
| 19 | `matrix_to_string` | positive height and zero width (one newline per row) | [x] |
| 20 | `matrix_to_string` | one/many cells containing negative, zero, and positive values | [x] |
| 21 | `matrix_to_string` | cells containing `INT_MIN` and `INT_MAX` (11/10 printed characters) | [x] |
| 22 | `write_to_file` | empty content to a creatable path | [x] |
| 23 | `write_to_file` | nonempty single-line content to a new/existing path | [x] |
| 24 | `write_to_file` | multiline content (including matrix format) truncating an existing file | [x] |
| 25 | `driver` | compatible 1x1 inputs; writes `matrix.txt` | [x] |
| 26 | `driver` | compatible rectangular inputs with one inner element | [x] |
| 27 | `driver` | compatible rectangular inputs with many inner elements and mixed-sign values | [x] |
| 28 | `driver` | extra tokens and `atoi`-specific token forms propagated end to end | [x] |

Rows describing rejected dimensions, missing tokens, null pointers, allocation
failures, incompatible multiplication, and I/O failures are in `ERRORS.md`.
