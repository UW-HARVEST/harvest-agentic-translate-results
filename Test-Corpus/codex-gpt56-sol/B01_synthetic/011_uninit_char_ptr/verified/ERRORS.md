# Error Surface

Mechanically derived from all `if`, null-check, return, assertion, range, and
error patterns in `c_src/src/main.c`. The source contains no assertions, error
enums, range checks, min/max constants, or error return values.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `printLine` | `line == NULL` | Return without calling `printf`; no bytes are written to stdout. | [x] |

`scanf` conversion failure and EOF are not rejection branches: `main` ignores
the return value, leaves its initialized `x` equal to zero, calls `bad`, and
returns zero. Those input shapes are therefore tracked in `CONFIGS.md`.

There are no length parameters or enum parameters in this API, so oversized
length and out-of-range enum boundary cases do not apply.
