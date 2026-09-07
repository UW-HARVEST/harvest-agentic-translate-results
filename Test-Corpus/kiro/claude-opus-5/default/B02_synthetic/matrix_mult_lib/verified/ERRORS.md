# ERRORS.md — error-surface table

Mechanically derived from every rejection point in `c_src/src/*.c`. Enumeration
commands used:

```
grep -n 'return NULL\|return -1\|return errno\|return EINVAL\|return EXIT_\|== NULL\|!= NULL\|is_null\|assert\|perror\|fprintf(stderr' c_src/src/*.c
```

There are no `assert`s, no error enums, no `RETURN_ERROR`-style macros and no
named min/max constants in this library. Every rejection is either a
`return NULL`, a `return <errno>`, or a `return EXIT_FAILURE`. One row per
distinct branch.

`E` = triggerable from outside the `.so` and covered by a differential test.
`OOM` = only reachable on allocation failure of a *small* request — documented,
not directly forceable through the FFI boundary without an allocator interposer.

| #  | function | trigger (exact invalid input/condition) | expected C result | test |
|----|----------|------------------------------------------|-------------------|------|
| 1  | `allocate_matrix` | `malloc(sizeof(matrix_t))` returns NULL (matrix.c:35) | `perror("Failed to allocate memory for matrix struct")`; return `NULL` | OOM |
| 2  | `allocate_matrix` | `malloc(height * sizeof(int*))` returns NULL (matrix.c:44) — forceable with `height < 0` (e.g. `-1`), since `(size_t)(-1)*8` is astronomically large | `perror("Failed to allocate memory for matrix rows")`; `free(mat)`; return `NULL` | E: `err_01_allocate_matrix_negative_height` |
| 3  | `allocate_matrix` | `malloc(width * sizeof(int))` for a row returns NULL (matrix.c:52) — forceable with `width < 0 && height > 0` | `perror("Failed to allocate memory for matrix columns")`; free rows `0..=i`, `free(mat->matrix)`, `free(mat)`; return `NULL` | E: `err_02_allocate_matrix_negative_width` |
| 4  | `free_matrix` | `mat == NULL` (matrix.c:67) | early `return`, no diagnostic, no crash | E: `err_03_free_matrix_null` |
| 5  | `initialize_matrix_from_string` | `strdup(input)` returns NULL (matrix.c:82) | `perror("Failed to duplicate input string")`; `free_matrix(mat)`; return `NULL` | OOM |
| 6  | `initialize_matrix_from_string` | `row_token == NULL` — fewer `\n`-separated rows in `input` than `height` (matrix.c:91) | `fprintf(stderr, "Insufficient rows in input string.\n")`; `free(input_copy)`; `free_matrix(mat)`; return `NULL` | E: `err_04_insufficient_rows` |
| 7  | `initialize_matrix_from_string` | `col_token == NULL` — row `i` has fewer space-separated columns than `width` (matrix.c:101) | `fprintf(stderr, "Insufficient columns in row %d.\n", i+1)` (1-based row number); `free(input_copy)`; `free_matrix(mat)`; return `NULL` | E: `err_05_insufficient_cols` |
| 8  | `initialize_matrix_from_string` | empty input `""` with `height > 0` — first `strtok_r` yields NULL (row 6's branch, distinct input class) | "Insufficient rows"; return `NULL` | E: `err_06_empty_input` |
| 9  | `initialize_matrix_from_string` | `height < 0` — `allocate_matrix` returns NULL and the C **does not check it** (matrix.c:79); the `i < height` loop body never runs | returns the unchecked `NULL` `mat` (no diagnostic beyond `allocate_matrix`'s `perror`) | E: `err_07_init_negative_height` |
| 10 | `initialize_matrix_from_string` | `width < 0 && height > 0`, enough rows present — `mat` is NULL, inner `j < width` loop never runs, no NULL deref | returns `NULL` | E: `err_08_init_negative_width` |
| 11 | `multiply_matrices` | `mat_a->width != mat_b->height` (matrix.c:119-121) | `fprintf(stderr, "Matrix dimensions do not allow multiplication.\n")`; return `NULL` | E: `err_09_dim_mismatch` |
| 12 | `matrix_to_string` | `mat == NULL` (matrix.c:138) | `fprintf(stderr, "Error: Matrix is NULL.\n")`; return `NULL` | E: `err_10_to_string_null` |
| 13 | `matrix_to_string` | `malloc(buffer_size)` returns NULL (matrix.c:145) — forceable because `buffer_size` is an `int` that overflows negative for large `width`/`height` (e.g. `width=200000000, height=1` → `-2094967295` → `(size_t)` huge) | `perror("Failed to allocate memory for matrix string")`; return `NULL` | E: `err_11_to_string_size_overflow` |
| 14 | `write_to_file` | `content == NULL` (write.c:33) | `fprintf(stderr, "Error: Content is NULL.\n")`; return `EINVAL` (22) | E: `err_12_write_null_content` |
| 15 | `write_to_file` | `fopen(filename,"w")` returns NULL (write.c:39) — nonexistent directory | `fprintf(stderr, "Error opening file '%s': %s\n", …)`; return `errno` = `ENOENT` (2) | E: `err_13_write_fopen_enoent` |
| 16 | `write_to_file` | `fopen` fails: `filename` is an existing **directory** | as row 15; return `errno` = `EISDIR` (21) | E: `err_14_write_fopen_eisdir` |
| 17 | `write_to_file` | `fopen` fails: `filename` is `""` | as row 15; return `errno` = `ENOENT` (2) | E: `err_15_write_fopen_empty_name` |
| 18 | `write_to_file` | `fopen` fails: `filename == NULL` (glibc returns NULL, `errno=EFAULT`; `%s` of a NULL prints `(null)`) | as row 15; return `errno` = `EFAULT` (14) | E: `err_16_write_fopen_null_name` |
| 19 | `write_to_file` | `fopen` fails: no write permission on the target (mode `0555` dir) | as row 15; return `errno` = `EACCES` (13) | E: `err_17_write_fopen_eacces` |
| 20 | `write_to_file` | `fprintf(file, "%s", content) < 0` (write.c:45) — write error detected at `fprintf` time; needs content larger than the stdio buffer so the flush happens inside `fprintf` (`/dev/full`) | `fprintf(stderr, "Error writing to file '%s': %s\n", …)`; `fclose(file)`; return `errno` = `ENOSPC` (28) | E: `err_18_write_fprintf_enospc_large` |
| 21 | `write_to_file` | `fclose(file) != 0` (write.c:51) — write error surfaces only at flush time (small content to `/dev/full`) | `fprintf(stderr, "Error closing file '%s': %s\n", …)`; return `errno` = `ENOSPC` (28) | E: `err_19_write_fclose_enospc_small` |
| 22 | `driver` | `initialize_matrix_from_string(matrix_a, …)` returns NULL (driver.c:37-38) | return `EXIT_FAILURE` (1) | E: `err_20_driver_bad_a` |
| 23 | `driver` | `initialize_matrix_from_string(matrix_b, …)` returns NULL (driver.c:41-43) | `free_matrix(mat_a)`; return `EXIT_FAILURE` (1) | E: `err_21_driver_bad_b` |
| 24 | `driver` | `multiply_matrices` returns NULL, i.e. `width_a != height_b` (driver.c:47-50) | free both; return `EXIT_FAILURE` (1) | E: `err_22_driver_dim_mismatch` |
| 25 | `driver` | `matrix_to_string(res)` returns NULL (driver.c:53-57) | free both, `free(res)` (struct only — the C leaks the rows; reproduced verbatim) ; return `EXIT_FAILURE` (1) | unreachable: `res` is non-NULL here and a `buffer_size` overflow would require a `res` whose own row allocation already failed |
| 26 | `driver` | `write_to_file("matrix.txt", res_str) != 0` (driver.c:67-68) — cwd not writable | return `EXIT_FAILURE` (1) | E: `err_23_driver_write_fails` |

## Generic FFI boundary cases (covered even though not distinct C branches)

| # | case | note | test |
|---|------|------|------|
| G1 | `matrix_to_string(NULL)` | = row 12 | `err_10_to_string_null` |
| G2 | `free_matrix(NULL)` | = row 4 | `err_03_free_matrix_null` |
| G3 | `write_to_file(NULL, NULL)` | content check happens **first**, so `EINVAL` wins over the NULL filename | `err_24_write_both_null` |
| G4 | zero lengths: `width==0`, `height==0`, `0x0` | `malloc(0)` is non-NULL; loops don't run; **valid**, not an error | `CONFIGS.md` rows A3–A5, C2–C4 |
| G5 | oversized lengths: `width`/`height` = `INT_MAX`, `INT_MIN`, `INT_MIN+1` | huge/negative `malloc` → NULL propagation per rows 2/3/9/10 | `err_25_extreme_dims` |
| G6 | one past valid range: `height = -1`, `width = -1` | rows 2/3 | `err_01`, `err_02` |
| G7 | out-of-range "enum" ints across FFI | **no enum type exists** in this API (`grep -n 'enum' c_src/` → 0 hits). The nearest equivalent is arbitrary `int` in the `width`/`height` parameters, whose whole `int` range (incl. values with no valid meaning: negative, `INT_MAX`, `INT_MIN`) is fuzzed against C | `err_25_extreme_dims`, `cfg_fuzz_dims` |
| G8 | `initialize_matrix_from_string` with non-numeric / overflowing tokens | `atoi` is called for real in both (glibc), so `"abc"`→0, `"99999999999"`→`(int)LONG_MAX`=-1, `"12abc"`→12 | `CONFIGS.md` row B7 |

## Known C undefined behaviour — deliberately NOT exercised

`matrix_to_string` allocates `height*(width*10 + width) + height + 1` bytes =
`11*width` per row, but a row of `width` values that are each 11 characters long
(e.g. `-2147483648`) needs `12*width` bytes. The C therefore **heap-overflows**
via `strcat` for 11-character values. The Rust reproduces the same arithmetic and
the same unchecked `strcat`, but comparing two heap-corrupting runs is not
meaningful, so the differential tests keep values to ≤10 characters
(`|v| <= 999_999_999`), which is exactly the largest width-independent safe
bound. This is documented, not fixed — the C is ground truth.

## Phase C result

Every row that is reachable through the FFI boundary has a passing differential
test that asserts the SAME sentinel/error code, not merely "both failed":

```
cargo test --test phase_c_errors -- --test-threads=1
test result: ok. 29 passed; 0 failed
```

| row(s) | status |
|--------|--------|
| 2, 3 | [x] `err_01_allocate_matrix_negative_height`, `err_02_allocate_matrix_negative_width` |
| 4 | [x] `err_03_free_matrix_null` |
| 6 | [x] `err_04_insufficient_rows` (also asserts the exact message text) |
| 7 | [x] `err_05_insufficient_cols` (also asserts the 1-based `row 3` in the message) |
| 8 | [x] `err_06_empty_input` |
| 9 | [x] `err_07_init_negative_height` |
| 10 | [x] `err_08_init_negative_width` |
| 11 | [x] `err_09_dim_mismatch` (also asserts the exact message text) |
| 12 | [x] `err_10_to_string_null` (also asserts the exact message text) |
| 13 | [x] `err_11_to_string_size_overflow` (asserts the C really took the `perror` branch) |
| 14 | [x] `err_12_write_null_content` → both return `EINVAL` (22) |
| 15 | [x] `err_13_write_fopen_enoent` → both return `ENOENT` (2) |
| 16 | [x] `err_14_write_fopen_eisdir` → both return `EISDIR` (21) |
| 17 | [x] `err_15_write_fopen_empty_name` → both return `ENOENT` (2) |
| 18 | [x] `err_16_write_fopen_null_name` → both return `EFAULT` (14) |
| 19 | [x] `err_17_write_fopen_eacces` → both return `EACCES` (13) |
| 20 | [x] `err_18_write_fprintf_enospc_large` → both return `ENOSPC` (28); asserts the `Error writing` branch |
| 21 | [x] `err_19_write_fclose_enospc_small` → both return `ENOSPC` (28); asserts the `Error closing` branch |
| 22 | [x] `err_20_driver_bad_a` → both return `EXIT_FAILURE` (1) |
| 23 | [x] `err_21_driver_bad_b` |
| 24 | [x] `err_22_driver_dim_mismatch` |
| 26 | [x] `err_23_driver_write_fails` (read-only cwd → `EXIT_FAILURE` + identical diagnostic) |
| G3 | [x] `err_24_write_both_null` (content check wins → `EINVAL`) |
| G5/G6/G7 | [x] `err_25_extreme_dims`, `cfg_fuzz_dims` (400 randomized dimension pairs over `INT_MIN`…`INT_MAX`) |
| extra | [x] `err_26_multiply_negative_dims_handbuilt`, `err_27_to_string_degenerate_dims`, `err_28_driver_degenerate_dims` |

### Rows 1, 5, 25 — not forceable, and why

- **Rows 1 and 5** need `malloc(16)` / `strdup` of a short string to fail. Both
  are small requests; forcing them would require interposing the allocator,
  which would also change the C's behaviour. The Rust code paths were verified
  by inspection to be structurally identical, and their `perror` literals were
  verified to match the C **mechanically**:

  ```
  $ diff <(grep -oh '"[^"]*"' c_src/src/*.c | sort -u) \
         <(grep -oh 'c"[^"]*"' translation/src/*.rs | sed 's/^c//' | sort -u)
  # only the license-comment words "AS IS" and "Software" appear on the C side;
  # every runtime literal, including all 5 perror strings, is byte-identical
  ```

- **Row 25** (`matrix_to_string` returning NULL inside `driver`) is unreachable:
  `res` is non-NULL at that point, and a `buffer_size` overflow would require a
  `res` whose own row allocations had already failed.

### Enum values across FFI

There is no `enum` in this API (`grep -rn 'enum' c_src/src c_src/include` → 0
hits), so the equivalent class of "an integer with no valid variant" is an
out-of-range `width`/`height`. `err_25_extreme_dims` and `cfg_fuzz_dims` sweep
that space, including `INT_MIN`, `INT_MIN+1`, `-1`, `0`, and `INT_MAX`, and
compare the resulting sentinel and diagnostic.

`height` is intentionally never a large positive value: the C would then attempt
billions of row allocations, whose outcome depends on the host's overcommit state
rather than on the translation. The same reasoning excludes
`matrix_to_string` dimension pairs whose wrapped `buffer_size` comes out
*positive* (e.g. `INT_MAX x 1` → `2147483639`): there `malloc` succeeds and the C
dereferences a row array it does not have, faulting identically in both
implementations. `err_11` asserts this precondition rather than assuming it.

## Test-suite validation (mutation check)

To prove these comparisons can actually fail, three deliberate bugs were injected
into the Rust, the suite was re-run, and the bugs were then reverted:

| injected bug | detected by |
|--------------|-------------|
| `matrix_to_string`: `j < width - 1` → `j < width` (separator off-by-one) | 9 Phase B tests + 6 Phase D tests |
| `write_to_file`: `return EINVAL` → `return 21` | `err_12_write_null_content`, `err_24_write_both_null` |
| `driver`: ignore a non-zero `write_to_file` result | `err_23_driver_write_fails`, `err_28_driver_degenerate_dims` |

All three were caught; all three were reverted and the suite is green again.

## Update to the UB note

The 11-character-value heap overflow described above is not only reachable from
input text — `multiply_matrices` can also *produce* such values by wrapping an
`int` accumulation into `[-2147483648, -1000000000]`. Tests that stringify a
product therefore ask the C library itself whether the result is in-bounds and
skip the case if not (`product_is_string_safe` / `init_is_string_safe` in
`tests/common/mod.rs`). The exact boundary is exercised safely at `width = 1`,
where the C's allocation (`12*height + 1`) equals the requirement exactly:
`stress_to_string_11_char_values_width_1` compares `-2147483648` and
`2147483647` rendering through the 12-byte `snprintf` buffer.
