# Error Surface

This table is derived from every C `default`, explicit null/range check, and
numeric limit in `c_src/src/lib.c`. The C source contains no `assert`,
`RETURN_ERROR`, `return -1`, or `return NULL`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `c2MakeProxy` | `type` is not 0, 1, or 2 | switch executes no case; caller-provided proxy bytes are unchanged | [x] |
| 2 | `c2GJKSimplexMetric` | `s->count` is not 1, 2, or 3 | returns `0.0f` | [x] |
| 3 | `c2D` | `s->count` is not 1 or 2 (including 3) | returns `{0.0f, 0.0f}` | [x] |
| 4 | `c2Support` | `count <= 0` with `verts` pointing to at least one element | reads `verts[0]`, skips the loop, and returns index 0 | [x] |
| 5 | `c2Witness` | `s->count` is not 1, 2, or 3 | writes `{0.0f, 0.0f}` to both outputs | [x] |
| 6 | `c2L` | `s->count` is not 1 or 2 | returns `{0.0f, 0.0f}` | [x] |
| 7 | `c2Collided` | `typeA` is not 0, 1, or 2 | returns 0 without dereferencing either shape | [x] |
| 8 | `c2Collided` | `typeA == C2_TYPE_CIRCLE` and `typeB` is invalid | returns 0 without dereferencing `B` | [x] |
| 9 | `c2Collided` | `typeA == C2_TYPE_AABB` and `typeB` is invalid | returns 0 without dereferencing `B` | [x] |
| 10 | `c2Collided` | `typeA == C2_TYPE_CAPSULE` and `typeB` is invalid | returns 0 without dereferencing `B` | [x] |
| 11 | `c2GJK` | `ax_ptr == NULL` | uses `c2xIdentity()` for A rather than rejecting | [x] |
| 12 | `c2GJK` | `bx_ptr == NULL` | uses `c2xIdentity()` for B rather than rejecting | [x] |
| 13 | `c2GJK` | `cache == NULL` | skips cache read and cache write | [x] |
| 14 | `c2GJK` | `cache != NULL` and `cache->count == 0` | ignores cached simplex, starts at vertex pair 0, then writes a new cache | [x] |
| 15 | `c2GJK` | cached metric satisfies `min(metric, old) < max(metric, old) * 2.0f && metric < -1.0e8f` | rejects cached simplex and starts at vertex pair 0 | [x] |
| 16 | `c2GJK` | search reaches `iter == 20` | terminates and reports 20 through non-null `iterations` | [x] |
| 17 | `c2GJK` | direction squared is less than `FLT_EPSILON * FLT_EPSILON` | terminates the search | [x] |
| 18 | `c2GJK` | `use_radius != 0` and distance is not greater than both radius sum and `FLT_EPSILON` | collapses witnesses to their midpoint and returns 0 | [x] |
| 19 | `c2GJK` | `outA == NULL`, `outB == NULL`, or `iterations == NULL` | skips only the corresponding output write | [x] |

## C Undefined Behavior Boundaries

These inputs have no C return value to compare and therefore are not rejection
rows. Required null pointers are exercised in process-isolated tests, which
verify matching C/Rust termination:

- Null required pointers in `c2BBVerts`, `c2MakeProxy`, simplex helpers,
  `c2Support`, `c2Witness`, `c2GJK` shape pointers, and valid-type
  `c2Collided`.
- A `c2Support` count larger than the actual `verts` allocation.
- Cache counts outside 0 through 3 or cache indices outside the selected
  proxy's vertex range.
- Invalid enum values passed to `c2GJK`, `ptr_from_parts`, or `omni_collide`.
  `c2MakeProxy` leaves an uninitialized proxy or `ptr_from_parts` reaches the
  end of a non-void function, after which behavior is undefined.
- Allocation failure in `ptr_from_parts`; C dereferences the null result from
  `malloc`.
