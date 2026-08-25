# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `CMakeLists.txt` declares no options
or conditional sources. There is exactly one valid feature combination:
The public API also declares no enums and no documented bounded numeric ranges,
so there are no out-of-range enum or one-past-documented-range configurations.

| # | Cargo invocation feature set | CMake configuration | [ ] |
|---|------------------------------|---------------------|-----|
| 1 | `--no-default-features` (empty set) | default | [x] |

## Runtime Configurations

Rows are the cross-product branches that the C implementation distinguishes:
dimension-controlled zero/one/many loops, token shapes, sign/value formatting,
dimension compatibility, content length, path state, and composed driver
stages. Extra tokens are included because the parser deliberately ignores
them.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `allocate_matrix`, `free_matrix` | zero width, zero height | [x] |
| 2 | `allocate_matrix`, `free_matrix` | zero width, one or many rows | [x] |
| 3 | `allocate_matrix`, `free_matrix` | one column, one row | [x] |
| 4 | `allocate_matrix`, `free_matrix` | many columns and many rows | [x] |
| 5 | `free_matrix` | valid zero-sized matrix | [x] |
| 6 | `initialize_matrix_from_string`, `free_matrix` | zero width and zero height; input tokens ignored | [x] |
| 7 | `initialize_matrix_from_string`, `free_matrix` | zero width and positive height; one nonempty row token required per row | [x] |
| 8 | `initialize_matrix_from_string`, `free_matrix` | positive width and zero height; input tokens ignored | [x] |
| 9 | `initialize_matrix_from_string`, `free_matrix` | one-by-one decimal input | [x] |
| 10 | `initialize_matrix_from_string`, `free_matrix` | many rows/columns with repeated spaces/newlines | [x] |
| 11 | `initialize_matrix_from_string`, `free_matrix` | extra columns and rows, which are ignored | [x] |
| 12 | `initialize_matrix_from_string`, `free_matrix` | signed, nondigit, and mixed-prefix tokens through `atoi` | [x] |
| 13 | `multiply_matrices`, `free_matrix` | conforming 1x1 matrices | [x] |
| 14 | `multiply_matrices`, `free_matrix` | conforming rectangular matrices with one inner element | [x] |
| 15 | `multiply_matrices`, `free_matrix` | conforming rectangular matrices with many inner elements | [x] |
| 16 | `multiply_matrices`, `free_matrix` | conforming zero inner dimension | [x] |
| 17 | `multiply_matrices`, `free_matrix` | conforming result with zero height | [x] |
| 18 | `multiply_matrices`, `free_matrix` | conforming result with zero width | [x] |
| 19 | `matrix_to_string` | zero-by-zero matrix produces an empty string | [x] |
| 20 | `matrix_to_string` | zero width with positive height produces one newline per row | [x] |
| 21 | `matrix_to_string` | one element: zero, positive, or negative | [x] |
| 22 | `matrix_to_string` | many rows/columns containing `INT_MIN`, `INT_MAX`, and mixed signs | [x] |
| 23 | `write_to_file` | empty content to a new path | [x] |
| 24 | `write_to_file` | nonempty content to a new path | [x] |
| 25 | `write_to_file` | nonempty content truncates an existing path | [x] |
| 26 | `driver` | valid 1x1 end-to-end operation writes `matrix.txt` | [x] |
| 27 | `driver` | valid rectangular end-to-end operation with one inner element | [x] |
| 28 | `driver` | valid rectangular end-to-end operation with many inner elements and mixed signs | [x] |
| 29 | `driver` | valid inputs with extra row/column tokens ignored end to end | [x] |
