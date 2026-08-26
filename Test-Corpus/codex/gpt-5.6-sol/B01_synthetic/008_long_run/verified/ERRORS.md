# Error-Surface Table

Mechanically derived from every `if`, error return, `errno` check, pointer use,
and range constant in `c_src/src/main.c`. Rows 1-4 are explicit C rejection
conditions. Rows 5-6 are the generic null-pointer FFI boundaries; C does not
reject them and instead terminates by signal, which Rust must match.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `main` | `argc != 2`, with readable `argv[0]` | return `1`; stderr is `Usage: <argv[0]> <seed>\n` | [x] |
| 2 | `main` | `argc == 2` and `*endptr != '\0'` after `strtoul`, e.g. seed `1x` or `+` | return `1`; stderr is `Invalid seed: '<seed>'\n` | [x] |
| 3 | `main` | `argc == 2` and `errno != 0` after `strtoul`, e.g. decimal greater than `ULONG_MAX` | return `1`; stderr is `Invalid seed: '<seed>'\n` | [x] |
| 4 | `main` | `argc == 2` and `temp_seed > UINT_MAX`, e.g. `4294967296` on this target | return `1`; stderr is `Invalid seed: '<seed>'\n` | [x] |
| 5 | `main` | `argc == 2` and `argv == NULL` | process receives `SIGSEGV` while reading `argv[1]` | [x] |
| 6 | `main` | `argc == 2`, readable `argv`, and `argv[1] == NULL` | process receives `SIGSEGV` in `strtoul` | [x] |

There are no asserts, error macros, error enums, explicit null checks,
length-taking APIs, or enum parameters in the C source. Therefore zero,
oversized, and out-of-range enum boundary cases do not apply to any public API.
