# Error Surface

Mechanical searches covered `return -1`, `return NULL`, `RETURN_ERROR`,
`assert`, explicit null/range checks, enums, and constants in `c_src/src/lib.c`
and `c_src/include/lib.h`.

The C source contains no error-return macro, error code, error enum, assertion,
or branch that rejects an input. Therefore the explicit rejection table has
zero rows:

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

## Defined Boundary Behavior

These are not C rejections, but they are defined edge cases required by the
generic FFI boundary audit. Required-pointer violations and out-of-bounds reads
are listed separately because the C source gives them no defined result.

| # | function | boundary input/condition | exact C result | Status |
|---|----------|--------------------------|----------------|--------|
| B1 | `c2MakeProxy` | `type` is any integer other than 0, 1, or 2; `shape` may be null because no case dereferences it | No `switch` case runs; `*p` remains byte-for-byte unchanged | [x] |
| B2 | `c2GJKSimplexMetric` | `count` is 0, negative, or greater than 3 | Returns positive zero via the `default`/`case 1` arm | [x] |
| B3 | `c2D` | `count` is 0, negative, or greater than 3 | Returns `{+0.0, +0.0}` via `default` | [x] |
| B4 | `c2Witness` | `count` is 0, negative, or greater than 3 with valid `a` and `b` pointers | Writes `{+0.0, +0.0}` to both outputs | [x] |
| B5 | `c2L` | `count` is 0, negative, or greater than 2 | Returns `{+0.0, +0.0}` via `default` | [x] |
| B6 | `c2Support` | `count` is 0 or negative and `verts` points to at least one element | Reads `verts[0]`, skips the loop, and returns index 0 | [x] |
| B7 | `c2Div` | divisor is `+0.0` or `-0.0` | Uses IEEE-754 division; components become signed infinity or NaN according to each numerator | [x] |
| B8 | `c2Norm` | input is `{+0.0, +0.0}` | Divides zero components by zero length and returns two NaNs | [x] |
| B9 | `c2GJK` | `ax_ptr` and/or `bx_ptr` is null | Substitutes `c2xIdentity()` independently for each null transform | [x] |
| B10 | `c2GJK` | any of `outA`, `outB`, or `iterations` is null | Completes normally and omits only the corresponding write | [x] |
| B11 | `c2GJK` | `cache` is null | Starts from vertex pair 0 and omits cache read/write | [x] |
| B12 | `c2GJK` | non-null cache has `count == 0` | Ignores cached fields, starts from vertex pair 0, then writes the resulting cache | [x] |
| B13 | `c2GJK` | `use_radius` is any nonzero integer, including negative values | Takes the radius-adjustment branch | [x] |
| B14 | `gjk` | `reverse` is any nonzero `char`, including negative values | Uses capsule as shape A and AABB as shape B | [x] |

## Undefined C Inputs

The following generic invalid inputs are outside the C abstract machine's
defined behavior. They have no C error code or sentinel to compare:

| API family | input | reason |
|------------|-------|--------|
| Pointer-taking helpers | null required `s`, `out`, `bb`, `p`, `a`, or `b` pointer | unconditional null dereference |
| `c2MakeProxy` | valid type with null `shape` or null `p` | unconditional null dereference in the selected case |
| `c2Support` | null/empty `verts`, or `count` greater than the allocated array | unconditional `verts[0]` read or later out-of-bounds read |
| `c2GJK` | null required shape pointer | selected proxy case dereferences it |
| `c2GJK` | out-of-range `typeA` or `typeB` | corresponding local `c2Proxy` remains uninitialized and is then read |
| `c2GJK` | cache count outside 0 through 3, or cached indices outside the selected proxy vertex ranges | out-of-bounds stack/array access |

No deterministic differential assertion can be made for these cases without
inventing behavior absent from the C ground truth.
