# ERRORS.md — Phase A error surface table

Every distinct rejection / error return in `c_src/`, found by grepping for
`return NULL`, `return -1`, `return errno`, `return EINVAL`, `return EXIT_*`,
`perror`, `fprintf(stderr`, and every `if (... == NULL)` / dimension check.
There are no `assert`s, no error enums and no named min/max constants in the C
sources; the only constants are `EINVAL`, `EXIT_FAILURE`, `EXIT_SUCCESS` and
the implicit `errno` values produced by `fopen`/`fprintf`/`fclose`.

Legend for "expected C result": the return value, plus the exact bytes written
to `stderr` (both are compared byte-for-byte by the differential tests).

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| E1 | `allocate_matrix` (`matrix.c:35`) | `malloc(sizeof(matrix_t))` fails (16-byte allocation; only under a hard address-space limit) | `perror("Failed to allocate memory for matrix struct")`; returns `NULL` | `e1_allocate_matrix_struct_malloc_fails` (child with `RLIMIT_AS`) | [x] |
| E2 | `allocate_matrix` (`matrix.c:44`) | `malloc(height * sizeof(int*))` fails — reached with `height < 0`, because `(size_t)height * 8` wraps to a huge request | `perror("Failed to allocate memory for matrix rows")`; `free(mat)`; returns `NULL` | `e2_allocate_matrix_negative_height` | [x] |
| E3 | `allocate_matrix` (`matrix.c:52`) | `malloc(width * sizeof(int))` fails for some row — reached with `width < 0 && height >= 1` | `perror("Failed to allocate memory for matrix columns")`; frees rows `0..=i`, rows array, struct; returns `NULL` | `e3_allocate_matrix_negative_width` | [x] |
| E4 | `free_matrix` (`matrix.c:67`) | `mat == NULL` | early `return`, no output, no crash | `e4_free_matrix_null` | [x] |
| E5 | `initialize_matrix_from_string` (`matrix.c:82`) | `strdup(input)` returns `NULL` (allocation failure) | `perror("Failed to duplicate input string")`; `free_matrix(mat)`; returns `NULL` | `e5_init_strdup_fails` (child with `RLIMIT_AS`) | [x] |
| E6 | `initialize_matrix_from_string` (`matrix.c:91`) | fewer than `height` `"\n"`-separated tokens in `input` (includes `input == ""`, and `height > 0` with a string whose rows run out) | `fprintf(stderr, "Insufficient rows in input string.\n")`; `free(input_copy)`; `free_matrix(mat)`; returns `NULL` | `e6_init_insufficient_rows` | [x] |
| E7 | `initialize_matrix_from_string` (`matrix.c:101`) | row `i` has fewer than `width` `" "`-separated tokens | `fprintf(stderr, "Insufficient columns in row %d.\n", i+1)`; `free(input_copy)`; `free_matrix(mat)`; returns `NULL` | `e7_init_insufficient_columns` | [x] |
| E8 | `multiply_matrices` (`matrix.c:119`) | `mat_a->width != mat_b->height` | `fprintf(stderr, "Matrix dimensions do not allow multiplication.\n")`; returns `NULL` | `e8_multiply_dimension_mismatch` | [x] |
| E9 | `matrix_to_string` (`matrix.c:138`) | `mat == NULL` | `fprintf(stderr, "Error: Matrix is NULL.\n")`; returns `NULL` | `e9_to_string_null` | [x] |
| E10 | `matrix_to_string` (`matrix.c:145`) | `malloc(buffer_size)` fails. `buffer_size = h*(w*10 + w) + h + 1` is an `int`; a negative value (e.g. `w < 0 && h > 0`, or `h < 0 && w > 0`) sign-extends to a huge `size_t` | `perror("Failed to allocate memory for matrix string")`; returns `NULL` | `e10_to_string_negative_buffer_size` | [x] |
| E11 | `write_to_file` (`write.c:33`) | `content == NULL` | `fprintf(stderr, "Error: Content is NULL.\n")`; returns `EINVAL` (`22`) | `e11_write_null_content` | [x] |
| E12 | `write_to_file` (`write.c:39`) | `fopen(filename, "w")` returns `NULL` | `fprintf(stderr, "Error opening file '%s': %s\n", filename, strerror(errno))`; returns `errno` | `e12_write_fopen_failures` | [x] |
| E12a | ↳ | `filename == ""` | returns `ENOENT` (`2`) | ↳ | [x] |
| E12b | ↳ | `filename` in a non-existent directory | returns `ENOENT` (`2`) | ↳ | [x] |
| E12c | ↳ | `filename` is an existing **directory** | returns `EISDIR` (`21`) | ↳ | [x] |
| E12d | ↳ | `filename == NULL` (null pointer across FFI; glibc `open(NULL)` → `EFAULT`, and `"%s"` prints `(null)`) | returns `EFAULT` (`14`) | ↳ | [x] |
| E12e | ↳ | parent path component is a regular file | returns `ENOTDIR` (`20`) | ↳ | [x] |
| E12f | ↳ | path component longer than `NAME_MAX`/`PATH_MAX` | returns `ENAMETOOLONG` (`36`) | ↳ | [x] |
| E12g | ↳ | target directory not writable (mode `0500`) | returns `EACCES` (`13`) | ↳ | [x] |
| E13 | `write_to_file` (`write.c:44`) | `fprintf(file, "%s", content) < 0` — content larger than `BUFSIZ` so the stream flushes during the call and the write fails (`/dev/full`) | `fprintf(stderr, "Error writing to file '%s': %s\n", ...)`; `fclose(file)`; returns `errno` = `ENOSPC` (`28`) | `e13_write_fprintf_fails_dev_full` | [x] |
| E14 | `write_to_file` (`write.c:50`) | `fclose(file) != 0` — content smaller than `BUFSIZ`, so the failure surfaces at flush-on-close (`/dev/full`) | `fprintf(stderr, "Error closing file '%s': %s\n", ...)`; returns `errno` = `ENOSPC` (`28`) | `e14_write_fclose_fails_dev_full` | [x] |
| E15 | `driver` (`driver.c:37`) | `initialize_matrix_from_string(matrix_a, width_a, height_a)` returns `NULL` | returns `EXIT_FAILURE` (`1`); nothing else allocated/freed | `e15_driver_mat_a_null` | [x] |
| E16 | `driver` (`driver.c:41`) | `mat_a` OK but `initialize_matrix_from_string(matrix_b, …)` returns `NULL` | `free_matrix(mat_a)`; returns `EXIT_FAILURE` (`1`) | `e16_driver_mat_b_null` | [x] |
| E17 | `driver` (`driver.c:47`) | both matrices OK but `multiply_matrices` returns `NULL` (`width_a != height_b`) | `free_matrix(mat_a)`, `free_matrix(mat_b)`; returns `EXIT_FAILURE` (`1`) | `e17_driver_dimension_mismatch` | [x] |
| E18 | `driver` (`driver.c:53`) | `matrix_to_string(res)` returns `NULL` (its `malloc` fails) | `free_matrix(mat_a)`, `free_matrix(mat_b)`, **plain `free(res)`** (rows leaked — bug preserved); returns `EXIT_FAILURE` (`1`) | `e18_driver_to_string_oom` (child with `RLIMIT_AS`) | [x] |
| E19 | `driver` (`driver.c:67`) | everything succeeds but `write_to_file` returns non-zero (CWD not writable / `matrix.txt` unopenable) | all four frees run, then returns `EXIT_FAILURE` (`1`) | `e19_driver_write_fails` | [x] |

## Generic FFI boundary cases (covered even though not distinct C branches)

| # | case | note | test | ✔ |
|---|------|------|------|---|
| G1 | `initialize_matrix_from_string(NULL, …)` | `strdup(NULL)` — glibc dereferences it, both builds fault identically | `g1_init_null_input_faults_both` (forked children, compare signal) | [x] |
| G2 | `multiply_matrices(NULL, b)` / `(a, NULL)` | C dereferences `mat_a`/`mat_b` with no check — both fault identically | `g2_multiply_null_faults_both` (forked children) | [x] |
| G3 | `matrix_to_string` / `free_matrix` with zero-sized dims | `w==0`, `h==0`, `w==0&&h>0`, `w>0&&h==0` — all *valid* in C (`malloc(0)` ≠ NULL) | `g3_zero_dimensions` | [x] |
| G4 | one step past valid range | `width`/`height` = `-1`, `0`, `1`, `INT_MAX`, `INT_MIN` into `allocate_matrix` / `matrix_to_string` | `g4_dimension_boundaries` | [x] |
| G5 | "out-of-range enum" analogue | the API takes no enums; the equivalent unconstrained `int` inputs are the dimension arguments, which accept **any** `int`. Full `i32` boundary sweep incl. `INT_MIN`, `INT_MAX`, `-1`, `0`, `1` and randomized negatives | `g4_dimension_boundaries`, `g5_random_dimension_fuzz` | [x] |
| G6 | oversized length | `write_to_file` with a >1 MiB content string (exercises many stream flushes) | `g6_write_oversized_content` | [x] |
| G7 | zero length | `write_to_file` with `content == ""` (0 bytes written, `fprintf` returns 0, not `< 0`) | `g7_write_empty_content` | [x] |
| G8 | `atoi` on non-numeric / out-of-range tokens | `"abc"`→0, `"12abc"`→12, `"+7"`, `"  9"`, `"0x10"`→0, `"99999999999999999999"` (clamped by `strtol`), `"-2147483649"` | `g8_atoi_token_forms` | [x] |

## Result

All 19 `ERRORS.md` rows (plus the 7 `E12` sub-rows and the 8 generic-boundary
rows) have a passing differential test in `tests/phase_c_errors.rs`; 27 `#[test]`
functions, all green. Each asserts the same return code / sentinel **and** the
same `stderr` bytes from both `.so`s — not merely "both failed somehow".

### Notes on how the hard rows were made testable

* **E1 / E5 / E18** need a specific `malloc` to fail. An `RLIMIT_AS` cap alone is
  not sufficient: a process always carries a pool of already-mapped-but-free
  heap (measured at ~1 MiB even in a freshly started test binary) that requests
  can be carved out of without growing the address space. Two things were needed:
  1. run each side in a **freshly re-exec'd process** (`spawn_child`) rather than
     a `fork`, so the child does not inherit whatever earlier tests freed;
  2. `constrain_heap_to(budget)` — reserve `budget` bytes, then allocate until
     `malloc` fails, then release the reservation, leaving *exactly* `budget`
     bytes allocatable. `mallopt(M_MMAP_THRESHOLD, 32 MiB)` pins everything to
     the `brk` heap so `free` cannot hand address space back via `munmap`, and
     the stack is pre-faulted because once the address space is full the kernel
     can no longer grow it.
  For E18 the budget window that isolates the `matrix_to_string` allocation while
  letting `res` through was measured as **384 KiB … 917 KiB**; the test uses
  512 KiB.
* **E13 vs E14** are distinguished purely by content length: `/dev/full` with
  more than `BUFSIZ` bytes fails inside `fprintf` (E13), with fewer it only fails
  when `fclose` flushes (E14).
* **E12d** (`filename == NULL`) is a genuine, non-crashing case: glibc passes the
  pointer straight to `open(2)`, which returns `EFAULT`, and `"%s"` renders
  `(null)`. Both builds print `Error opening file '(null)': Bad address`.
* **G1 / G2** are *undefined behaviour* in the C (an unchecked NULL dereference).
  The shipped **release** Rust `.so` reproduces it exactly — `SIGSEGV`, no output.
  A **debug** Rust `.so` additionally carries rustc's `debug_assertions`, which
  turn the same dereference into a "null pointer dereference occurred" panic and
  `SIGABRT`. That is a build-profile artifact, not a translation difference, so
  those two tests compare the exact signal and stderr only when the artifact
  under test has assertions disabled (`ub_strict()`); otherwise they still
  require both sides to terminate abnormally.

### Rows that turned out NOT to be error paths

Derived from the C and corrected after measurement:

* `initialize_matrix_from_string("   ", 0, 1)` succeeds (see `CONFIGS.md`).
* `matrix_to_string` with `width == -1, height == -1` succeeds (`buffer_size` is
  `+11`). Likewise `(1, INT_MIN)` → `buffer_size == 1` and `(65536, 65536)` →
  `buffer_size == 65537`: the `int` arithmetic wraps back to a *positive* value,
  so `malloc` succeeds and the C then dereferences the row array. E10 therefore
  computes `buffer_size` with the same wrapping `int` arithmetic as the C and
  only asserts the `NULL` return for the pairs where it really is `<= 0`.

### Verified by negative control

`scripts/negative_control.sh` builds mutated copies of the C library and feeds
each to the suite in place of the Rust `.so`. Every mutant is caught, which is
what makes the all-green result above meaningful.
