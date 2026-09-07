# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection / error-return site in
`c_src/src/goto.c`. There are no `assert`s, no error enums, no min/max
constants and no `RETURN_ERROR`-style macros in this library; the complete set
of rejection sites is:

```
goto.c:30-32   if (x < 0) { goto error; }              -> forward_goto_example
goto.c:37-39   error: fprintf(stderr,...); return -1;  -> forward_goto_example
goto.c:44-46   if (!fp) { goto cleanup; }              -> open_with_cleanup
goto.c:53-55   if (ferror(fp)) { goto cleanup; }       -> open_with_cleanup
goto.c:59-62   cleanup: fprintf(stderr,...); ...; return NULL;
goto.c:67-68   if (res == -1) { return -1; }           -> driver
goto.c:74-75   if (out == NULL) { return -2; }         -> driver
```

`open_with_cleanup` has exactly two `goto cleanup` branches, so the table has
two rows for it (rows 4 and 5 split row-4's trigger by the two distinct
`fopen`-failure causes that a caller can actually produce, plus row 6 for the
`ferror` branch).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `forward_goto_example` | `x < 0`, ordinary negative (e.g. `-1`, `-42`) — takes `goto error` | returns `-1`; writes exactly `Error: negative input\n` to stderr; writes **nothing** to stdout (no `Processing:` line) | [x] |
| 2 | `forward_goto_example` | `x == INT_MIN` (extreme negative, one step past `-INT_MAX`) — still `x < 0` | returns `-1`; `Error: negative input\n` on stderr | [x] |
| 3 | `forward_goto_example` | `x == -1` specifically: the *valid* return value collides with the error sentinel path used by `driver` | returns `-1` (sentinel indistinguishable from error — replicate, do not "fix") | [x] |
| 4 | `open_with_cleanup` | `filename` names a nonexistent path (`fopen` → `NULL`, `ENOENT`) — first `goto cleanup`, `fp == NULL` so **no `fclose`** | returns `NULL`; stderr gets `Error: opening or processing file <path>\n` | [x] |
| 5 | `open_with_cleanup` | `filename` is a NULL pointer. glibc's `fopen` passes it to `openat(2)` unread, so it returns `NULL`/`EFAULT` (verified, no crash); `fprintf("%s", NULL)` then prints glibc's `(null)` | returns `NULL`; stderr gets `Error: opening or processing file (null)\n` | [x] |
| 6 | `open_with_cleanup` | `filename` is a **directory**: `fopen` *succeeds* on Linux, then `fgets` fails with `EISDIR` and sets `ferror` → second `goto cleanup` with `fp != NULL`, so `fclose(fp)` **is** called | returns `NULL`; stderr gets `Error: opening or processing file <dir>\n`; stdout unchanged | [x] |
| 7 | `open_with_cleanup` | `filename` is an unreadable file (mode `0000`, `fopen` → `NULL`, `EACCES`) — first `goto cleanup` | returns `NULL`; stderr message | [x] |
| 8 | `open_with_cleanup` | `filename` is the empty string `""` (`fopen("")` → `NULL`, `ENOENT`) | returns `NULL`; stderr `Error: opening or processing file \n` | [x] |
| 9 | `driver` | `forward_goto_example(num)` returned `-1`, i.e. `num < 0` — early `return -1` **before** `open_with_cleanup` is ever called (so no file is touched and no stderr file message appears) | returns `-1`; stderr only `Error: negative input\n`; stdout empty | [x] |
| 10 | `driver` | `num >= 0` but `open_with_cleanup(filename)` returned `NULL` (any of rows 4–8) — `return -2` | returns `-2`; stdout has `Processing: <num>\n` + `Goto output: <2*num>\n`; stderr has the file message | [x] |
| 11 | `driver` | `num < 0` **and** `filename` invalid at the same time: the `num` check wins | returns `-1` (not `-2`); file never opened | [x] |
| 12 | `driver` | `filename` is NULL pointer with `num >= 0` (null-pointer boundary across FFI) | returns `-2`; stderr `...file (null)\n` | [x] |
| 13 | `driver` | `num == INT_MIN` (extreme / one-step-past boundary) | returns `-1` | [x] |

## Generic FFI boundary cases also covered by the tests

* Null pointers: rows 5 and 12 (`filename == NULL`).
* Zero-length input: row 8 (`filename == ""`), plus a zero-byte **file** which is
  a *valid* path (see `CONFIGS.md` row C2 — `fgets` returns `NULL` immediately,
  `ferror` is 0, so the file handle is returned successfully).
* Oversized length: a path longer than `PATH_MAX` (`ENAMETOOLONG`) — behaves as
  row 4.
* One step past a documented valid range: `INT_MIN` / `INT_MAX` for `num`
  (rows 2, 13 and `CONFIGS.md` row A5, which exercises the `x * 2` signed
  overflow).
* Out-of-range enum values: **not applicable** — this API declares no `enum`
  type and takes no enum-typed parameter. Both parameters are an unrestricted
  `int` and a `const char*`, and the full `int` range is swept (including
  `INT_MIN`/`INT_MAX`) by the randomized Phase B/C tests.
