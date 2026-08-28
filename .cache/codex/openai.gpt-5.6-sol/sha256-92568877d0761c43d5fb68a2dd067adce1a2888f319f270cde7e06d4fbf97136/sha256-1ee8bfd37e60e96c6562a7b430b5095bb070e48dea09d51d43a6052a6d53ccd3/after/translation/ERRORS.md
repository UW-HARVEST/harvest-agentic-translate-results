# Error Surface

Mechanical searches covered `return`, `RETURN_ERROR`, `ERROR`, `assert`,
`NULL`, `if`, `switch`, `case`, and min/max or relational checks in
`include/lib.h` and `src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | Tested |
|---|----------|----------------------------------------------|-------------------|--------|

There are no explicit rejection branches, error returns, assertions, range
checks, null checks, enums, or documented min/max constraints in the C API.
`premultiply` returns `void`.

Generic FFI boundaries:

| # | function | boundary | expected C result | Tested |
|---|----------|----------|-------------------|--------|
| G1 | `premultiply` | `img == NULL` | process terminates with `SIGSEGV` on the test platform | [x] |
| G2 | `premultiply` | `img->pix == NULL` and the computed byte extent is positive | process terminates with `SIGSEGV` on the test platform | [x] |
| G3 | `premultiply` | zero width or zero height, with a null pixel pointer | returns normally without dereferencing `pix` | [x] |
| G4 | `premultiply` | oversized width `w == INT_MAX`, with zero height and a null pixel pointer | returns normally without dereferencing `pix` | [x] |

Because `sizeof(cp_pixel_t)` has unsigned type, the width multiplication is
unsigned and its conversion to `int` is implementation-defined when the value
does not fit; G4 verifies the actual GCC/platform result. Inputs for which the
later signed `stride * h` multiplication overflows have undefined behavior
rather than a defined rejection. There are no enum parameters or documented
valid ranges with a defined one-past result.
