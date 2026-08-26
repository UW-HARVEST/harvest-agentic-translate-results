# Error Surface

Mechanical searches covered `return`, `NULL`, `assert`, comparisons, preprocessor
conditionals, allocation, and error-like calls across all of `c_src/src/main.c`.
There are no rejection branches, error-return macros, assertions, null checks,
range checks, enum inputs, pointer inputs, length inputs, or min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | covered |
|---|----------|----------------------------------------------|-------------------|---------|

`scanf("%d", &x)` conversion failure is not an error return from this library:
`main` ignores the conversion count, retains the initialized value `x = 0`,
runs normally, and returns `0`. That behavior is a valid configuration in
`CONFIGS.md`.

Generic FFI boundaries are not applicable: `run` accepts one by-value `int`,
and `main` accepts no declared pointer, length, or enum parameters.

Phase C status: [x] complete; the mechanically derived rejection table has zero
rows.
