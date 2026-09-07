# Error and rejection surface

Mechanical source search covered `return -1`, `return NULL`, `RETURN_ERROR`,
`assert`, all `return 0` sites, null checks, enum defaults, count switches,
range comparisons, and the constants/array bounds used by the implementation.
This C file has no `assert`, `RETURN_ERROR`, `return -1`, `return NULL`, or
error enum. Required-pointer misuse is undefined behavior, not a C rejection.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---:|---|---|---|
| 1 | `c2Collided` | `typeA` is not 0, 1, or 2 | [x] returns `0` without dereferencing either shape |
| 2 | `c2Collided` | `typeA == C2_TYPE_CIRCLE`, `typeB` is not 0, 1, or 2 | [x] returns `0` |
| 3 | `c2Collided` | `typeA == C2_TYPE_AABB`, `typeB` is not 0, 1, or 2 | [x] returns `0` |
| 4 | `c2Collided` | `typeA == C2_TYPE_CAPSULE`, `typeB` is not 0, 1, or 2 | [x] returns `0` |
| 5 | `c2MakeProxy` | `type` is not 0, 1, or 2 | [x] no switch arm executes; output proxy bytes remain unchanged |
| 6 | `c2GJKSimplexMetric` | `s->count` is outside 1..=3 | [x] default arm returns `0.0f` |
| 7 | `c2D` | `s->count` is outside 1..=2 | [x] default arm returns vector `(0.0f, 0.0f)` |
| 8 | `c2Witness` | `s->count` is outside 1..=3 | [x] default arm writes `(0.0f, 0.0f)` to both outputs |
| 9 | `c2L` | `s->count` is outside 1..=2 | [x] default arm returns vector `(0.0f, 0.0f)` |
| 10 | `c2Support` | `count == 0` with `verts` still pointing to one readable element | [x] loop is skipped and `0` is returned |

The required non-null pointers (`c2BBVerts`, valid-type `c2MakeProxy`, simplex
helpers, and valid-type `c2Collided`) are dereferenced without checks in C.
Passing null to those positions is C undefined behavior and has no stable
error code/sentinel to reproduce. The nullable `c2GJK` pointers are valid
configurations and are enumerated in `CONFIGS.md`.

