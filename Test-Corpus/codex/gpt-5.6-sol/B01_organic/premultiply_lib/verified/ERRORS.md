# Error Surface

Mechanical search covered `return`, `assert`, null checks, range checks,
preprocessor error macros, error constants/enums, and min/max constants in
`c_src/src/lib.c` and `c_src/include/lib.h`.

The C API returns `void` and contains no rejection branch, error return,
assertion, explicit null check, explicit range check, error enum, or min/max
constant. Therefore the C-derived error-surface table has zero rows:

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|

## Mandatory Generic Boundary Probes

These are required FFI boundaries rather than C rejection branches. A null
pointer used on an active path has undefined behavior in C; the differential
test compares the observable subprocess termination produced by the built C
and Rust shared libraries on this platform.

| # | function | boundary input | expected C result | tested |
|---|----------|----------------|-------------------|--------|
| G1 | `premultiply` | null `img` | process terminates while dereferencing `img` | [x] |
| G2 | `premultiply` | non-null image, null `pix`, positive loop bound | process terminates while reading `pix` | [x] |
| G3 | `premultiply` | non-null image, null `pix`, non-positive loop bound | returns without dereferencing `pix` | [x] |
| G4 | `premultiply` | zero dimensions | returns without modifying storage | [x] |
| G5 | `premultiply` | extreme `int` dimensions (`INT_MIN`/`INT_MAX`) | matches the built C ABI's narrowed-stride behavior; inactive bounds return unchanged, while `INT_MAX,-1` processes one pixel | [x] |

There are no enum parameters, documented numeric ranges, or separate length
parameters in the public API, so out-of-range-enum and one-past-range probes
are not applicable.
