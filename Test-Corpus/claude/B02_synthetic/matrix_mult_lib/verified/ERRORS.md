# ERRORS.md — Error-surface table (Phase A) / Phase C checklist

Mechanically derived from every `return NULL`, `return EINVAL`, `return errno`,
`return EXIT_FAILURE`, `perror(...)`, `fprintf(stderr, ...)` and every `== NULL`
/ `!=` guard in `c_src/src/{matrix,write,driver}.c` (see the grep in the session
log). There are **no** `assert`s, **no** error enums and **no** explicit
min/max range constants in the C sources; the only numeric limits that matter
are those of `int` (`INT_MIN`/`INT_MAX`) and the implicit
`int → size_t` sign-extension performed by the `malloc()` calls.

`✔` = a differential test (C `.so` vs Rust `.so`, both via `libloading`) exists
and passes, asserting the *same* sentinel/error code **and** the same bytes on
`stderr`.

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| E1 | `allocate_matrix` | `malloc(sizeof(matrix_t))` (16 B) returns NULL — reached by capping `RLIMIT_AS` in a child process and draining the heap | `perror("Failed to allocate memory for matrix struct")`, return `NULL` | ✔ `oom_e1_allocate_struct_malloc_fails` (`tests/oom_parity.rs`) |
| E2 | `allocate_matrix` | `height < 0` ⇒ `malloc((size_t)height * 8)` = huge ⇒ NULL (e.g. `height = -1`, `INT_MIN`, `-7`) | `perror("Failed to allocate memory for matrix rows")`, `free(mat)`, return `NULL` | ✔ `err_e2_allocate_negative_height` |
| E3 | `allocate_matrix` | `height > 0 && width < 0` ⇒ row `malloc((size_t)width * 4)` = huge ⇒ NULL | `perror("Failed to allocate memory for matrix columns")`, free rows `0..=i`, `free(mat->matrix)`, `free(mat)`, return `NULL` | ✔ `err_e3_allocate_negative_width` |
| E4 | `allocate_matrix` | huge positive dims (`width` ≈ 2e9 ⇒ 8 GiB row) — malloc may or may not fail depending on overcommit | either `NULL` + `perror(...columns)` or a valid matrix — **must be the same for both libs** | ✔ `err_e4_allocate_huge_width` |
| E5 | `free_matrix` | `mat == NULL` | returns immediately, no output, no crash | ✔ `err_e5_free_null` |
| E6 | `initialize_matrix_from_string` | `strdup(input)` returns NULL — reached with a 64 MiB input and a 1 MiB address-space slack in a child process (small allocations still succeed, so `allocate_matrix` works and `strdup` is the failing call) | `perror("Failed to duplicate input string")`, `free_matrix(mat)`, `NULL` | ✔ `oom_e6_strdup_fails` (`tests/oom_parity.rs`) |
| E7 | `initialize_matrix_from_string` | fewer `\n`-separated row tokens than `height` (incl. `input == ""`, `"\n"`, `"\n\n"`, `height` > #rows) | `fprintf(stderr, "Insufficient rows in input string.\n")`, `free`, `NULL` | ✔ `err_e7_insufficient_rows` |
| E8 | `initialize_matrix_from_string` | some row has fewer ` `-separated column tokens than `width` | `fprintf(stderr, "Insufficient columns in row %d.\n", i + 1)` (1-based row index!), `free`, `NULL` | ✔ `err_e8_insufficient_columns` |
| E9 | `initialize_matrix_from_string` | `width < 0` (allocation fails ⇒ `mat == NULL`, but the *unchecked* `mat` is never dereferenced because the `j < width` loop body never runs) | `perror(...columns)` from `allocate_matrix`, then `NULL` (or `"Insufficient rows"` first if rows run out) | ✔ `err_e9_init_negative_width` |
| E9b | `initialize_matrix_from_string` | `width > 0` **but so large that the row `malloc` fails** (`width = 500000000` under a capped `RLIMIT_AS`). Now `mat == NULL` *and* the `j < width` loop body **does** run ⇒ C dereferences the unchecked NULL and dies. (This is the case the original E9 reasoning missed: it only holds for *negative* widths.) | `perror(...columns)` then **SIGSEGV** | ✔ `oom_e9b_init_dereferences_unchecked_null` (`tests/oom_parity.rs`) |
| E10 | `initialize_matrix_from_string` | `height < 0` (allocation fails; the `i < height` loop never runs) | `perror(...rows)` from `allocate_matrix`, then `NULL` | ✔ `err_e10_init_negative_height` |
| E11 | `initialize_matrix_from_string` | `input == NULL` | `strdup(NULL)` ⇒ SIGSEGV in **both** libraries (C UB) | not testable in-process (crashes both); verified by inspection: Rust passes the pointer straight to `strdup`. The *same class* of crash **is** verified end-to-end by E9b/E13b/E29b, which compare the terminating signal of a C child against a Rust child |
| E12 | `multiply_matrices` | `mat_a->width != mat_b->height` (incl. `0 != 1`, `1 != 0`, negatives) | `fprintf(stderr, "Matrix dimensions do not allow multiplication.\n")`, `NULL` | ✔ `err_e12_dim_mismatch` |
| E13 | `multiply_matrices` | dims agree but `mat_b->width < 0` ⇒ inner `allocate_matrix` fails, `result == NULL` never dereferenced (`j < mat_b->width` loop empty) | `perror(...columns)`, then `NULL` returned (unchecked) | ✔ `err_e13_mul_negative_result_width` |
| E13b | `multiply_matrices` | dims agree and `mat_b->width > 0` **but so large that the inner `allocate_matrix` fails** ⇒ `result == NULL` *is* dereferenced by `result->matrix[i][j] = 0` | `perror(...columns)` then **SIGSEGV** | ✔ `oom_e13b_multiply_dereferences_unchecked_null` (`tests/oom_parity.rs`) |
| E14 | `multiply_matrices` | dims agree but `mat_a->height < 0` ⇒ inner `allocate_matrix` fails (`i < mat_a->height` loop empty) | `perror(...rows)`, then `NULL` | ✔ `err_e14_mul_negative_result_height` |
| E15 | `multiply_matrices` | `mat_a == NULL` or `mat_b == NULL` | dereference of `mat_a->width` ⇒ SIGSEGV in **both** (C UB) | not testable in-process; verified by inspection (`src/matrix.rs:197` dereferences unconditionally, exactly like C) |
| E16 | `matrix_to_string` | `mat == NULL` | `fprintf(stderr, "Error: Matrix is NULL.\n")`, `NULL` | ✔ `err_e16_to_string_null` |
| E17 | `matrix_to_string` | `buffer_size = h*(11w) + h + 1` overflows `int` to a **negative** value ⇒ `malloc((size_t)negative)` = huge ⇒ NULL. Verified for `(w,h)` = `(1, 2e8)`, `(2, 1e8)`, `(11, 2e7)`, `(1, INT_MAX)`, `(-1, 1)`, `(1, -1)`, `(INT_MIN, 1)`. Shapes whose wrap lands back on a small **positive** value (`(-1,-1)` ⇒ 11, `(1, INT_MIN)` ⇒ 1) instead succeed and return `""` because `height <= 0` renders no row — the Rust port must match that too, and does. | `perror("Failed to allocate memory for matrix string")` + `NULL`, resp. `""` | ✔ `err_e17_to_string_buffer_overflow` |
| E17b | `matrix_to_string` | `buffer_size` stays **positive** but the `malloc` fails anyway (`width = 100000000, height = 1` ⇒ 1 100 000 002 bytes under a capped `RLIMIT_AS`) | `perror("Failed to allocate memory for matrix string")`, `NULL` | ✔ `oom_e17b_to_string_positive_size_malloc_fails` (`tests/oom_parity.rs`) |
| E18 | `write_to_file` | `content == NULL` | `fprintf(stderr, "Error: Content is NULL.\n")`, return `EINVAL` (22) | ✔ `err_e18_write_null_content` |
| E19 | `write_to_file` | `fopen` fails: path component does not exist | `fprintf(stderr, "Error opening file '%s': %s\n", ...)`, return `errno` = `ENOENT` (2) | ✔ `err_e19_write_enoent` |
| E20 | `write_to_file` | `fopen` fails: `filename` **is a directory** | same message, return `errno` = `EISDIR` (21) | ✔ `err_e20_write_eisdir` |
| E21 | `write_to_file` | `fopen` fails: no write permission (0o400 file / 0o500 dir) | same message, return `errno` = `EACCES` (13) | ✔ `err_e21_write_eacces` |
| E22 | `write_to_file` | `filename == ""` (empty path) | same message, return `errno` = `ENOENT` (2) | ✔ `err_e22_write_empty_filename` |
| E23 | `write_to_file` | `filename` longer than `PATH_MAX` | same message, return `errno` = `ENAMETOOLONG` (36) | ✔ `err_e23_write_enametoolong` |
| E24 | `write_to_file` | `filename == NULL` (invalid pointer handed to `fopen`) | glibc `fopen(NULL, "w")` fails ⇒ return `errno` (`EFAULT`, 14) — must match bit-for-bit | ✔ `err_e24_write_null_filename` |
| E25 | `write_to_file` | `fprintf(file, ...)` returns `< 0` **or** `fclose(file) != 0` (out-of-space device: `/dev/full`) | `fprintf(stderr, "Error writing/closing file ...")`, return `errno` = `ENOSPC` (28) | ✔ `err_e25_write_enospc_devfull` |
| E26 | `driver` | `initialize_matrix_from_string(matrix_a, …)` returns `NULL` (bad `matrix_a`, or `width_a`/`height_a` invalid) | return `EXIT_FAILURE` (1) | ✔ `err_e26_driver_bad_a` |
| E27 | `driver` | `mat_a` ok, `initialize_matrix_from_string(matrix_b, …)` returns `NULL` | `free_matrix(mat_a)`, return `EXIT_FAILURE` (1) | ✔ `err_e27_driver_bad_b` |
| E28 | `driver` | `width_a != height_b` ⇒ `multiply_matrices` returns `NULL` | frees both, return `EXIT_FAILURE` (1) | ✔ `err_e28_driver_dim_mismatch` |
| E29 | `driver` | `matrix_to_string(res)` returns `NULL` (needs `res` with `11*w*h > INT_MAX`, i.e. ≥ ~780 MB of successfully allocated rows) | frees `mat_a`, `mat_b`, `free(res)` (rows leaked — reproduced verbatim in Rust), return `EXIT_FAILURE` | unreachable in practice (multi-GB allocation); code path reproduced verbatim, no test |
| E29b | `driver` | `width_a` so large that `initialize_matrix_from_string`'s row `malloc` fails ⇒ the unchecked NULL dereference happens inside `driver` | **SIGSEGV** (after `perror(...columns)`) | ✔ `oom_driver_null_deref` (`tests/oom_parity.rs`) |
| E30 | `driver` | `write_to_file("matrix.txt", …) != 0` (e.g. a **directory** named `matrix.txt` in the CWD, or a read-only CWD) | all frees, return `EXIT_FAILURE` (1) | ✔ `err_e30_driver_write_fails` |
| E31 | generic | out-of-range enum value across the FFI boundary | **N/A** — the public API (`matrix.h`, `write.h`) has no `enum` parameter; the only scalar parameters are `int` width/height, whose full range (`INT_MIN`, `-1`, `0`, `1`, `INT_MAX`) is covered by E2–E4, E9, E10, E17 and CONFIGS rows C1–C6 | ✔ (covered) |
| E32 | generic | one step past a valid range: `width`/`height` = `-1` vs `0`, `INT_MAX`, `INT_MIN`; row index `height` vs `height-1`; `width - 1` separator boundary in `matrix_to_string` | same sentinel/bytes as C | ✔ `err_e32_boundary_dims` |
| E33 | generic | zero lengths: `width = 0`, `height = 0`, `width = height = 0`, empty content string | **not** errors in C: they succeed (see CONFIGS C1–C3, C24) | ✔ (covered in Phase B) |
| E34 | generic | `write_to_file` with a **trailing slash** on a regular file | `fopen` fails, return `errno` (`ENOTDIR`, 20) | ✔ `err_e20_write_eisdir` (case c) |
| E35 | ordering | every error path above, but with the **Rust** library called FIRST. Several C paths `return errno`, i.e. a *global*: a port that read a stale `errno` would still match when C ran first. Re-runs E2/E3/E7/E8/E12/E17/E18–E25 with the order flipped and interleaved with successful calls | identical error codes and identical `stderr` | ✔ `err_order_write_rust_first`, `err_order_matrix_rust_first`, `err_e26b_driver_errors_rust_first` |
| E36 | generic | randomised error/valid mix: 4000 + 800 + 600 + 600 iterations of random dimensions (incl. negative), random/mutated/soup input text and random file targets — all rejections compared | identical sentinel + `stderr` | ✔ `tests/fuzz.rs` |

## Observed error codes (confirmed identical for C and Rust)

| condition | value returned by both |
|-----------|------------------------|
| `content == NULL` | `EINVAL` = 22 |
| missing directory component / empty filename | `ENOENT` = 2 |
| filename is a directory | `EISDIR` = 21 |
| trailing slash on a regular file | `ENOTDIR` = 20 |
| unwritable file (0o444) / unwritable directory (0o500) | `EACCES` = 13 |
| filename > `PATH_MAX` / component > `NAME_MAX` | `ENAMETOOLONG` = 36 |
| `filename == NULL` | `EFAULT` = 14 |
| `/dev/full` target (non-empty content) | `ENOSPC` = 28 |
| successful write (incl. `/dev/null`, empty content) | `0` |
| any `driver` failure | `EXIT_FAILURE` = 1 |
| all `matrix.c` failures | `NULL` (plus the exact `perror`/`fprintf` diagnostic) |

## Divergence found and fixed (dev profile)

Rows **E9b / E13b / E29b** exposed the only real behavioural difference found
during this verification:

| | C | Rust `release` | Rust `dev` **before** the fix | Rust `dev` **after** the fix |
|---|---|---|---|---|
| terminating signal | `SIGSEGV` (11) | `SIGSEGV` (11) | **`SIGABRT` (6)** | `SIGSEGV` (11) |
| `stderr` | `Failed to allocate memory for matrix columns: Cannot allocate memory` | identical | identical **plus** `thread '<unnamed>' panicked at src/matrix.rs:58:15: null pointer dereference occurred` … | identical |

Cause: `debug-assertions` (on by default in the `dev` profile) makes rustc inject
UB checks, so the NULL dereference that the C code performs unchecked turned into
a Rust panic which — crossing an `extern "C"` boundary — aborts.

Fix (`Cargo.toml`, no change to the translated code, since the C is the ground
truth and the Rust must reproduce its unchecked dereference):

```toml
[profile.dev]
debug-assertions = false
overflow-checks = false
```

Both profiles now behave identically to the C library on every row of this
table.
