# Error and invalid-input surface

Mechanical search covered `return`, `assert`, `if`, `switch`, enum defaults,
pointer checks, count checks, and numeric limits in `c_src/src/lib.c`.
There are no `assert`, `RETURN_ERROR`, `return -1`, error enums, or documented
min/max input checks. The deterministic rejection/default branches and safe
generic boundaries are below.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `c2MakeProxy` | `type` is `-1` or `3` (no `switch` case) | returns `void`; destination proxy bytes remain unchanged | [x] |
| 2 | `c2GJKSimplexMetric` | `s->count == 0` | `0.0f` | [x] |
| 3 | `c2GJKSimplexMetric` | `s->count == 4` | `0.0f` | [x] |
| 4 | `c2D` | `s->count == 0` | vector `{0.0f, 0.0f}` | [x] |
| 5 | `c2D` | `s->count == 4` | vector `{0.0f, 0.0f}` | [x] |
| 6 | `c2Witness` | `s->count == 0` | writes `{0,0}` to both output vectors | [x] |
| 7 | `c2Witness` | `s->count == 4` | writes `{0,0}` to both output vectors | [x] |
| 8 | `c2L` | `s->count == 0` | vector `{0.0f, 0.0f}` | [x] |
| 9 | `c2L` | `s->count == 3` | vector `{0.0f, 0.0f}` | [x] |
| 10 | `c2L` | `s->count == 4` | vector `{0.0f, 0.0f}` | [x] |
| 11 | `c2Support` | `count == 0`, with `verts` still pointing to one readable element | reads `verts[0]`, performs no loop iterations, returns `0` | [x] |
| 12 | `c2Support` | `count == -1`, with `verts` still pointing to one readable element | reads `verts[0]`, performs no loop iterations, returns `0` | [x] |
| 13 | `c2Support` | `count == 9` (one past the proxy's eight-vertex capacity), with a real nine-element input array | accepts the count and returns the first maximum-dot index | [x] |
| 14 | `c2Collided` | `typeA == C2_TYPE_CIRCLE`, `typeB` is `-1` or `3` | `0` without dereferencing `A` or `B` | [x] |
| 15 | `c2Collided` | `typeA == C2_TYPE_AABB`, `typeB` is `-1` or `3` | `0` without dereferencing `A` or `B` | [x] |
| 16 | `c2Collided` | `typeA == C2_TYPE_CAPSULE`, `typeB` is `-1` or `3` | `0` without dereferencing `A` or `B` | [x] |
| 17 | `c2Collided` | `typeA` is `-1` or `3` | `0` without dereferencing `A` or `B` | [x] |
| 18 | `c2Div` | divisor is `+0.0f` or `-0.0f` | no rejection; IEEE-754 infinities/NaNs from `a * (1/b)` | [x] |
| 19 | `c2Norm` | input vector is `{0.0f, 0.0f}` | no rejection; IEEE-754 NaN components | [x] |
| 20 | `c2CircletoCapsule` | capsule has `a == b` (zero-length segment) | no rejection; follows the endpoint-B branch and returns the exact C integer | [x] |
| 21 | `c2GJK` | `ax_ptr`, `bx_ptr`, `outA`, `outB`, `iterations`, and `cache` are all null while both shape pointers/types are valid | null transforms mean identity; null outputs/cache are skipped; returns distance | [x] |
| 22 | `c2GJK` | `use_radius` is `-1` or `2` | any nonzero value enables radius handling; returns the same result as `1` | [x] |

## Undefined behavior, not rejection

The C code does not define a result for null mandatory pointers (`shape`,
`proxy`, simplex, vertex, or collision-shape pointers when a valid enum causes
dereference), invalid `c2GJK` shape enums, malformed cache counts/indices,
allocation failure, or invalid `ptr_from_parts`/`omni_collide` enums.
`ptr_from_parts` falls off a non-void function for an unknown enum. These cases
cannot have a byte-identical deterministic result and are not represented as
passing rejection rows.
