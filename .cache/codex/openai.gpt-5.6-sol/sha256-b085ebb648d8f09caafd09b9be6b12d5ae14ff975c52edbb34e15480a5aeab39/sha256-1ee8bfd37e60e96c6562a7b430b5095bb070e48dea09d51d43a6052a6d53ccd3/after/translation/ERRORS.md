# Error Surface

The C source has no `assert`, `RETURN_ERROR`, `return -1`, error enum, or
explicit allocation-failure handling. Its explicit rejection/default paths
and boundary behaviors are below. Null transform/output/cache pointers in
`c2GJK` are accepted options and are catalogued in `CONFIGS.md`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] 1 | `c2MakeProxy` | `type < C2_TYPE_CAPSULE` or `type > C2_TYPE_AABB`, with readable `shape` and writable `p` | Switch executes no case; every byte of `*p` remains unchanged |
| [x] 2 | `c2GJKSimplexMetric` | `s->count <= 0` or `s->count > 3` | Default branch returns `0.0f` |
| [x] 3 | `c2D` | `s->count <= 0` or `s->count > 3` | Default branch returns `{0.0f, 0.0f}` |
| [x] 4 | `c2Witness` | `s->count <= 0` or `s->count > 3`, with non-null output pointers | Default branch writes `{0.0f, 0.0f}` to both outputs |
| [x] 5 | `c2L` | `s->count <= 0` or `s->count > 2` | Default branch returns `{0.0f, 0.0f}` |
| [x] 6 | `c2Support` | `count <= 0`, while `verts` still points to at least one readable element | Reads `verts[0]`, skips the loop, and returns index `0` |
| [x] 7 | `c2Support` | `count` exceeds the proxy maximum of 8, while the caller supplies `count` readable elements | Scans all supplied elements and returns the first maximum-dot index |
| [x] 8 | `c2Collided` | `typeA < C2_TYPE_CAPSULE` or `typeA > C2_TYPE_AABB` | Outer default branch returns `0` without dereferencing either shape |
| [x] 9 | `c2Collided` | `typeA == C2_TYPE_CIRCLE` and `typeB` is outside `[C2_TYPE_CAPSULE, C2_TYPE_AABB]` | Circle inner default branch returns `0` |
| [x] 10 | `c2Collided` | `typeA == C2_TYPE_AABB` and `typeB` is outside `[C2_TYPE_CAPSULE, C2_TYPE_AABB]` | AABB inner default branch returns `0` |
| [x] 11 | `c2Collided` | `typeA == C2_TYPE_CAPSULE` and `typeB` is outside `[C2_TYPE_CAPSULE, C2_TYPE_AABB]` | Capsule inner default branch returns `0` |
| [x] 12 | `omni_collide` | either public enum argument is outside `[C2_TYPE_CAPSULE, C2_TYPE_AABB]` | `c2Collided` rejects the invalid type and returns `0`; the C helper's invalid-type pointer value is never dereferenced |
| [x] 13 | `c2GJK` | `ax_ptr == NULL` | Accepted optional pointer; uses `c2xIdentity()` for shape A |
| [x] 14 | `c2GJK` | `bx_ptr == NULL` | Accepted optional pointer; uses `c2xIdentity()` for shape B |
| [x] 15 | `c2GJK` | `cache == NULL` | Accepted optional pointer; cold-starts the simplex and performs no cache write |
| [x] 16 | `c2GJK` | `cache != NULL && cache->count == 0` | Treats the cache as empty, cold-starts, then writes the resulting cache |
| [x] 17 | `c2GJK` | `outA == NULL` | Accepted optional output; computes the result without writing witness A |
| [x] 18 | `c2GJK` | `outB == NULL` | Accepted optional output; computes the result without writing witness B |
| [x] 19 | `c2GJK` | `iterations == NULL` | Accepted optional output; computes the result without writing the iteration count |

`ptr_from_parts` has no `default` and reaches the end of a non-void C function
for an invalid enum. `c2GJK` passes invalid enums to `c2MakeProxy` and then
uses an uninitialized proxy. Those cases have undefined C behavior, not a C
error result, so no byte-identical result exists to test. Required
out-of-range-enum coverage is provided at the public dispatch boundary
(`c2Collided` and `omni_collide`) and at the defined no-op boundary
(`c2MakeProxy`).

All non-optional pointer parameters are raw C preconditions. Passing null where
the C function dereferences it causes undefined behavior rather than a
rejection sentinel.

The remaining numeric constants (`verts[8]`, cache index arrays of length 3,
the 20-iteration cap, `FLT_MAX`, `FLT_EPSILON`, the `-1.0e8f` cache threshold,
and the cache metric ratio `2.0f`) control valid GJK execution rather than
rejecting API input. Their defined paths are covered by `CONFIGS.md` rows
39-40 and 49-58; invalid cache counts/indices would index outside C arrays and
therefore have undefined behavior.
