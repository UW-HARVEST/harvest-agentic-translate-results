# Error and rejection surface

There are no error enums, `assert` statements, `RETURN_ERROR` macros, or
negative error codes in the C source. Collision/query rejection is represented
by integer `0`. These rows are derived from each explicit rejection condition
or rejecting delegated branch in `src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `c2RaytoCircle` | `disc = b*b-c < 0` | `0` | [x] |
| 2 | `c2RaytoCircle` | discriminant is nonnegative but computed `t < 0` | `0` | [x] |
| 3 | `c2RaytoCircle` | discriminant is nonnegative but computed `t > A.t` | `0` | [x] |
| 4 | `c2AABBtoAABB` | `B.max.x < A.min.x` | `0` | [x] |
| 5 | `c2AABBtoAABB` | `A.max.x < B.min.x` | `0` | [x] |
| 6 | `c2AABBtoAABB` | `B.max.y < A.min.y` | `0` | [x] |
| 7 | `c2AABBtoAABB` | `A.max.y < B.min.y` | `0` | [x] |
| 8 | `c2RaytoAABB` | ray segment bounding AABB does not overlap `B` | `0` | [x] |
| 9 | `c2RaytoAABB` | segment/AABB separating-axis value `d > 0` | `0` | [x] |
| 10 | `c2RaytoAABB` | all four `tN <= 1.0` comparisons are false (`hit == 0`; reachable with NaN plane parameters) | `0` | [x] |
| 11 | `c2AABBtoPoint` | `B.x < A.min.x` | `0` | [x] |
| 12 | `c2AABBtoPoint` | `B.y < A.min.y` | `0` | [x] |
| 13 | `c2AABBtoPoint` | `B.x > A.max.x` | `0` | [x] |
| 14 | `c2AABBtoPoint` | `B.y > A.max.y` | `0` | [x] |
| 15 | `c2CircleToPoint` | squared distance is greater than or equal to `A.r*A.r` (boundary is rejected) | `0` | [x] |
| 16 | `c2RaytoCapsule` | start is outside body/endcaps and neither centerline crossing nor minimum absolute local x is `< B.r` | `0` | [x] |
| 17 | `c2RaytoCapsule` | `abs(yAp.x) < B.r`, `yAp.y < 0`, and delegated `c2RaytoCircle(A, capsule_a, out)` rejects | delegated `0` | [x] |
| 18 | `c2RaytoCapsule` | `abs(yAp.x) < B.r`, `yAp.y >= 0`, and delegated `c2RaytoCircle(A, capsule_b, out)` rejects | delegated `0` | [x] |
| 19 | `c2RaytoCapsule` | side crossing computes `y <= 0` and delegated A-end circle cast rejects | delegated `0` | [x] |
| 20 | `c2RaytoCapsule` | side crossing computes `y >= yBb.y` and delegated B-end circle cast rejects | delegated `0` | [x] |
| 21 | `c2RaytoPoly` | for an edge, `den == 0 && num < 0` (parallel outside that plane) | `0` | [x] |
| 22 | `c2RaytoPoly` | clipping updates make `hi < lo` | `0` | [x] |
| 23 | `c2RaytoPoly` | loop completes with `index == ~0` (no entering plane) | `0` | [x] |
| 24 | `c2RaytoPoly` | `B.count <= 0`, so the loop is empty and `index == ~0` | `0` | [x] |
| 25 | `c2CastRay` | `typeB` is not `0`, `1`, `2`, or `3` | `0` | [x] |

Mechanical validation notes:

- The only pointer check in C is `bx_ptr ? *bx_ptr : c2xIdentity()` in
  `c2RaytoPoly`; null `bx_ptr` is a supported identity-transform mode.
- C does not validate `B`, `out`, `cast1`, or `cast2` before dereference.
  Null dereference behavior is therefore tested in isolated child processes,
  rather than represented as an error-code row.
- C does not validate `c2Poly.count` against the physical capacity of 8.
  Counts `0`, negative, `8`, and `9` are covered; the `9` case uses an
  over-allocated backing object so both implementations can observe the same
  ninth vertex/normal without reading unmapped storage.
