# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. Every rejection / error / sentinel
/ boundary the C code actually contains is listed below.

Exhaustive grep of the rejection sites in the C source:

```
$ grep -n -E "return|assert|exit|NULL|if " c_src/src/lib.c
11:    if (search == NULL) return path;      <- sentinel: separator not found
12:    return search+1;
39:    if (!result) {                        <- allocation failure
41:        exit(30);
45:    if (outDirName[strlen(outDirName)-1] == separator) {
```

There are **no** `assert`s, **no** error enums, **no** error-code returns and
**no** explicit range checks or min/max constants in this library. Its entire
error surface is the two sites above (rows 1 and 2). Rows 3-14 are the generic
C-API boundaries that Phase C additionally mandates (NULL pointers, zero and
oversized lengths, one-step-past-range values, and every possible value of the
`char separator` argument — the analogue of an out-of-range enum value, since a
C `char` parameter accepts any of the 256 byte values across the FFI boundary).

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|---------------------------------------------|-------------------|------|-----|
| 1  | `extractFilename` | `strrchr(path, separator) == NULL`, i.e. `separator` byte does not occur anywhere in `path` (incl. `path == ""`) | returns `path` unchanged (identical pointer, offset 0) | `err_01_extract_separator_absent_returns_path` | [x] |
| 2  | `FIO_createFilename_fromOutDir` | `calloc(1, strlen(outDirName)+1+strlen(filenameStart)+suffixLen+1)` returns `NULL` (forced with `suffixLen = SIZE_MAX/2`) | prints `zstd: FIO_createFilename_fromOutDir: <strerror(errno)>` to `stderr` (no newline) and `exit(30)` — process exit status 30 | `err_02_alloc_failure_exits_30` (subprocess, compares exit code **and** stderr bytes) | [x] |
| 3  | `extractFilename` | `separator == '\0'` — `strrchr` treats the terminating NUL as part of the string, so it is *found* | returns `path + strlen(path) + 1` (a one-past-the-end pointer), **not** `path` | `err_03_extract_nul_separator` | [x] |
| 4  | `extractFilename` | `path == ""` (zero length) with `separator == '\0'` | returns `path + 1` | `err_04_extract_empty_path_nul_separator` | [x] |
| 5  | `extractFilename` | out-of-range `char` value for `separator`: all 256 byte values `0x00..0xFF` (negative `c_char` values included), against paths that do and do not contain them | last-occurrence pointer, or `path` when absent; sign handling must match `strrchr`'s `(char)c` conversion | `err_05_extract_all_256_separator_values` | [x] |
| 6  | `extractFilename` | `path == NULL` | dereferences NULL → `SIGSEGV` (signal 11) | `err_06_extract_null_path_segv` (subprocess, compares terminating signal) | [x] |
| 7  | `FIO_createFilename_fromOutDir` | `path == NULL` | `strrchr(NULL, ...)` → `SIGSEGV` | `err_07_fio_null_path_segv` (subprocess) | [x] |
| 8  | `FIO_createFilename_fromOutDir` | `outDirName == NULL` | `strlen(NULL)` → `SIGSEGV` | `err_08_fio_null_outdir_segv` (subprocess) | [x] |
| 9  | `FIO_createFilename_fromOutDir` | `outDirName == ""` (zero length) → `strlen(outDirName)-1` wraps to `SIZE_MAX` and `outDirName[SIZE_MAX]` reads the byte **before** the buffer (out-of-bounds read present in the C) | branch is chosen by that out-of-bounds byte; must be read at the same address by both libraries. Verified with the preceding byte pinned to `'/'` (⇒ trailing-separator branch) and to `'X'` (⇒ separator-inserting branch) | `err_09_fio_empty_outdir_oob_read` | [x] |
| 10 | `FIO_createFilename_fromOutDir` | `suffixLen == 0` (zero length, minimum) | buffer of exactly `strlen(outDir)+1+strlen(file)+1` bytes, NUL-terminated | `err_10_fio_zero_suffixlen` | [x] |
| 11 | `FIO_createFilename_fromOutDir` | `suffixLen == SIZE_MAX` (one step past the largest usable value; `size_t` addition wraps) | wrapped size `strlen(outDir)+strlen(file)+1`; `calloc` succeeds, the payload fills the buffer exactly and the result is **not** NUL-terminated | `err_11_fio_suffixlen_size_max_wraps` | [x] |
| 12 | `FIO_createFilename_fromOutDir` | `suffixLen == SIZE_MAX - 1` and `SIZE_MAX - 2` (further wrapped sizes, still allocatable) | wrapped sizes; contents/termination must match byte-for-byte | `err_12_fio_suffixlen_near_size_max_wraps` | [x] |
| 13 | `FIO_createFilename_fromOutDir` | oversized-but-not-wrapping `suffixLen` (`SIZE_MAX/2`, `SIZE_MAX/4`, `1<<62`, `1<<48`) — `calloc` cannot satisfy it | `exit(30)` + identical `stderr` message (same path as row 2, several magnitudes) | `err_13_fio_oversized_suffixlen_exits_30` (subprocess) | [x] |
| 14 | `FIO_createFilename_fromOutDir` | `path == ""` and `path` ending in the separator ⇒ `filenameStart` is the empty string (zero-length component) | `outDir` + `'/'` (or nothing extra when `outDir` already ends in `'/'`) + NUL | `err_14_fio_empty_filename_component` | [x] |

## Extra boundary tests beyond the table

| test | what it covers |
|------|----------------|
| `err_15_extract_chained_interior_pointers` | feeding the interior / one-past-the-end pointer returned by `extractFilename` straight back into it (what the Windows branch of the C does, and what a chaining caller does), with separators `'/'`, `'.'` and `'\0'` |
| `err_16_fio_suffixlen_sweep` | `suffixLen` swept over `0..40` and `2^10, 2^12, 2^16, 2^20`, both branches of line 45 |

## Deliberately excluded (non-deterministic undefined behaviour)

`suffixLen` chosen so that the wrapped `calloc` size is **smaller** than the
`strlen(outDirName)+1+strlen(filenameStart)` bytes that are then written (e.g.
`suffixLen = SIZE_MAX - strlen(outDir) - strlen(file) - 1`, giving size `0`)
makes the C code overflow the heap block. The observable outcome depends on the
allocator's block layout at that moment, so it is not a differentially testable
result for either implementation; both libraries perform the identical
`calloc`/`memcpy` sequence with the identical sizes (which rows 11 and 12 pin
down exactly). Rows 11/12 cover the largest wrapped sizes that are still
well-defined (payload fits exactly).
