# Error And Rejection Surface

The C library has no error enum, error macro, assertions, or explicit pointer
validation. Its rejection sentinel is integer `0`. Rows below are the distinct
source-level conditions by which a public entry point rejects a geometric
query. Pointer dereferences outside these guarded paths have C undefined
behavior and are not represented as claimed error handling.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `c2RaytoCircle` | with `m=A.p-B.p`, `c=dot(m,m)-B.r*B.r`, and `b=dot(m,A.d)`, `disc=b*b-c < 0` | `0`; `out` untouched | [x] |
| 2 | `c2RaytoCircle` | discriminant is nonnegative but `t = -b-sqrt(disc) < 0` | `0`; `out` untouched | [x] |
| 3 | `c2RaytoCircle` | discriminant is nonnegative but `t = -b-sqrt(disc) > A.t` | `0`; `out` untouched | [x] |
| 4 | `c2AABBtoAABB` | `B.max.x < A.min.x` | `0` | [x] |
| 5 | `c2AABBtoAABB` | `A.max.x < B.min.x` | `0` | [x] |
| 6 | `c2AABBtoAABB` | `B.max.y < A.min.y` | `0` | [x] |
| 7 | `c2AABBtoAABB` | `A.max.y < B.min.y` | `0` | [x] |
| 8 | `c2RaytoAABB` | segment bounding box and `B` fail `c2AABBtoAABB` | `0`; `out` untouched | [x] |
| 9 | `c2RaytoAABB` | SAT distance `abs(dot(n,p0-center))-dot(abs(n),half_extents) > 0` | `0`; `out` untouched | [x] |
| 10 | `c2RaytoAABB` | all four plane candidates are `> 1`, making `hit == 0` | `0`; `out` untouched (arithmetically unreachable for finite inputs, retained as a C return path) | [x] |
| 11 | `c2AABBtoPoint` | `B.x < A.min.x` | `0` | [x] |
| 12 | `c2AABBtoPoint` | `B.y < A.min.y` | `0` | [x] |
| 13 | `c2AABBtoPoint` | `B.x > A.max.x` | `0` | [x] |
| 14 | `c2AABBtoPoint` | `B.y > A.max.y` | `0` | [x] |
| 15 | `c2CircleToPoint` | `dot(A.p-B,A.p-B) >= A.r*A.r` (the boundary is rejected) | `0` | [x] |
| 16 | `c2RaytoCapsule` | start is outside the core and both caps, and neither `yAe.x*yAp.x < 0` nor `min(abs(yAe.x),abs(yAp.x)) < B.r` | `0`; `out` remains the initialized `{t=0,n=norm(B.b-B.a)}` | [x] |
| 17 | `c2RaytoPoly` | for an examined plane, `den == 0 && num < 0` (parallel and outside) | `0`; `out` untouched | [x] |
| 18 | `c2RaytoPoly` | after clipping a plane, `hi < lo` | `0`; `out` untouched | [x] |
| 19 | `c2RaytoPoly` | loop completes with `index == ~0` (no entering plane, including `count <= 0`) | `0`; `out` untouched | [x] |
| 20 | `c2CastRay` | `typeB` is not `0`, `1`, `2`, or `3` | `0`; `B`, `bx`, and `out` are not dereferenced | [x] |
