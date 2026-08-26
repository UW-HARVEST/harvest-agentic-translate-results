# Error Surface

The first four rows are the complete set of explicit rejection returns in
`c_src/src/lib.c`. Rows 5-7 are the mandatory generic null-pointer boundaries:
the C function unconditionally passes each argument to `strlen` before any
branch, so these have no C return sentinel and must be compared in isolated
processes.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|-|
| 1 | `searchAndReplace` | First match starts after byte 0 and the initial `malloc(inx_start + 1)` returns `NULL` (lines 32-36) | `NULL` | [x] |
| 2 | `searchAndReplace` | The `realloc` used to append a replacement returns `NULL` (lines 42-47) | `NULL` | [x] |
| 3 | `searchAndReplace` | A later match has a nonempty gap and the `realloc` used to append that gap returns `NULL` (lines 55-64) | `NULL` | [x] |
| 4 | `searchAndReplace` | Bytes remain after the last match and the final suffix `realloc` returns `NULL` (lines 77-82) | `NULL` | [x] |
| 5 | `searchAndReplace` | `orig == NULL`; the first `strlen(orig)` dereferences it (line 11) | no return; process receives `SIGSEGV` on the test platform | [x] |
| 6 | `searchAndReplace` | `search == NULL`; `strlen(search)` dereferences it (line 12) | no return; process receives `SIGSEGV` on the test platform | [x] |
| 7 | `searchAndReplace` | `value == NULL`; `strlen(value)` dereferences it (line 13) | no return; process receives `SIGSEGV` on the test platform | [x] |

There are no length parameters, enums, asserts, range checks, min/max constants,
error enums, or `return -1` branches in the public API. Zero-length `orig` and
`value` are valid shapes covered in `CONFIGS.md`; zero-length `search` is also
listed there because C enters a nonterminating loop rather than rejecting it.
