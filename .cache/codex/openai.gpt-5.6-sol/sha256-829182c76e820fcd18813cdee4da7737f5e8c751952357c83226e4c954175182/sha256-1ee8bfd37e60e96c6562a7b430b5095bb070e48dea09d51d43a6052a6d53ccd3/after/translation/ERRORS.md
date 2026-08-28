# Error Surface

The rows below come from the checks and terminal error return in
`../c_src/src/lib.c:7-20`. `22` and `34` are returned as literal integers.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `wcscat` | `dst == NULL` (including nonzero and oversized `numElem`) | return `22`; no memory is written | [x] |
| 2 | `wcscat` | `numElem == 0` with non-null `dst` (including null or non-null `src`) | return `22`; destination is unchanged | [x] |
| 3 | `wcscat` | `src == NULL`, with non-null `dst` and `numElem > 0` | write `dst[0] = 0`, then return `22` | [x] |
| 4 | `wcscat` | no NUL exists in `dst[0..numElem]`, so the destination scan consumes all capacity | write `dst[0] = 0`, then return `34`; `src` is not read | [x] |
| 5 | `wcscat` | a destination NUL exists, but no source NUL is copied before the remaining capacity is exhausted (source length is equal to or greater than remaining capacity) | write `dst[0] = 0`, then return `34`; bytes copied before exhaustion remain in `dst[1..]` | [x] |

There are no assertions, enums, explicit min/max constants, switches, or other
error-return statements in the C source.
