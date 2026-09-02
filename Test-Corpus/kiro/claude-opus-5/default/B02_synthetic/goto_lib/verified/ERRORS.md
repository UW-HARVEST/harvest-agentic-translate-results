# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/goto.c`. Every `goto`, every early
`return` of an error value, every null check, every guard, and the one size
constant in the file is listed below. There are **no** `assert`s, no error
enums, no `RETURN_ERROR`-style macros, and no numeric range checks in this
library — the complete rejection surface is the six branches below plus the
buffer bound.

Grep basis:

```
$ grep -n 'goto\|return\|if (\|if(\|sizeof' c_src/src/goto.c
30:  if (x < 0) {
31:    goto error;
34:  printf("Processing: %d\n", x);
35:  return x * 2;
37: error:
38:  fprintf(stderr, "Error: negative input\n");
39:  return -1;
44:  if (!fp) {
45:    goto cleanup;
50:  while (fgets(buffer, sizeof(buffer), fp)) {
54:  if (ferror(fp)) {
55:    goto cleanup;
58:  return fp;
60: cleanup:
62:  if(fp) fclose(fp);
63:  return NULL;
67:  int res = forward_goto_example(num);
68:  if (res == -1) {
69:      return -1;
75:  if (out == NULL) {
76:      return -2;
```

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `forward_goto_example` | `x < 0` (`goto error`, line 30–31) | writes `Error: negative input\n` to **stderr**, nothing to stdout, returns `-1` | `err_e1_negative_x` | [x] |
| E2 | `forward_goto_example` | `x == INT_MIN` — boundary of E1, and the value whose negation overflows | same as E1: stderr message, returns `-1` | `err_e2_int_min` | [x] |
| E3 | `open_with_cleanup` | `fopen` returns NULL (`if (!fp)`, line 44) — nonexistent path | writes `Error: opening or processing file <name>\n` to stderr, **no** `fclose` (the `if(fp)` guard on line 62 is false), returns `NULL` | `err_e3_fopen_enoent` | [x] |
| E4 | `open_with_cleanup` | `fopen` fails for reasons other than ENOENT: empty filename `""` (ENOENT), unreadable file (EACCES), path with a non-directory component (ENOTDIR), oversized name (ENAMETOOLONG) | identical to E3 — the C never inspects `errno`, so every `fopen` failure takes the same branch and returns `NULL` | `err_e4_fopen_other_failures` | [x] |
| E5 | `open_with_cleanup` | `ferror(fp)` true after the read loop (line 54) — reached by `fopen`-ing a **directory** (succeeds on Linux, then `fgets` fails with `EISDIR`) | writes the same `Error: opening or processing file <name>\n` to stderr, **does** call `fclose(fp)` (guard true), returns `NULL` | `err_e5_ferror_directory` | [x] |
| E6 | `open_with_cleanup` | `filename == NULL` — passed straight into `fopen` and then into `fprintf`'s `%s` | `fopen(NULL,"r")` fails ⇒ E3 path; `%s` on a null pointer is glibc's `(null)`. Both `.so`s call the same libc, so both must print the identical bytes and return `NULL` | `err_e6_null_filename` | [x] |
| E7 | `driver` | `res == -1`, i.e. `num < 0` (line 68) | returns `-1` **without** touching `filename` — `open_with_cleanup` is never called, so no `fopen` and no `Goto output:` line | `err_e7_driver_negative_num` | [x] |
| E8 | `driver` | `out == NULL`, i.e. `num >= 0` but `open_with_cleanup` fails for any of E3/E4/E5/E6 | prints `Processing:`/`Goto output:` on stdout, the cleanup message on stderr, returns `-2` | `err_e8_driver_file_failure` | [x] |
| E9 | `open_with_cleanup` | buffer bound: `sizeof(buffer) == 100`, so `fgets` stores at most 99 bytes + NUL. A line of exactly 99/100/101+ bytes is split across iterations | not an error return, but the only size constant in the file; splitting must happen at identical offsets so the `printf("%s")` byte stream matches | `err_e9_buffer_boundary` | [x] |
| E10 | `open_with_cleanup` | content containing embedded `NUL` bytes: `printf("%s", buffer)` stops at the first NUL, silently dropping the rest of the chunk | output is *lossy* in exactly the same way for both; return value still non-NULL on success | `err_e10_embedded_nul` | [x] |

## Generic FFI boundary cases (required regardless of the table)

| case | covered by |
|------|-----------|
| null pointer argument | E6 (`open_with_cleanup(NULL)`, `driver(n, NULL)`) |
| zero length / empty input | `""` filename (E4), zero-byte file (Phase B row C6) |
| oversized length | `ENAMETOOLONG` filename (E4), file far larger than the 100-byte buffer (Phase B rows C10–C12) |
| one step past a valid range | `x = -1` and `x = 0` straddling the `x < 0` test (E1); `INT_MAX`, `INT_MAX/2`, `INT_MAX/2 + 1` straddling the signed-overflow point of `x * 2` (Phase B row C3) |
| out-of-range enum value across FFI | **not applicable** — the public API has no `enum` parameter. Both `int` parameters (`num`) already accept the full `int32` range, and the whole range is covered by the randomized sweep in Phase B row C1/C2 plus the explicit extremes. |
