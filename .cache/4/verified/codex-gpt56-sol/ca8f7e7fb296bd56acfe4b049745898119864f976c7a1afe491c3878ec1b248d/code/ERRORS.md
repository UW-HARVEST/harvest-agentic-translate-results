# Error Surface

This table is derived from every rejection branch in `c_src/src/lib.c`.
There are no error enums, assertions, range checks, or error return
sentinels. The sole explicit failure path terminates the process.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `FIO_createFilename_fromOutDir` | `calloc(1, strlen(outDirName) + 1 + strlen(filenameStart) + suffixLen + 1)` returns `NULL` | Write `zstd: FIO_createFilename_fromOutDir: ` followed by `strerror(errno)` to `stderr`, then `exit(30)` | [x] |

## Generic FFI Boundaries

The C source does not reject these inputs. Null pointers and the
zero-length `outDirName` indexing expression have no portable C result, so
they are tested in subprocesses or with controlled guard storage. There are
no enum parameters or documented numeric ranges in this API.

| # | function | boundary | C source behavior | [ ] |
|---|----------|----------|-------------------|-----|
| B1 | `extractFilename` | `path == NULL` | Passes `NULL` to `strrchr`; compare isolated process termination | [x] |
| B2 | `FIO_createFilename_fromOutDir` | `path == NULL` | Passes `NULL` to `strrchr`; compare isolated process termination | [x] |
| B3 | `FIO_createFilename_fromOutDir` | `outDirName == NULL` | Passes `NULL` to `strlen`; compare isolated process termination | [x] |
| B4 | `FIO_createFilename_fromOutDir` | `outDirName` points at an empty string | Evaluates `outDirName[strlen(outDirName)-1]`; compare using controlled preceding guard storage | [x] |
| B5 | `FIO_createFilename_fromOutDir` | `suffixLen == 0` | Valid allocation with no reserved suffix bytes; covered by configuration rows | [x] |
| B6 | `FIO_createFilename_fromOutDir` | oversized, non-wrapping `suffixLen` | Makes `calloc` fail; must take error row 1 exactly | [x] |
