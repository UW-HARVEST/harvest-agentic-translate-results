# Error Surface

Mechanical source scan:

```text
rg -n 'RETURN_ERROR|return\s+-1|return\s+NULL|assert\s*\(|if\s*\(|NULL|MIN|MAX|exit\s*\(|calloc' c_src/include c_src/src
```

There are no error-return macros, `-1`/`NULL` error returns, assertions, enum
validation branches, explicit argument-null checks, or min/max range checks.
The sole explicit rejection branch is allocation failure:

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 [x] | `FIO_createFilename_fromOutDir` | `calloc(1, strlen(outDirName) + 1 + strlen(filenameStart) + suffixLen + 1)` returns `NULL` | Write `zstd: FIO_createFilename_fromOutDir: ` followed by `strerror(errno)` to `stderr` (no newline), then terminate via `exit(30)` |

Mandatory generic FFI boundary cases, although they are not explicit C
rejection branches, are also tested: null `path`, null `outDirName`, zero
`suffixLen`, and an oversized `suffixLen`. There are no enum parameters or
documented numeric ranges in this API.

