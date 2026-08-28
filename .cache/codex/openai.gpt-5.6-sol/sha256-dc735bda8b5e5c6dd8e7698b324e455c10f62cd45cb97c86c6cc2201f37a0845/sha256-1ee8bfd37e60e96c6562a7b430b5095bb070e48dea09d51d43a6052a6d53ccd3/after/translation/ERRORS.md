# Error Surface

The C source has no error-return macro, `assert`, `return -1`, `return NULL`,
or documented min/max rejection. The rows below are every explicit default
branch that rejects an invalid enum or invalid simplex count. Pointer arguments
that C dereferences are preconditions, not rejected inputs; passing null to
those paths is undefined behavior rather than an observable error result.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---:|----------|---------------------------------------------|-------------------|----------|
| 1 | `c2MakeProxy` | `type` is not 0, 1, or 2 | returns `void` without changing `*p` | [x] |
| 2 | `c2GJKSimplexMetric` | `s->count` is outside 1 through 3 | returns `0.0f` | [x] |
| 3 | `c2D` | `s->count` is outside 1 through 3 | returns vector `{0.0f, 0.0f}` | [x] |
| 4 | `c2Witness` | `s->count` is outside 1 through 3 | writes `{0.0f, 0.0f}` to both outputs | [x] |
| 5 | `c2L` | `s->count` is outside 1 through 3 | returns vector `{0.0f, 0.0f}` | [x] |
| 6 | `c2Collided` | `typeA == C2_TYPE_CIRCLE` and `typeB` is outside 0 through 2 | returns `0` without dereferencing `B` | [x] |
| 7 | `c2Collided` | `typeA == C2_TYPE_AABB` and `typeB` is outside 0 through 2 | returns `0` without dereferencing `B` | [x] |
| 8 | `c2Collided` | `typeA == C2_TYPE_CAPSULE` and `typeB` is outside 0 through 2 | returns `0` without dereferencing `B` | [x] |
| 9 | `c2Collided` | `typeA` is outside 0 through 2 (for any `typeB`) | returns `0` without dereferencing either shape | [x] |

Safe null-pointer modes explicitly implemented by C are valid configurations:
`c2GJK` accepts null transform, witness-output, iteration-output, and cache
pointers. A null required shape/struct/array pointer, or `c2Support` with zero
length, is undefined behavior in C and therefore has no rejection row.
