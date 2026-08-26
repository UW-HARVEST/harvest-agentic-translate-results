# Error Surface

Mechanical source scan covered `return -1`, `return NULL`, `RETURN_ERROR`,
assertions, null/range checks, and min/max constants. The source contains no
error return, assertion, enum, length, or numeric range check. Its only
explicit input rejection is the pointer guard below.

| # | function | trigger (the exact invalid input/condition) | expected C result | Tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `printLine` | `line == NULL` | Return `void` without writing output | [x] |

`main` ignores `scanf`'s result. Conversion failure and EOF are therefore
valid observed states rather than errors: `x` remains zero, `bad()` runs, and
`main` returns 0.
