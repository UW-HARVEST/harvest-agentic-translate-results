# Error Surface

Mechanical source scan covered `return -1`, `return NULL`, uppercase
error-return values, `RETURN_ERROR`, `assert`, `if`, `switch`, null checks,
range checks, and min/max constants. The C source contains none of these:
`main` always returns `0` after delegating parsing and arithmetic to libc.

The API has no pointer, length, enum, or documented-range parameters, so the
generic FFI null/length/enum cases do not apply.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `main` | second successfully scanned integer is `0` | process receives `SIGFPE` from libc `div`; no return value | [x] |

`INT_MIN / -1` is excluded: C signed division overflow is undefined behavior,
not a defined rejection result.
