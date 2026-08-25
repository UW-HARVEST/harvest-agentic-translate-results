# Error Surface

Mechanically derived from every rejecting `if`/`return 1` path in
`c_src/src/main.c`. The program has no `assert`, error enum, explicit null
check, or separate length parameter.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| E01 | `main` | `argc == 1` at line 36 | return `1`; stdout is `Error: there should be one to three arguments passed:\n<string> [start] [stop]\n` | [x] |
| E02 | `main` | `argc > 4` at line 36 | return `1`; same two-line stdout as E01 | [x] |
| E03 | `main` | `argc >= 3` and `strtol(argv[2], &end, 10)` consumes no digits, so `end == argv[2]` at line 49 | return `1`; stdout is `Second argument must be an integer!` | [x] |
| E04 | `main` | parsed `start` satisfies `(size_t)start > strlen(argv[1])` at line 53; this includes `len + 1`, larger values, and negative `int` values after unsigned conversion | return `1`; stdout is `Error: start is off the end of the string!\n` | [x] |
| E05 | `main` | after parsing `argv[2]`, its saved `end` pointer equals `argv[3]` at line 63 (possible when argument pointers alias) | return `1`; stdout is `Third argument must be an integer!` | [x] |
| E06 | `main` | parsed `stop` satisfies `(size_t)stop > strlen(argv[1])` at line 68; this includes `len + 1`, larger values, and negative `int` values after unsigned conversion | return `1`; stdout is `Error: stop is off the end of the string!\n` | [x] |
| E07 | `main` | parsed `stop <= start` at line 73, including a nonnumeric third argument parsed by `strtol` as zero when E05 is false | return `1`; stdout is `Error: stop must come after start!\n` | [x] |

## Generic FFI Boundaries

The C source does not reject null pointers: null `argv`, `argv[1]`, `argv[2]`,
or `argv[3]` is undefined behavior and normally terminates with `SIGSEGV`.
Differential subprocess cases must verify that Rust has the same process-level
result. There are no enum parameters or explicit input-length parameters.

- [x] Null-pointer process behavior matches.
- [x] Zero-size strings, one-past indices, and overflowing decimal inputs match.
