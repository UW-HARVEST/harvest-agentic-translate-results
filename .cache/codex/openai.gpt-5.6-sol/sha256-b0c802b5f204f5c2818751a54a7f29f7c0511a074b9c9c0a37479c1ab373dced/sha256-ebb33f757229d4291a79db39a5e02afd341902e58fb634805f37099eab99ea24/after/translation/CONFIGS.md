# Configuration-Surface Table

The C API has no compile-time Cargo feature or runtime mode flag. Its
configuration axes are matrix dimensions, token shapes, integer values, shape
compatibility, and file state. Rows below are the branch-distinct valid
cross-product pruned to behavior the C source actually distinguishes.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `allocate_matrix`, `free_matrix` | width 0, height 0 (no row loop) | [x] |
| 2 | `allocate_matrix`, `free_matrix` | width 0, positive height (row loop; zero-byte rows) | [x] |
| 3 | `allocate_matrix`, `free_matrix` | positive width, height 0 (row-pointer allocation; no rows) | [x] |
| 4 | `allocate_matrix`, `free_matrix` | 1 x 1 | [x] |
| 5 | `allocate_matrix`, `free_matrix` | rectangular many x many | [x] |
| 6 | `initialize_matrix_from_string`, `free_matrix` | height 0 and width 0; input is not consumed | [x] |
| 7 | `initialize_matrix_from_string`, `free_matrix` | height 0 and positive width; input is not consumed | [x] |
| 8 | `initialize_matrix_from_string`, `free_matrix` | positive height and width 0; rows are required but columns are not consumed | [x] |
| 9 | `initialize_matrix_from_string`, `free_matrix` | 1 x 1 decimal token | [x] |
| 10 | `initialize_matrix_from_string`, `free_matrix` | rectangular matrix with exactly the required rows and columns | [x] |
| 11 | `initialize_matrix_from_string`, `free_matrix` | extra columns and extra rows; extras are ignored | [x] |
| 12 | `initialize_matrix_from_string`, `free_matrix` | repeated spaces/newlines; `strtok_r` skips empty fields | [x] |
| 13 | `initialize_matrix_from_string`, `free_matrix` | signed, prefixed, suffixed, and nonnumeric tokens exercising `atoi` | [x] |
| 14 | `multiply_matrices`, `free_matrix` | compatible 1 x 1 matrices | [x] |
| 15 | `multiply_matrices`, `free_matrix` | compatible rectangular matrices with inner dimension 1 | [x] |
| 16 | `multiply_matrices`, `free_matrix` | compatible rectangular matrices with inner dimension many | [x] |
| 17 | `multiply_matrices`, `free_matrix` | compatible matrices with inner dimension 0; each result cell is zero | [x] |
| 18 | `multiply_matrices`, `free_matrix` | zero output height | [x] |
| 19 | `multiply_matrices`, `free_matrix` | zero output width | [x] |
| 20 | `matrix_to_string` | 0 x 0 matrix produces an empty C string | [x] |
| 21 | `matrix_to_string` | positive width and zero height produces an empty C string | [x] |
| 22 | `matrix_to_string` | zero width and positive height produces one newline per row | [x] |
| 23 | `matrix_to_string` | 1 x 1 positive, zero, and negative values | [x] |
| 24 | `matrix_to_string` | rectangular many-cell matrix; spaces between columns and newline after every row | [x] |
| 25 | `matrix_to_string` | `INT_MIN` and `INT_MAX` formatting boundaries | [x] |
| 26 | `write_to_file` | nonexistent output path with empty content; creates an empty file | [x] |
| 27 | `write_to_file` | nonexistent output path with nonempty/multiline content | [x] |
| 28 | `write_to_file` | existing file; truncates and replaces previous bytes | [x] |
| 29 | `driver` | 1 x 1 end-to-end parse, multiply, stringify, and file write | [x] |
| 30 | `driver` | compatible rectangular matrices, inner dimension 1 | [x] |
| 31 | `driver` | compatible rectangular matrices, inner dimension many | [x] |
| 32 | `driver` | extra input rows/columns and `atoi` token forms flow through the composed pipeline | [x] |
| 33 | `driver` | existing `matrix.txt` is truncated and replaced | [x] |

No executable target is declared by either CMake or Cargo, so binary stdout
comparison is not applicable. Cargo declares no features; the only feature
configuration is the default/no-feature-equivalent build.
