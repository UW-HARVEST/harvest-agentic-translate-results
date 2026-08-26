# Error Surface

The C source contains no `RETURN_ERROR`, `return -1`, `return NULL`, assertion,
or explicit range-check rejection. These are all explicit unsupported-enum
branches that return or preserve a defined rejection result.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `c2MakeProxy` | `type` is not `C2_TYPE_CIRCLE`, `C2_TYPE_AABB`, or `C2_TYPE_CAPSULE` | Return normally without changing `*p` | [x] |
| 2 | `c2Collided` | `typeA` is not a defined `C2_TYPE` value | Return `0` without reading either shape | [x] |
| 3 | `c2Collided` | `typeA == C2_TYPE_CIRCLE` and `typeB` is not a defined `C2_TYPE` value | Return `0` without reading either shape | [x] |
| 4 | `c2Collided` | `typeA == C2_TYPE_AABB` and `typeB` is not a defined `C2_TYPE` value | Return `0` without reading either shape | [x] |
| 5 | `c2Collided` | `typeA == C2_TYPE_CAPSULE` and `typeB` is not a defined `C2_TYPE` value | Return `0` without reading either shape | [x] |

## Unchecked C Contracts

The following are not rejection paths: the C code dereferences these values
without a check, so violating the contract has undefined behavior and no
stable C result to compare.

- `c2BBVerts`: `out` and `bb` must point to writable/readable objects.
- `c2MakeProxy`: `shape` and `p` must be valid for a supported `type`.
- Simplex helpers: the simplex pointer must be valid.
- `c2Support`: `verts` must contain element zero and all elements selected by
  `count`; zero/negative `count` still reads `verts[0]`.
- `c2Witness`: `s`, `a`, and `b` must be valid.
- `c2GJK`: both shape pointers and both type values must describe matching,
  valid objects. A nonempty cache must contain valid counts and vertex indices.
- `c2Collided`: shape pointers must be valid whenever their enum arm reads them.

`c2GJK` explicitly accepts null transform, output, iteration, and cache
pointers. Those defined null-pointer modes are valid configurations in
`CONFIGS.md`, not errors.
