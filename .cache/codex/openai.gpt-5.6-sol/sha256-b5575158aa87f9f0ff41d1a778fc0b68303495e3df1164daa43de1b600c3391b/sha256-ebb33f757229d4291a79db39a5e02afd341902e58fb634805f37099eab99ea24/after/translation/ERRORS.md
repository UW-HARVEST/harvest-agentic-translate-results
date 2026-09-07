# Error surface

The mechanical scan covered `c_src/include/lib.h` and `c_src/src/lib.c` for
error returns, `NULL`, assertions, enums, explicit range checks, and min/max
constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|

There are no explicit rejection or error paths in the C source. In particular,
`tool_basename` unconditionally passes its argument to `strrchr`; a null
pointer is outside the function's accepted C-string contract and is not
converted to an error code or sentinel.

Generic FFI boundary coverage is tracked separately in the differential tests:

- [x] zero-length C string: valid input, covered in `CONFIGS.md`;
- [x] null pointer: C and Rust behavior compared in isolated child processes because
  the C implementation dereferences it through libc rather than rejecting it;
- [x] oversized lengths: not applicable (the API has no length parameter);
- [x] enum values: not applicable (the API has no enum parameter);
- [x] one-past-range values: not applicable (the API documents no numeric range).

The applicable boundary tests pass under both the default build and
`--no-default-features`.
