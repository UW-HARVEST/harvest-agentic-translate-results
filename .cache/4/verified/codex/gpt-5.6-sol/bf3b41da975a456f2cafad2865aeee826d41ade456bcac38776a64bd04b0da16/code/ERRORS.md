# Error Surface

The complete source scan covered `return`, `RETURN_ERROR`, `NULL`, `assert`,
`abort`, explicit `if`/`switch` checks, range constants, and public declarations
in `c_src/src/lib.c` and `c_src/include/lib.h`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are no rows: `hdr_compare` has no error return, assertion, explicit range
check, null check, enum parameter, or length parameter. It returns the boolean
comparison result as integer `0` or `1`.

Phase C status: [x] complete (empty error surface).

## FFI Boundary Audit

- Each non-null pointer must make at least three readable bytes available whenever
  C reaches the corresponding dereference. Passing an unreadable pointer is
  undefined behavior in C, not a defined rejection, so it has no expected C
  result to compare.
- `h2 == NULL` is always undefined because C reads `h2[0]`.
- `h1 == NULL` is undefined when `h2` is valid. When `h2` is invalid, C
  short-circuits before reading `h1`; that defined case is covered by
  checked row 1 of `CONFIGS.md`.
- Zero and oversized lengths do not apply because the API has no length
  parameter. Out-of-range enums do not apply because the API has no enum
  parameter.
