# ERRORS.md — Phase C error-surface table

Every distinct rejection / error return / diagnostic in the C sources, found by
grepping `c_src/src/*.c` for `return NULL`, `return -1`, `return errno`,
`return EINVAL`, `return EXIT_FAILURE`, `perror`, `fprintf(stderr`, `== NULL`,
`!=`, and `assert`.

The C library has **no error enum and no `assert`s**. Its rejection channels are
exactly three: a `NULL` pointer return, an `errno`-valued `int` return, and
`EXIT_FAILURE` (`1`). Diagnostics go to `stderr` via `perror`/`fprintf`; the
tests compare the captured `stderr` bytes too, not just the return value.

| # | function | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|------------------------------------------|-------------------|------|---|
| 1 | `allocate_matrix` | `malloc(sizeof(matrix_t))` returns NULL (`matrix.c:35`) | `perror("Failed to allocate memory for matrix struct")`, return `NULL` | not reachable without an allocator fault injector — documented, not triggerable in-process | n/a |
| 2 | `allocate_matrix` | `malloc(height * sizeof(int*))` returns NULL (`matrix.c:44`). Reachable: `height < 0` → `(size_t)height * 8` is astronomically large; also `height` huge positive | `perror("Failed to allocate memory for matrix rows")`, `free(mat)`, return `NULL` | `err_02_allocate_negative_height` | [x] |
| 3 | `allocate_matrix` | `malloc(width * sizeof(int))` returns NULL for some row (`matrix.c:52`). Reachable: `width < 0 && height >= 1` | `perror("Failed to allocate memory for matrix columns")`, `free` rows `0..=i`, `free(mat->matrix)`, `free(mat)`, return `NULL` | `err_03_allocate_negative_width` | [x] |
| 4 | `free_matrix` | `mat == NULL` (`matrix.c:67`) | silent early `return`, no output, no crash | `err_04_free_matrix_null` | [x] |
| 5 | `initialize_matrix_from_string` | `strdup(input)` returns NULL (`matrix.c:82`) | `perror("Failed to duplicate input string")`, `free_matrix(mat)`, return `NULL` | not reachable without an allocator fault injector (note: `input == NULL` is *not* checked — `strdup(NULL)` faults in both builds identically; covered out-of-process by `err_05_null_input_crashes_both`) | [x] |
| 6 | `initialize_matrix_from_string` | fewer than `height` `"\n"`-separated tokens in `input` (`matrix.c:91`) — incl. empty string, all-newline string, `height` > line count | `fprintf(stderr, "Insufficient rows in input string.\n")`, `free(input_copy)`, `free_matrix(mat)`, return `NULL` | `err_06_insufficient_rows` | [x] |
| 7 | `initialize_matrix_from_string` | some row `i` has fewer than `width` `" "`-separated tokens (`matrix.c:101`) | `fprintf(stderr, "Insufficient columns in row %d.\n", i+1)` (1-based), `free(input_copy)`, `free_matrix(mat)`, return `NULL` | `err_07_insufficient_columns` | [x] |
| 8 | `initialize_matrix_from_string` | `allocate_matrix` failed (row 2/3) but the loop bounds make the body unreachable (`height < 0` → outer loop skipped; `width < 0` → inner loop skipped) → the *unchecked* `mat` (NULL) is returned as a "success" value | return `NULL` (no diagnostic beyond `perror` from `allocate_matrix`) | `err_08_init_returns_null_from_failed_alloc` | [x] |
| 8b | `initialize_matrix_from_string` | `allocate_matrix` failed for a *large positive* `width`/`height` (e.g. `(INT_MAX, 2)`, `(2, INT_MAX)`, `(1, INT_MAX/4)`) so the loops DO run and `mat->matrix[i][j] = …` dereferences NULL | SIGSEGV | `g3_oversized_lengths` (out-of-process payloads `init_huge_width`, `init_huge_height`, `init_alloc_fail_deref`) | [x] |
| 9 | `multiply_matrices` | `mat_a->width != mat_b->height` (`matrix.c:119`) | `fprintf(stderr, "Matrix dimensions do not allow multiplication.\n")`, return `NULL` | `err_09_dimension_mismatch` | [x] |
| 10 | `multiply_matrices` | `mat_a == NULL` or `mat_b == NULL` — **not checked** by C, immediate NULL deref | SIGSEGV | `err_10_multiply_null_crashes_both` (out-of-process, compares signal) | [x] |
| 11 | `matrix_to_string` | `mat == NULL` (`matrix.c:138`) | `fprintf(stderr, "Error: Matrix is NULL.\n")`, return `NULL` | `err_11_to_string_null` | [x] |
| 12 | `matrix_to_string` | `malloc(buffer_size)` returns NULL (`matrix.c:145`). Reachable: `buffer_size` (an `int`) computes negative → sign-extended to a huge `size_t` | `perror("Failed to allocate memory for matrix string")`, return `NULL` | `err_12_to_string_buffer_size_overflow` | [x] |
| 13 | `write_to_file` | `content == NULL` (`write.c:33`) | `fprintf(stderr, "Error: Content is NULL.\n")`, return `EINVAL` (`22`) | `err_13_write_null_content` | [x] |
| 14 | `write_to_file` | `fopen(filename,"w")` returns NULL (`write.c:39`) — nonexistent directory (`ENOENT` 2), filename is a directory (`EISDIR` 21), unwritable path (`EACCES` 13), empty filename (`ENOENT` 2), over-long name (`ENAMETOOLONG` 36) | `fprintf(stderr, "Error opening file '%s': %s\n", …)`, return `errno` | `err_14_write_fopen_failures` | [x] |
| 15 | `write_to_file` | `fprintf(file, "%s", content) < 0` (`write.c:44`). Reachable: the target is `/dev/full` and `content` is larger than `BUFSIZ`, so `fprintf` itself has to flush and the write returns `ENOSPC` (28) | `fprintf(stderr, "Error writing to file '%s': %s\n", …)`, `fclose(file)`, return `errno` | `err_15_write_fprintf_failure` | [x] |
| 16 | `write_to_file` | `fclose(file) != 0` (`write.c:50`). Reachable: the target is `/dev/full` and `content` fits in the stdio buffer, so only the final flush inside `fclose` fails (`ENOSPC` 28) | `fprintf(stderr, "Error closing file '%s': %s\n", …)`, return `errno` | `err_16_write_fclose_failure` | [x] |
| 17 | `write_to_file` | `filename == NULL` — **not checked**; passed straight to `fopen(NULL,"w")` | glibc `fopen(NULL,…)` → `NULL` + `errno = EFAULT`(14); the `%s` of a NULL `filename` prints `(null)` | `err_17_write_null_filename` | [x] |
| 18 | `driver` | `initialize_matrix_from_string(matrix_a, …)` returns NULL (`driver.c:37`) | return `EXIT_FAILURE` (`1`) | `err_18_driver_mat_a_fails` | [x] |
| 19 | `driver` | `initialize_matrix_from_string(matrix_b, …)` returns NULL (`driver.c:41`) | `free_matrix(mat_a)`, return `EXIT_FAILURE` | `err_19_driver_mat_b_fails` | [x] |
| 20 | `driver` | `multiply_matrices` returns NULL, i.e. `width_a != height_b` (`driver.c:47`) | `free_matrix` ×2, return `EXIT_FAILURE` | `err_20_driver_dim_mismatch` | [x] |
| 21 | `driver` | `matrix_to_string(res)` returns NULL (`driver.c:53`) — needs `res` NULL or its `malloc` to fail; `res` is never NULL here, so only via a negative `buffer_size` | `free_matrix` ×2, **plain `free(res)`** (leaks rows — bug preserved), return `EXIT_FAILURE` | `err_21_driver_to_string_fails` | [x] |
| 22 | `driver` | `write_to_file` returns non-zero (`driver.c:67`). Reachable: `matrix.txt` in the CWD is a **directory**, so `fopen(…,"w")` fails with `EISDIR` (21) | `free_matrix` ×3 + `free(res_str)` first, then return `EXIT_FAILURE` | `err_22_driver_write_fails` | [x] |
| 23 | `driver` | all steps succeed (`driver.c:71`) | return `EXIT_SUCCESS` (`0`) and `matrix.txt` written | `err_23_driver_success_code` | [x] |
| 24 | `free_matrix` | `mat != NULL` but `mat->matrix == NULL` — the field is **never** checked (`matrix.c:71-72`). With `height <= 0` the row loop is skipped, so `free(NULL); free(mat)` is a silent no-op | silent success, no output | `err_24_free_matrix_null_rows_nonpositive_height` | [x] |
| 24b | `free_matrix` | same, but `height > 0` → `free(mat->matrix[0])` dereferences NULL | SIGSEGV | `err_24b_free_matrix_null_rows_positive_height` (out-of-process) | [x] |

## Generic FFI boundary cases (mandated in addition to the table)

| # | case | ✔ |
|---|------|---|
| G1 | NULL pointers into every pointer parameter of every entry point (`free_matrix`, `matrix_to_string`, `initialize_matrix_from_string`, `multiply_matrices`, `write_to_file`'s `filename` and `content`) **and** into the `matrix_t::matrix` field reached through the struct — the ones that legitimately fault are compared out-of-process (`g1_null_pointers`, `err_04`, `err_05`, `err_10`, `err_11`, `err_13`, `err_17`, `err_24`, `err_24b`) | [x] |
| G2 | zero lengths: `allocate_matrix(0,0)`, `(0,n)`, `(n,0)`; `initialize_matrix_from_string(s,0,0)`; empty `content`/`input` strings | [x] |
| G3 | oversized lengths: `INT_MAX`, `INT_MAX-1`, `1<<20`, `1<<30` for `width`/`height` | [x] |
| G4 | one step past valid range: `width = -1`, `height = -1`, `INT_MIN`, and `atoi` inputs one past `INT_MAX`/`INT_MIN` (`"2147483648"`, `"-2147483649"`) | [x] |
| G5 | out-of-range "enum" values across FFI: the C API declares **no `enum` types**; the equivalent unconstrained-integer surface is `width`/`height` (any `int` is accepted by the ABI) and the `int` cell values. Every `int` parameter is fuzzed over the full `i32` range including `INT_MIN`/`INT_MAX`, which is the exact analogue of "a value with no valid variant". | [x] |

## How the rows are exercised

* Every row is a differential test: the C `.so` and the Rust `.so` are both
  `dlopen`ed and called through their exported C symbols, and the test asserts
  the **same return value / sentinel** *and* the **same `stderr` bytes** (the
  `perror`/`fprintf` diagnostics are part of the observable behaviour). Where
  the errno is deterministic the test also pins the exact value
  (`EINVAL` 22, `ENOENT` 2, `EACCES` 13, `EFAULT` 14, `EISDIR` 21,
  `ENOSPC` 28, `ENAMETOOLONG` 36).
* Rows whose C behaviour is a fault (5, 8b, 10, 24b) are run in a **child
  process** (`oop_payload`) and the parent compares the child's exit code, the
  terminating signal, and its `stderr`. That is the only way to assert "both
  crash the same way" without killing the test runner.
* `allocate_matrix`'s `malloc` failures are made deterministic by capping
  `RLIMIT_AS` to 3 GiB in the test process (see `tests/common/mod.rs`). Both
  libraries live in that one address space, so the cap applies to them
  identically; without it Linux overcommit lets some of the pathological
  requests succeed and the process is OOM-killed instead of returning `NULL`.
* Rows 1 and 5 (a failure of `malloc(sizeof(matrix_t))` / `strdup`) are the only
  ones with no reachable trigger — they need an allocator fault injector. They
  are recorded here for completeness and the Rust code is a line-for-line
  transcription of those two branches (same `perror` string, same cleanup, same
  `NULL` return).

## Result

All rows except the two unreachable allocator-fault branches (1 and 5's
`strdup` case) have a passing differential test. `cargo test --release` →
**30 passed, 0 failed** in `tests/phase_c_errors.rs`.
