# Error Surface

Mechanical searches covered every C source file under `c_src/src/` for error
returns, `return -1`, `return NULL`, assertions, conditionals, switches, null
checks, range checks, error enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

There are no explicit rejection branches in the C source. In particular,
`main` ignores all four `scanf` return values, and `print_foo` dereferences its
argument without a null check. The generic FFI boundary checks therefore
compare the unchecked behavior: integer extrema, failed scans retaining the
zero-initialized fields, and a null `print_foo` pointer terminating by signal.
There are no lengths or enums in the public API.

