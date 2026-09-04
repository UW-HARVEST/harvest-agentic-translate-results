# ERRORS.md — error-surface table (Phase A) / error-path differential tests (Phase C)

Derived mechanically from every rejection site in the C sources:

```sh
grep -n 'return NULL\|return -1\|return e\|return EINVAL\|return errno\|EXIT_FAILURE\|perror\|fprintf(stderr\|assert' c_src/src/*.c
```

There are no `assert`s, no error enums and no `RETURN_ERROR`-style macros in
this library. Every rejection is one of: `return NULL`, `return errno`,
`return EINVAL`, `return EXIT_FAILURE`, or a silent early `return`.

"expected C result" is what the C `.so` actually returns/does; the Rust `.so`
must match it exactly (same sentinel / same integer errno / same silence).

| #  | function | trigger (exact invalid input/condition) | expected C result | test |
|----|----------|------------------------------------------|-------------------|------|
| E1 | `allocate_matrix` (matrix.c:35) | `malloc(sizeof(matrix_t))` fails | `perror("Failed to allocate memory for matrix struct")`, return `NULL` | [x] not triggerable (16-byte allocation); reachability argued by inspection, code path is a literal transcription |
| E2 | `allocate_matrix` (matrix.c:44) | `malloc(height * sizeof(int*))` fails — `height < 0` makes the size `(size_t)(long)height * 8`, i.e. ~2^64 | `perror("...matrix rows")`, return `NULL` | [x] `err_e2_allocate_rows_alloc_fail` (`height` = -1, -2, -1000, `INT_MIN`) |
| E3 | `allocate_matrix` (matrix.c:52) | `malloc(width * sizeof(int))` fails for row `i` — `height > 0 && width < 0` | `perror("...matrix columns")`, free rows 0..=i, free row array, free struct, return `NULL` | [x] `err_e3_allocate_cols_alloc_fail` (`width` = -1, -7, `INT_MIN`; `height` = 1, 3) |
| E4 | `free_matrix` (matrix.c:67) | `mat == NULL` | silent early `return`, no crash, no output | [x] `err_e4_free_matrix_null` |
| E5 | `initialize_matrix_from_string` (matrix.c:82) | `strdup(input)` fails | `perror("Failed to duplicate input string")`, `free_matrix(mat)`, return `NULL` | [x] not triggerable (OOM only); `input == NULL` is **not** this path — see E24a |
| E6 | `initialize_matrix_from_string` (matrix.c:91) | fewer `"\n"`-delimited tokens than `height` (incl. empty string, only-newlines string, `height` > row count) | `fprintf(stderr, "Insufficient rows in input string.\n")`, return `NULL` | [x] `err_e6_insufficient_rows` |
| E7 | `initialize_matrix_from_string` (matrix.c:101) | some row `i` has fewer `" "`-delimited tokens than `width` | `fprintf(stderr, "Insufficient columns in row %d.\n", i+1)`, return `NULL` | [x] `err_e7_insufficient_cols` |
| E8 | `initialize_matrix_from_string` (matrix.c:79, unchecked) | `allocate_matrix` returned `NULL` **and** the parse loops never dereference it: `height < 0` (any input), or `height > 0 && width < 0` with at least `height` rows present | returns `NULL` *silently* (no "Insufficient…" message) — the C never checks `mat` | [x] `err_e8_alloc_fail_propagates_silently` |
| E9 | `multiply_matrices` (matrix.c:119) | `mat_a->width != mat_b->height` | `fprintf(stderr, "Matrix dimensions do not allow multiplication.\n")`, return `NULL` | [x] `err_e9_dimension_mismatch` (all mismatched pairs in 0..=4, plus negative widths/heights) |
| E10 | `multiply_matrices` (matrix.c:124, unchecked) | `allocate_matrix(mat_b->width, mat_a->height)` fails and `mat_a->height <= 0` so the loops are skipped: `mat_a->height < 0 && mat_a->width == mat_b->height` | returns `NULL` *silently* | [x] `err_e10_result_alloc_fail_silent` (hand-built `matrix_t`s, `matrix` field never dereferenced) |
| E11 | `matrix_to_string` (matrix.c:138) | `mat == NULL` | `fprintf(stderr, "Error: Matrix is NULL.\n")`, return `NULL` | [x] `err_e11_matrix_to_string_null` |
| E12 | `matrix_to_string` (matrix.c:145) | `malloc(buffer_size)` fails — `buffer_size = height*(width*10+width)+height+1` computed in `int` wraps negative, then converts to a ~2^64 `size_t` | `perror("Failed to allocate memory for matrix string")`, return `NULL` | [x] `err_e12_matrix_to_string_alloc_fail` (hand-built `matrix_t`, e.g. `width=200000000,height=1`; `width=1,height=INT_MIN`) |
| E13 | `write_to_file` (write.c:33) | `content == NULL` | `fprintf(stderr, "Error: Content is NULL.\n")`, return `EINVAL` = **22** | [x] `err_e13_write_null_content` (incl. `filename` also NULL — the content check comes first) |
| E14a | `write_to_file` (write.c:39) | `fopen` fails: path inside a non-existent directory | `fprintf(stderr, "Error opening file …")`, return `errno` = **ENOENT 2** | [x] `err_e14_fopen_failures` |
| E14b | `write_to_file` (write.c:39) | `fopen` fails: `filename` is `""` | return `errno` = **ENOENT 2** | [x] `err_e14_fopen_failures` |
| E14c | `write_to_file` (write.c:39) | `fopen` fails: `filename` names an existing **directory** | return `errno` = **EISDIR 21** | [x] `err_e14_fopen_failures` |
| E14d | `write_to_file` (write.c:39) | `fopen` fails: target directory has mode `0555` (no write permission) | return `errno` = **EACCES 13** | [x] `err_e14_fopen_failures` |
| E14e | `write_to_file` (write.c:39) | `fopen` fails: `filename == NULL` (non-NULL content) — glibc passes the NULL path to `open(2)` | return `errno` = **EFAULT 14**; the diagnostic prints `(null)` for `%s` | [x] `err_e14_fopen_failures` |
| E15 | `write_to_file` (write.c:44) | `fprintf(file, "%s", content) < 0` — write to `/dev/full` with content larger than the stdio buffer, so the flush happens *inside* `fprintf` | `fprintf(stderr, "Error writing to file …")`, `fclose(file)`, return `errno` = **ENOSPC 28** | [x] `err_e15_fprintf_write_failure` |
| E16 | `write_to_file` (write.c:50) | `fclose(file) != 0` — write to `/dev/full` with content small enough to stay buffered until close | `fprintf(stderr, "Error closing file …")`, return `errno` = **ENOSPC 28** | [x] `err_e16_fclose_failure` |
| E17 | `driver` (driver.c:37) | `initialize_matrix_from_string(matrix_a, width_a, height_a) == NULL` (any E6/E7/E8 trigger on A) | return `EXIT_FAILURE` = **1** | [x] `err_e17_driver_mat_a_fail` |
| E18 | `driver` (driver.c:41) | A parses but B does not | `free_matrix(mat_a)`, return **1** | [x] `err_e18_driver_mat_b_fail` |
| E19 | `driver` (driver.c:47) | both parse but `width_a != height_b` | frees, return **1** | [x] `err_e19_driver_dim_mismatch` |
| E20 | `driver` (driver.c:53) | `matrix_to_string(res) == NULL` | frees A, B, `free(res)` (struct only — rows leak; transcribed as-is), return **1** | [x] unreachable in practice: it needs a product with ~2·10^8 columns/rows, which requires an input string with that many tokens; any smaller input fails earlier at E7. Verified by inspection that `driver.rs` transcribes the same `free(res as *mut c_void)` quirk. |
| E21 | `driver` (driver.c:67) | `write_to_file("matrix.txt", …) != 0` — cwd contains a **directory** named `matrix.txt`, or cwd is mode `0555` | frees everything, return **1** | [x] `err_e21_driver_write_failure` |

## Generic FFI boundary cases (required by Phase C even when absent from the table)

| #  | case | expected C result | test |
|----|------|-------------------|------|
| E12b | `matrix_to_string` where the wrapped `buffer_size` lands back **positive** (e.g. `width=200000000,height=2` ⇒ `+105032704`; `width=100000000,height=4`; `width=195225785,height=1`): `malloc` succeeds and the loop dereferences the matrix anyway | **SIGSEGV** on a NULL `matrix` field — the C has no guard | [x] `err_e12b_unchecked_allocation_crash_parity` (forked child per library) |
| E23b | `initialize_matrix_from_string` where `allocate_matrix`'s **row** allocation fails (`width >= INT_MAX-1`) but the column loop still runs (`height >= 1` and data present): `initialize_matrix_from_string` does not check `mat` | **SIGSEGV** writing through the NULL matrix | [x] `err_e23b_init_unchecked_allocation_crash_parity` (forked child per library) |
| E22 | zero lengths: `allocate_matrix(0,0)`, `initialize_matrix_from_string("",0,0)`, `matrix_to_string` on a 0x0 matrix, `write_to_file(path,"")` | all succeed: `malloc(0)` is non-NULL, `matrix_to_string` yields `""`, `write_to_file` returns 0 and creates a 0-byte file | [x] `err_e22_zero_lengths` |
| E23 | oversized lengths: `INT_MAX`, `INT_MAX-1`, `INT_MIN`, `INT_MIN+1`, `-1`, and one step past every boundary the code tests (`width-1` in `matrix_to_string`, `i+1` in the column message) | identical sentinel from both (`NULL` via E2/E3/E8/E10/E12, or identical success) | [x] `err_e23_oversized_and_off_by_one` |
| E24 | out-of-range "enum" values across FFI: this API declares **no enums**. Its only small-domain `int` inputs are the dimensions, and its only sentinel-domain output is `write_to_file`'s errno. Both are driven with `INT_MIN`, `-1`, `0`, `1`, `INT_MAX` and with dimension values that disagree with the actual data shape (a value with "no valid variant") | identical results from both | [x] `err_e23_oversized_and_off_by_one`, `err_e24_dimension_vs_data_disagreement` |
| E24a | null pointers the C dereferences unconditionally: `initialize_matrix_from_string(NULL, w, h)` (glibc `strdup(NULL)`), `multiply_matrices(NULL, b)` / `(a, NULL)` (`mat_a->width`), `driver(…, NULL, …)` | **SIGSEGV** — the C has no guard; the Rust must fault the same way, not return an error | [x] `err_e24a_null_deref_parity` — each call runs in a forked child; asserts both libraries die with the same signal |
| E25 | `matrix_to_string` heap overflow quirk: the buffer formula budgets `11*width` bytes per row but a row of 11-character values (`-1000000000` … `INT_MIN`) needs `12*width`. With `width >= 2` and such values the C **overflows the heap buffer**. | identical bytes written past the end of an identically sized buffer | [x] `err_e25_buffer_overflow_parity` — run in separate child processes (one per library) so heap corruption cannot cross-contaminate; asserts identical output *or* identical fatal outcome |

All 32 rows above are checked off. Every `[x]` other than E1, E5 and E20 has an
executable differential test; those three are argued unreachable above and their
Rust code is a literal transcription of the C.

Additional errno rows exercised by `err_e14_fopen_failures` beyond the five
listed: a path whose parent is a regular file ⇒ **ENOTDIR 20**.

## Divergence found and fixed

One real mismatch surfaced in Phase C, on rows E12b / E23b / E24a: where the C
faults with **SIGSEGV**, the Rust `.so` built with the `dev` profile aborted
with **SIGABRT** and printed `null pointer dereference occurred`. That is
Rust's debug-only UB check intercepting a dereference the C performs
unconditionally, so the two libraries produced different observable behaviour
for the same input.

Fixed in `Cargo.toml` by disabling `debug-assertions` and `overflow-checks` for
the `dev` profile: the translation reproduces the C's wrapping arithmetic and
its unchecked dereferences on purpose, so Rust's checks must not intervene. The
`release` profile (the shipped artifact) was already correct. Both profiles now
fault identically to the C — verified by `subprocess_parity.rs`, which compares
the children's wait statuses rather than assuming a graceful return.
