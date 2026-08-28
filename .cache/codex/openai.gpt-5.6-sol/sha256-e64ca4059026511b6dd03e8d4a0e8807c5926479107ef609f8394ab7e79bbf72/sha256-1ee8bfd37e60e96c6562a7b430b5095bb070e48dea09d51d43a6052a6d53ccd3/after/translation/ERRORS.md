# Error Surface

Derived from every rejection branch in `../c_src/src/lib.c`. There are no
assertions, enums, error-return macros, null checks, or error sentinels.

| # | function | trigger (the exact invalid input/condition) | expected C result | covered |
|---|----------|---------------------------------------------|-------------------|---------|
| 1 | `bin2hex` | `bin_len >= SIZE_MAX / 2` | process aborts (`SIGABRT`) | [x] |
| 2 | `bin2hex` | `bin_len < SIZE_MAX / 2` and `hex_maxlen <= bin_len * 2` | process aborts (`SIGABRT`) | [x] |

Generic FFI boundary cases required by Phase C:

| # | function | trigger (the exact invalid input/condition) | expected C result | covered |
|---|----------|---------------------------------------------|-------------------|---------|
| G1 | `bin2hex` | `hex == NULL`, otherwise-valid lengths | invalid write terminates the child process (`SIGSEGV`) | [x] |
| G2 | `bin2hex` | `bin == NULL`, `bin_len > 0`, and sufficient output capacity | invalid read terminates the child process (`SIGSEGV`) | [x] |
| G3 | `bin2hex` | `bin == NULL`, `bin_len == 0`, non-null `hex`, and `hex_maxlen > 0` | accepted; returns `hex` and writes one NUL byte | [x] |
