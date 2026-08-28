# Error Surface

Mechanical review covered every `return`, `switch`/`default`, comparison,
assertion, null check, and min/max token in `include/lib.h` and `src/lib.c`.
There are no assertions, error macros, error enums, range checks, null checks,
length parameters, or min/max constants. The three rows below correspond to
the three distinct rejecting `default` branches in `collided`.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `collided` | `typeA` is neither `C2_TYPE_CIRCLE` (0) nor `C2_TYPE_AABB` (1), for any `typeB`; pointers are not read | returns `0` | [x] |
| 2 | `collided` | `typeA == C2_TYPE_CIRCLE` and `typeB` is neither 0 nor 1; pointers are not read | returns `0` | [x] |
| 3 | `collided` | `typeA == C2_TYPE_AABB` and `typeB` is neither 0 nor 1; pointers are not read | returns `0` | [x] |

For valid type pairs, `collided` dereferences both pointers without checking
them. A null pointer in those configurations is undefined behavior rather than
a C rejection path. Null pointers are safely exercised for all three rejection
rows because the C branch returns before dereferencing them. There are no
lengths or additional enum-typed parameters to boundary-test.
