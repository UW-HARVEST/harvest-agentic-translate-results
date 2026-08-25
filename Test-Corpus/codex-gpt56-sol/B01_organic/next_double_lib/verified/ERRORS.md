# Error Surface

The mechanical source scan covered `c_src/include/lib.h` and
`c_src/src/lib.c` for error-return statements/macros, assertions, null
checks, range checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | Status |
|---|----------|---------------------------------------------|-------------------|--------|

There are no explicit rejection paths in the C source. `next_double`
unconditionally dereferences `rnd`; a null pointer therefore invokes undefined
behavior rather than returning an error or sentinel. The API has no length,
range, or enum parameters.

Generic FFI-boundary coverage:

| # | function | boundary | expected C result | Status |
|---|----------|----------|-------------------|--------|
| G1 | `next_double` | `rnd == NULL` | process fault/abnormal termination (C undefined behavior) | [x] differential subprocess test |

Zero and maximum values of both state words are valid inputs and are covered
by `CONFIGS.md`. Length and enum boundary cases are not applicable to this API.
