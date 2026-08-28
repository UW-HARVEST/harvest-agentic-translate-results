# Error Surface

The C source has no `RETURN_ERROR`, `return -1`, `return NULL`, `assert`, or
explicit range/null rejection. Required pointer arguments are dereferenced
without validation, so null and invalid-type inputs to `c2GJK` are outside the
C function contracts rather than rejected inputs. `c2Support` zero, negative,
and large counts are tested with valid backing storage; even for nonpositive
counts C reads `verts[0]`. Optional null pointers accepted by `c2GJK` are valid
configurations and appear in `CONFIGS.md`.

The complete explicit rejection/default surface is:

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `c2MakeProxy` | `type` is not `C2_TYPE_CIRCLE`, `C2_TYPE_AABB`, or `C2_TYPE_CAPSULE` | returns `void`; writes nothing to `*p` | [x] |
| 2 | `c2Collided` | `typeA == C2_TYPE_CIRCLE` and `typeB` is outside the enum | returns `0` without dereferencing `B` | [x] |
| 3 | `c2Collided` | `typeA == C2_TYPE_AABB` and `typeB` is outside the enum | returns `0` without dereferencing `B` | [x] |
| 4 | `c2Collided` | `typeA == C2_TYPE_CAPSULE` and `typeB` is outside the enum | returns `0` without dereferencing `B` | [x] |
| 5 | `c2Collided` | `typeA` is outside the enum (regardless of `typeB`) | returns `0` without dereferencing either shape | [x] |
