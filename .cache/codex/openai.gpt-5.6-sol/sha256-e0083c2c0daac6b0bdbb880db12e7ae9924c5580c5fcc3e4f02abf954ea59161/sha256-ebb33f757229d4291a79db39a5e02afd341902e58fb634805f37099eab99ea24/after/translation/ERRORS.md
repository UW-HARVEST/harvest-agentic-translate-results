# Error and rejection surface

The C source contains no `assert`, `RETURN_ERROR`, `return -1`, `return NULL`,
or public error enum. The rows below are every explicit default/range rejection
that has defined observable behavior. Pointer arguments that C unconditionally
dereferences are not rejection paths: null there is C undefined behavior.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `c2MakeProxy` | `type` is not `C2_TYPE_CIRCLE` (0), `C2_TYPE_AABB` (1), or `C2_TYPE_CAPSULE` (2) | switch executes no case; output proxy bytes remain unchanged | [x] |
| 2 | `c2GJKSimplexMetric` | `s->count` is any value other than 2 or 3 (including 0, 1, negative, and greater than 3) | returns `0.0f` | [x] |
| 3 | `c2D` | `s->count` is any value other than 1 or 2 (including 0, 3, negative, and greater than 3) | returns `{0.0f, 0.0f}` | [x] |
| 4 | `c2Witness` | `s->count` is any value other than 1, 2, or 3 | writes `{0.0f, 0.0f}` to both output vectors | [x] |
| 5 | `c2L` | `s->count` is any value other than 1 or 2 | returns `{0.0f, 0.0f}` | [x] |
| 6 | `c2Support` | `count <= 0`, while `verts` still points to at least one readable element | loop is skipped after reading `verts[0]`; returns index `0` | [x] |
| 7 | `c2Collided` | `typeA` is outside 0..=2 | returns `0` without dereferencing either shape pointer | [x] |
| 8 | `c2Collided` | `typeA == C2_TYPE_CIRCLE` and `typeB` is outside 0..=2 | returns `0` without dereferencing `B` | [x] |
| 9 | `c2Collided` | `typeA == C2_TYPE_AABB` and `typeB` is outside 0..=2 | returns `0` without dereferencing `B` | [x] |
| 10 | `c2Collided` | `typeA == C2_TYPE_CAPSULE` and `typeB` is outside 0..=2 | returns `0` without dereferencing `B` | [x] |

The generic boundary audit also covers:

- null optional `c2GJK` transform, output, iteration, and cache pointers;
- zero and negative `c2Support` counts with a readable first element;
- invalid integer enum values for both `c2MakeProxy` and `c2Collided`;
- zero divisors/zero vectors, which C does not reject and instead processes
  using IEEE-754 arithmetic.

Excluded as undefined in the C ground truth: null pointers passed where the C
body unconditionally dereferences them, invalid shape types passed to `c2GJK`
(which then consumes an uninitialized proxy), and oversized cache/simplex
counts that index beyond fixed arrays.
