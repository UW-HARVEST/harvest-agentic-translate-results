# Error Surface

Mechanical source scan found no `assert`, error-return macro, error enum,
pointer parameter, null check, explicit range check, or min/max constant.
`driver` has no rejection path. The only input rejection occurs inside the
`scanf("%d", &x)` conversion used by `main`; C intentionally ignores its
return value after initializing `x` to zero.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `main` | `scanf` returns `EOF` because no input item is available before conversion (empty input) | `x` remains `0`; no stdout bytes; return `0` | [x] |
| 2 | `main` | `scanf` returns `0` because the first non-whitespace bytes do not match `%d` | `x` remains `0`; no stdout bytes; return `0` | [x] |

Generic FFI boundary audit: neither exported function accepts pointers,
lengths, or enums. Zero and negative `int` values are valid `driver` inputs
covered in `CONFIGS.md`; there is no documented numeric rejection range.
