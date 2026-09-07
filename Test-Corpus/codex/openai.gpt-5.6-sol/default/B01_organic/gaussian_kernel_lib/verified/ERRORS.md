# Error surface

Mechanical searches covered `RETURN_ERROR`, negative/sentinel returns,
`return NULL`, assertions, explicit `if`/`switch` validation, null checks,
range checks, enums, and min/max constants in `../c_src/include` and
`../c_src/src`.

The public function returns `void`. The C source contains no rejection or
validation path, so the required rejection table has zero rows:

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

## Required generic FFI boundaries

These are defined-behavior boundary cases, not C rejection paths. Cases that
would make C dereference an invalid pointer are undefined behavior and have no
C result that Rust can be required to match.

| # | function | boundary condition | expected C result | status |
|---|----------|--------------------|-------------------|--------|
| G1 | `gaussian_kernel` | null `dest`, `size <= -2` | returns normally without dereferencing `dest` | [x] |
| G2 | `gaussian_kernel` | `size == 0`, storage for one float supplied | writes one unnormalized center coefficient | [x] |
| G3 | `gaussian_kernel` | oversized but allocated positive even and odd lengths | completes; even size writes one extra coefficient | [x] |
| G4 | `gaussian_kernel` | enum one-past-range | not applicable: public API has no enum parameter | [x] |
| G5 | `gaussian_kernel` | one past documented valid range | not applicable: header documents no valid range | [x] |
