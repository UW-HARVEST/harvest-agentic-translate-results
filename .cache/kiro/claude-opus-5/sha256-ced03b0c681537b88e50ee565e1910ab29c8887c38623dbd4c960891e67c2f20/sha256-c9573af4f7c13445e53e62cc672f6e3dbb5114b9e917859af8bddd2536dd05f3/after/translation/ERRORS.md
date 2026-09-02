# ERRORS.md — error-surface table (Phase A → gate for Phase C)

Derived mechanically by grepping every `return NULL`, `return -1`-style
sentinel, `return errno`, `return EINVAL`, `return EXIT_FAILURE`, every
`if (… == NULL)`, every dimension comparison, and every constant in
`c_src/src/*.c`. There are **no** `assert`s, **no** enums, and **no** explicit
numeric range checks in the C sources — the entire rejection surface is
NULL-checks, one dimension-compatibility check, tokenizer exhaustion, and
`errno` propagation from stdio.

Constants that participate: `EINVAL` (22), `EXIT_FAILURE` (1),
`EXIT_SUCCESS` (0), `char buffer[12]` in `matrix_to_string`,
`OUT_FILE = "matrix.txt"` in `driver`.

| #   | function | trigger (the exact invalid input/condition) | expected C result | test | ✅ |
|-----|----------|---------------------------------------------|-------------------|------|----|
| 1  | `allocate_matrix` (`matrix.c:35`) | `malloc(sizeof(matrix_t))` returns NULL | `perror` + return `NULL` | not reachable (24-byte alloc); documented, no test | n/a |
| 2  | `allocate_matrix` (`matrix.c:44`) | `malloc(height * sizeof(int*))` fails — reached with `height < 0`, which sign-extends to a huge `size_t` | `perror` + `free(mat)` + return `NULL` | `err_allocate_matrix_negative_height` | [x] |
| 3  | `allocate_matrix` (`matrix.c:52`) | `malloc(width * sizeof(int))` fails for some row — reached with `height > 0 && width < 0` | `perror` + free rows `0..=i` + `free(mat->matrix)` + `free(mat)` + return `NULL` | `err_allocate_matrix_negative_width` | [x] |
| 4  | `free_matrix` (`matrix.c:67`) | `mat == NULL` | early `return`, no crash, no output (void) | `err_free_matrix_null` | [x] |
| 5  | `initialize_matrix_from_string` (`matrix.c:82`) | `strdup(input)` returns NULL (OOM) | `perror` + `free_matrix(mat)` + return `NULL` | not reachable without an allocator fault injector; documented | n/a |
| 6  | `initialize_matrix_from_string` (`matrix.c:91`) | `row_token == NULL`: fewer `\n`-separated rows in `input` than `height` (incl. `input == ""` with `height >= 1`) | `fprintf(stderr,"Insufficient rows in input string.\n")` + frees + return `NULL` | `err_init_insufficient_rows` | [x] |
| 7  | `initialize_matrix_from_string` (`matrix.c:101`) | `col_token == NULL`: some row `i` has fewer space-separated tokens than `width` | `fprintf(stderr,"Insufficient columns in row %d.\n", i+1)` + frees + return `NULL` | `err_init_insufficient_cols` | [x] |
| 8  | `multiply_matrices` (`matrix.c:119`) | `mat_a->width != mat_b->height` | `fprintf(stderr,"Matrix dimensions do not allow multiplication.\n")` + return `NULL` | `err_multiply_dim_mismatch` | [x] |
| 9  | `matrix_to_string` (`matrix.c:138`) | `mat == NULL` | `fprintf(stderr,"Error: Matrix is NULL.\n")` + return `NULL` | `err_matrix_to_string_null` | [x] |
| 10 | `matrix_to_string` (`matrix.c:145`) | `malloc(buffer_size)` fails — reached when the `int` expression `height*(width*10+width)+height+1` is negative (e.g. `height < 0`) and sign-extends to a huge `size_t` | `perror` + return `NULL` | `err_matrix_to_string_alloc_fail` | [x] |
| 11 | `write_to_file` (`write.c:33`) | `content == NULL` | `fprintf(stderr,"Error: Content is NULL.\n")` + return `EINVAL` = **22** | `err_write_null_content` | [x] |
| 12a | `write_to_file` (`write.c:39`) | `fopen` fails: path in a non-existent directory | `fprintf(stderr, "Error opening file …")` + return `errno` = **ENOENT 2** | `err_write_fopen_enoent` | [x] |
| 12b | `write_to_file` (`write.c:39`) | `fopen` fails: `filename == ""` | return `errno` = **ENOENT 2** | `err_write_fopen_empty_name` | [x] |
| 12c | `write_to_file` (`write.c:39`) | `fopen` fails: `filename` names an existing **directory** | return `errno` = **EISDIR 21** | `err_write_fopen_eisdir` | [x] |
| 12d | `write_to_file` (`write.c:39`) | `fopen` fails: `filename == NULL` (glibc `fopen(NULL,"w")` → NULL/EFAULT, does not crash) | return `errno` = **EFAULT 14** | `err_write_fopen_null_name` | [x] |
| 12e | `write_to_file` (`write.c:39`) | `fopen` fails: target file exists with mode `0400` (no write permission) | return `errno` = **EACCES 13** | `err_write_fopen_eacces` | [x] |
| 13 | `write_to_file` (`write.c:44`) | `fprintf(file,"%s",content) < 0`: write error surfaces during the call — content larger than `BUFSIZ` written to `/dev/full` | `fprintf(stderr,"Error writing to file …")` + `fclose` + return `errno` = **ENOSPC 28** | `err_write_fprintf_fails` | [x] |
| 14 | `write_to_file` (`write.c:50`) | `fclose(file) != 0`: short content to `/dev/full`, error deferred to the flush in `fclose` | `fprintf(stderr,"Error closing file …")` + return `errno` = **ENOSPC 28** | `err_write_fclose_fails` | [x] |
| 15 | `driver` (`driver.c:37`) | `initialize_matrix_from_string(matrix_a,…)` returns NULL (rows/cols short for A) | return `EXIT_FAILURE` = **1** | `err_driver_bad_a` | [x] |
| 16 | `driver` (`driver.c:41`) | A parses, `initialize_matrix_from_string(matrix_b,…)` returns NULL | `free_matrix(mat_a)` + return `EXIT_FAILURE` = **1** | `err_driver_bad_b` | [x] |
| 17 | `driver` (`driver.c:47`) | `multiply_matrices` returns NULL: `width_a != height_b` | frees + return `EXIT_FAILURE` = **1** | `err_driver_dim_mismatch` | [x] |
| 18 | `driver` (`driver.c:53`) | `matrix_to_string(res)` returns NULL | `free(res)` (struct only — leaks rows; reproduced verbatim) + return `EXIT_FAILURE` | not reachable: `res` is non-NULL here and its `buffer_size` cannot be made to fail without dimensions that `allocate_matrix`/parsing could not have produced. Documented | n/a |
| 19 | `driver` (`driver.c:67`) | `write_to_file("matrix.txt", …)` returns non-zero: cwd contains a **directory** named `matrix.txt` | return `EXIT_FAILURE` = **1** | `err_driver_write_fails` | [x] |

## Generic FFI boundaries also covered (not distinct C branches)

| # | condition | note | test | ✅ |
|---|-----------|------|------|----|
| G1 | `width == 0` / `height == 0` (zero lengths) | `malloc(0)` is non-NULL on glibc; loops do not execute; valid, not an error | covered in `CONFIGS.md` rows 1–3 and `err_zero_dims_not_an_error` | [x] |
| G2 | out-of-range enum value across FFI | **N/A** — this API declares no enums; every parameter is `int`, `const char*` or `matrix_t*`. The `int` axis is instead swept with `INT_MIN`, `-1`, `0`, `1`, `INT_MAX` | `err_int_extremes_dims` | [x] |
| G3 | one step past valid range | `width`/`height` have no documented range; `-1` (one past 0) and `INT_MIN`/`INT_MAX` are the boundaries | `err_int_extremes_dims` | [x] |
| G4 | oversized lengths | `width`/`height` = `INT_MAX` → allocation failure path | `err_int_extremes_dims` | [x] |
| G5 | NULL pointers into every entry point that checks them | `free_matrix(NULL)`, `matrix_to_string(NULL)`, `write_to_file(_, NULL)`, `write_to_file(NULL, _)` | rows 4, 9, 11, 12d | [x] |

### Deliberately NOT tested (undefined behaviour in the C — no defined answer to compare)

These are inputs where the C itself has UB, so "byte-identical" is not
meaningful; the Rust reproduces the same instruction sequence, but exercising
them would corrupt the heap of whichever process ran them:

* `initialize_matrix_from_string(NULL, …)` → `strdup(NULL)` dereferences NULL.
* `initialize_matrix_from_string` when `allocate_matrix` returned NULL
  (e.g. negative `width`/`height` **and** enough tokens) → NULL deref at
  `mat->matrix[i][j]`. Note `err_allocate_matrix_negative_*` calls
  `allocate_matrix` directly, avoiding this.
* `multiply_matrices(NULL, …)` / `(…, NULL)` → unchecked `mat_a->width` deref.
* `matrix_to_string` on a matrix whose decimal digits exceed the buffer
  formula (any `width >= 2` containing a value needing 11 chars, i.e.
  `<= -1000000000`) → `strcat` heap overflow in the C. See `CONFIGS.md`
  "value-range safety" note.
