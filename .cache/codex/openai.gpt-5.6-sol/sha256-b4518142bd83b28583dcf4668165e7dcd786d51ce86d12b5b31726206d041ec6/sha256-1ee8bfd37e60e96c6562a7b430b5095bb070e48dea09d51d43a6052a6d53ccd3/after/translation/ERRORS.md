# Error-surface table

The C source contains no `assert`, `RETURN_ERROR`, `return -1`, error enum, or
documented length/range validation. The explicit handled rejection branches are
the four `default: return 0` paths in `c2Collided`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `c2Collided` | `typeA == C2_TYPE_CIRCLE` and `typeB` is not 0, 1, or 2 | `0` | [x] |
| 2 | `c2Collided` | `typeA == C2_TYPE_AABB` and `typeB` is not 0, 1, or 2 | `0` | [x] |
| 3 | `c2Collided` | `typeA == C2_TYPE_CAPSULE` and `typeB` is not 0, 1, or 2 | `0` | [x] |
| 4 | `c2Collided` | `typeA` is not 0, 1, or 2 (for any `typeB`) | `0` | [x] |
| 5 | `c2MakeProxy` | `type` is not 0, 1, or 2; no `switch` case executes | return normally and leave the output proxy byte-for-byte unchanged | [x] |

## Explicitly unhandled/undefined inputs

These are not rejection rows because the C source does not define a result:

- `ptr_from_parts` with an invalid `C2_TYPE` reaches the end of a non-void
  function without returning.
- `omni_collide` with an invalid type consumes that undefined return value.
- `c2Support` always reads `verts[0]`; null `verts` or zero count is undefined.
- Pointer-taking helpers dereference required shape/simplex/output pointers;
  null for those required pointers is undefined.
- `c2GJK` requires valid shape pointers and valid shape types. Its transform,
  witness-output, iteration-output, and cache pointers are explicitly optional.
- A non-empty `c2GJKCache` must contain a count and indices valid for its fixed
  arrays and the selected shape proxies; the C source does not range-check them.

All defined rejection rows were verified through both shared-library FFI
boundaries in `tests/differential.rs`.
