# ERRORS.md — Phase A error-surface table

Mechanically derived from `c_src/src/lib.c`. The complete set of conditional /
error constructs in the C translation unit is:

```
src/lib.c:11:    if (search == NULL) return path;            <- sentinel-return branch
src/lib.c:39:    if (!result) { fprintf(...); exit(30); }    <- the ONLY hard failure
src/lib.c:45:    if (outDirName[strlen(outDirName)-1] == ...) <- unguarded index (may read [-1])
```

There are **no** `assert`s, **no** error enums, **no** `RETURN_ERROR` macros,
**no** range checks, and **no** NULL-pointer validation of the parameters. The
header comment even states the function "never returns an error (it may abort()
in case of pb)". Consequently the error surface is small, and most "invalid
input" cases are *unchecked* — the C code faults or reads out of bounds, and the
Rust translation must do the identical thing.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| 1 | `extractFilename` | `separator` not present anywhere in `path` (`strrchr` returns `NULL`) | returns the `path` pointer itself, *unchanged* (pointer-identical to the argument) | `err_01_extract_separator_absent_returns_path` | [x] |
| 2 | `extractFilename` | `path` is `NULL` | no check: `strrchr(NULL, c)` → `SIGSEGV` (process killed by signal 11) | `err_02_extract_null_path_segv` | [x] |
| 3 | `extractFilename` | `separator == '\0'` (out-of-band value: NUL is never a legal path byte) | `strrchr` matches the terminating NUL, so it returns `path + strlen(path) + 1`, i.e. one byte **past** the end of the string | `err_03_extract_nul_separator` | [x] |
| 4 | `FIO_createFilename_fromOutDir` | `calloc` fails, forced with an enormous `suffixLen` (`1<<63`), so the request `strlen(out)+1+strlen(f)+suffixLen+1` cannot be satisfied | `fprintf(stderr, "zstd: FIO_createFilename_fromOutDir: %s", strerror(errno))` then `exit(30)` — process exit status **30** | `err_04_calloc_failure_exits_30` | [x] |
| 5 | `FIO_createFilename_fromOutDir` | `outDirName` is the empty string `""` → `outDirName[strlen(outDirName)-1]` == `outDirName[-1]`, an out-of-bounds read one byte *before* the buffer | no check; the branch taken depends on whatever byte precedes the buffer. Tested deterministically by handing both libraries the *same* pointer into a buffer whose preceding byte we control: preceding byte `'/'` → "already ends in separator" branch; any other byte → separator-inserting branch | `err_05_empty_outdir_oob_read_both_branches` | [x] |
| 6 | `FIO_createFilename_fromOutDir` | `path` is `NULL` | no check: reaches `strrchr(NULL, '/')` → `SIGSEGV` | `err_06_fio_null_path_segv` | [x] |
| 7 | `FIO_createFilename_fromOutDir` | `outDirName` is `NULL` | no check: reaches `strlen(NULL)` → `SIGSEGV` | `err_07_fio_null_outdir_segv` | [x] |
| 8 | `FIO_createFilename_fromOutDir` | `suffixLen == SIZE_MAX` — the size expression *wraps*: `a+1+b+SIZE_MAX+1 == a+b+1 (mod 2^64)`, so `calloc` **succeeds** with a tiny buffer | no check, no error: allocation of `a+b+1` bytes, both memcpy branches still fit exactly, and a normal pointer is returned. Note that in the separator-inserting branch the writes fill the buffer *exactly*, so no NUL terminator is written and `strlen` on the result reads out of bounds — the test compares the whole allocation byte-for-byte and skips the `strlen` cross-check for that branch | `err_08_suffixlen_size_max_wraps` | [x] |
| 9 | `FIO_createFilename_fromOutDir` | `suffixLen == SIZE_MAX - out_dir_len - filename_len - 1` (size expression wraps to exactly `0`) | no check: `calloc(1, 0)` returns a unique non-NULL 0-byte block, and the following `memcpy`s overflow it. Both implementations perform the identical overflow; asserted only for equal NULL-ness of the return value to avoid corrupting the test heap further | `err_09_suffixlen_wraps_to_zero` | [x] |

## Generic FFI boundary cases also covered (Phase C)

| case | where | note | status |
|------|-------|------|--------|
| null pointers | rows 2, 6, 7 | both die with the same signal | [x] |
| zero length input | `path == ""`, `outDirName == ""` | rows 5 and Phase-B rows 1–4 | [x] |
| oversized length | rows 4, 8, 9 (`suffixLen` = `1<<63`, `SIZE_MAX`, wrap-to-0) | [x] |
| one past a valid range | `separator` = `0x00`, `0x7F`, `0x80`, `0xFF` (full `char` domain incl. the sign-extension boundary) | `err_10_extract_full_char_domain` | [x] |
| out-of-range "enum" value across FFI | this API has **no enums**; the nearest analogue is `char separator`, which C widens to `int` at the `strrchr` call. Passed as raw `i8`/`u8` over the full `0x00..=0xFF` domain, including the negative (sign-extended) half that has no valid path-separator meaning | `err_10_extract_full_char_domain`, `err_11_extract_separator_int_widening` | [x] |

## Notes on how these rows are asserted

* The three `SIGSEGV` rows and the `exit(30)` row cannot be observed in-process,
  so they are driven in child processes: the test binary re-executes itself with
  `--exact <helper>` and `DIFFTEST_CHILD_IMPL=c|rust`, and the two terminations
  (exit code *and* signal) must be identical. `err_04b` additionally compares the
  bytes the two implementations write to `stderr` on the allocation-failure path.
* Row 5 makes an out-of-bounds read deterministic by handing both libraries the
  *same* pointer into a buffer whose preceding byte the test controls, so both
  branches of the unguarded `outDirName[strlen(outDirName)-1]` test are reachable
  and reproducible.
* The exact `calloc` request (`nmemb` and `size`, including the wrapping cases)
  is compared in `tests/phase_e_alloc.rs` via an `LD_PRELOAD` `calloc`
  interposer, because `malloc_usable_size` reports the reused chunk's capacity
  rather than the requested size and is therefore not a reliable oracle.
