# Error surface

Mechanical searches covered `return -1`, `return NULL`, `RETURN_ERROR`,
assertions, null checks, range checks, enum switches, and numeric bounds in
`c_src/src/lib.c` and `c_src/include/lib.h`.

The C API has no error-return macro, error enum, assertion, or explicit
rejection return. It does have one deterministic response to an invalid enum:
the `c2MakeProxy` switch has no `default`, so an unknown type leaves the output
object untouched.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `c2MakeProxy` | `type` is any integer other than `0`, `1`, or `2`; `shape` may be null because no case dereferences it | returns `void` and leaves every byte of `*p` unchanged [x] |

## Non-rejecting boundaries

These are valid optional-pointer modes and therefore appear in `CONFIGS.md`,
not as errors: null `ax_ptr`, `bx_ptr`, `outA`, `outB`, `iterations`, and
`cache` in `c2GJK`.

The remaining invalid pointer/count inputs are not rejected by C and have no
defined C result to compare:

- Null required pointers in `c2BBVerts`, valid-type `c2MakeProxy`,
  `c2GJKSimplexMetric`, `c22`, `c23`, `c2D`, `c2Support`, `c2Witness`, and
  `c2L` are dereferenced.
- `c2Support` always reads `verts[0]`; `count <= 0` is not treated as an empty
  input and therefore still requires one readable element. A count larger than
  the actual backing array reads out of bounds.
- Invalid `c2GJK` shape enums leave a local `c2Proxy` uninitialized and then
  consume it. Invalid cache counts or indices access fixed arrays out of
  bounds.

Those cases are C undefined behavior rather than an error/sentinel surface.
