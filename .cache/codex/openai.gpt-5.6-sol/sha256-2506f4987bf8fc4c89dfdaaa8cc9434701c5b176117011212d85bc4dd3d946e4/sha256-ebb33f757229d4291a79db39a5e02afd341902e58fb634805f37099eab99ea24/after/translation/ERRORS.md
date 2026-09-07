# Error surface

Mechanical scans covered `RETURN_ERROR`, `return -1`, `return NULL`, `assert`,
conditionals, null checks, range checks, enums, and min/max constants in
`../c_src/include` and `../c_src/src`.

The C source contains no explicit rejection branch, error return, assertion,
enum, documented numeric range, length parameter, or min/max constant.
`driver(..., iterations <= 0)` is accepted and performs zero iterations, so it
is a valid configuration listed in `CONFIGS.md`, not an error.

The mandatory generic FFI boundary that applies is:

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|-|
| 1 | `static_alias` | `outer == NULL`; line 30 dereferences it without a null check | process terminates with `SIGSEGV` on the test platform; no return value | [x] |

Generic boundary categories that do not exist in this API: lengths, enum
arguments, and documented bounded values.
