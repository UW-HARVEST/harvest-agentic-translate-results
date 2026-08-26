# Error-Surface Table

The first three rows come from the three explicit rejection branches in
`c_src/src/main.c`. Rows 4-7 record the generic null-pointer FFI boundaries
required by the verification protocol. The C source does not reject those
nulls; it dereferences them, so parity means observing the same process signal
in an isolated child process.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `main` | `argc != 3` | Prints `Error: should only be two (integer) arguments!\n`; returns `1` | [x] |
| 2 | `main` | `argc == 3` and `strtol(argv[1], &end, 10)` sets `end == argv[1]` | Prints `Error: first argument must be an integer!\n`; returns `1` | [x] |
| 3 | `main` | First argument parses, but `strtol(argv[2], &end, 10)` sets `end == argv[2]` | Prints `Error: second argument must be an integer!\n`; returns `1` | [x] |
| 4 | `static_alias` | `outer == NULL` | No C rejection; dereference terminates an isolated process with `SIGSEGV` | [x] |
| 5 | `main` | `argc == 3` and `argv == NULL` | No C rejection; `argv[1]` access terminates an isolated process with `SIGSEGV` | [x] |
| 6 | `main` | `argc == 3`, `argv != NULL`, and `argv[1] == NULL` | No C rejection; `strtol(NULL, ...)` terminates an isolated process with `SIGSEGV` | [x] |
| 7 | `main` | `argc == 3`, first argument parses, and `argv[2] == NULL` | No C rejection; `strtol(NULL, ...)` terminates an isolated process with `SIGSEGV` | [x] |

No assertions, error enums, explicit numeric range checks, min/max constants,
length parameters, or enum-typed FFI inputs exist in the C source. `strtol`
overflow is not rejected: C ignores `errno`, converts the returned `long` to
`int`, and continues. It is therefore covered as a valid configuration.
