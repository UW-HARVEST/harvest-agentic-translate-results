# Error Surface

Mechanical scans covered `RETURN_ERROR`, `return -1`, `return NULL`, `assert`,
conditionals, null checks, range constants, and enum declarations in all files
under `c_src/`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|

There are no rejection branches, error returns, assertions, pointers, lengths,
or enums in the public C API. `ldexp_q2` returns a `float` for every call. A
negative `exp_q2` makes the C expression shift by a negative count, which is
undefined by the C language and is therefore covered as an observable
configuration of the built reference `.so` in `CONFIGS.md`, not as a rejection.

Generic FFI error-boundary categories are inapplicable: the API has no pointer,
buffer, length, enum, or documented constrained range.

## Completion

- [x] Every error-surface row has a passing differential test (zero rows).
- [x] All applicable generic FFI boundaries have differential coverage.
