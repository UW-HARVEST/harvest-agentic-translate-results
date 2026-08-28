# Error Surface

The C source has one explicit rejection. It has no error enums, error-return
macros, `assert` calls, range checks, or null checks.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `FIO_createFilename_fromOutDir` | `calloc(1, strlen(outDirName) + 1 + strlen(filenameStart) + suffixLen + 1)` returns `NULL` | writes `zstd: FIO_createFilename_fromOutDir: ` followed by `strerror(errno)` to `stderr`, then exits with status 30 | [x] |

## Generic FFI Boundaries

These are not explicit C rejections. They exercise the mandatory generic
boundaries in isolated subprocesses because the C implementation dereferences
the pointers and therefore terminates by signal for null inputs.

| # | function | boundary | expected C result | tested |
|---|----------|----------|-------------------|--------|
| G1 | `extractFilename` | `path == NULL` | process terminates by signal while `strrchr` reads `path` | [x] |
| G2 | `FIO_createFilename_fromOutDir` | `path == NULL` | process terminates by signal while extracting the filename | [x] |
| G3 | `FIO_createFilename_fromOutDir` | `outDirName == NULL` | process terminates by signal while evaluating `strlen(outDirName)` | [x] |
| G4 | `FIO_createFilename_fromOutDir` | empty `outDirName` placed at a guard-page boundary | process terminates by signal on `outDirName[strlen(outDirName)-1]` | [x] |
| G5 | `FIO_createFilename_fromOutDir` | `suffixLen == 0` | valid result; covered by every zero-suffix configuration row | [x] |
| G6 | `FIO_createFilename_fromOutDir` | oversized `suffixLen` whose allocation cannot be represented by the allocator | same explicit allocation-failure result as row 1 | [x] |

There are no enum parameters or documented numeric ranges, so out-of-range
enum and one-past-range cases do not apply.
