# Error and rejection surface

The C library has no error enum, error macro, assertion, explicit null check,
or length parameter. Its rejection sentinel is `0`. Rows 1–21 are the distinct
source-level false/rejection branches. Rows 22–28 track the mandatory generic
FFI boundary cases even though the C code does not guard them.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `c2AABBtoAABB` | `B.max.x < A.min.x` | `0` | [x] |
| 2 | `c2AABBtoAABB` | `A.max.x < B.min.x` | `0` | [x] |
| 3 | `c2AABBtoAABB` | `B.max.y < A.min.y` | `0` | [x] |
| 4 | `c2AABBtoAABB` | `A.max.y < B.min.y` | `0` | [x] |
| 5 | `c2AABBtoPoint` | `B.x < A.min.x` | `0` | [x] |
| 6 | `c2AABBtoPoint` | `B.y < A.min.y` | `0` | [x] |
| 7 | `c2AABBtoPoint` | `B.x > A.max.x` | `0` | [x] |
| 8 | `c2AABBtoPoint` | `B.y > A.max.y` | `0` | [x] |
| 9 | `c2CircleToPoint` | squared point distance is equal to or greater than `A.r * A.r` | `0` | [x] |
| 10 | `c2RaytoCircle` | `disc < 0` | `0`; `out` unchanged | [x] |
| 11 | `c2RaytoCircle` | `disc >= 0` and computed `t < 0` | `0`; `out` unchanged | [x] |
| 12 | `c2RaytoCircle` | `disc >= 0` and computed `t > A.t` | `0`; `out` unchanged | [x] |
| 13 | `c2RaytoAABB` | segment bounding box does not overlap `B` (`!c2AABBtoAABB(a_box, B)`) | `0`; `out` unchanged | [x] |
| 14 | `c2RaytoAABB` | overlap passes but separating-axis value `d > 0` | `0`; `out` unchanged | [x] |
| 15 | `c2RaytoAABB` | all four `tN <= 1.0f` tests are false (reachable with unordered/NaN intermediates) | `0`; `out` unchanged | [x] |
| 16 | `c2RaytoCapsule` | start is outside body/end circles and lateral path neither crosses the axis nor approaches within `B.r` | `0` after initializing `out` | [x] |
| 17 | `c2RaytoCapsule` | `abs(yAp.x) < B.r`, `yAp.y < 0`, and delegated `c2RaytoCircle(A, Ca, out)` rejects | delegated `0` | [x] |
| 18 | `c2RaytoCapsule` | `abs(yAp.x) < B.r`, `yAp.y >= 0`, and delegated `c2RaytoCircle(A, Cb, out)` rejects | delegated `0` | [x] |
| 19 | `c2RaytoCapsule` | side crossing has intersection coordinate `y <= 0` and delegated `c2RaytoCircle(A, Ca, out)` rejects | delegated `0` | [x] |
| 20 | `c2RaytoCapsule` | side crossing has intersection coordinate `y >= yBb.y` and delegated `c2RaytoCircle(A, Cb, out)` rejects | delegated `0` | [x] |
| 21 | `c2CastRay` | `typeB` is outside `0..=2` | C reaches the end of the non-void function; observed ABI return and output must match | [x] |
| 22 | `c2RaytoCircle` | `out == NULL` on a rejecting ray | `0`; no dereference | [x] |
| 23 | `c2RaytoCircle` | `out == NULL` on a hit | process receives `SIGSEGV` | [x] |
| 24 | `c2RaytoAABB` | `out == NULL` on a rejecting ray | `0`; no dereference | [x] |
| 25 | `c2RaytoAABB` | `out == NULL` on a hit | process receives `SIGSEGV` | [x] |
| 26 | `c2RaytoCapsule` | `out == NULL` | process receives `SIGSEGV` before geometric tests | [x] |
| 27 | `c2CastRay` | `B == NULL` with a valid `typeB` | process receives `SIGSEGV` while copying the selected shape | [x] |
| 28 | `c2CastRay` | `out == NULL` with a valid discriminator and a selected-shape hit | process receives `SIGSEGV` in delegated hit handling | [x] |
| 29 | `c2CastRay` | `B == NULL` and `out == NULL` with an invalid discriminator | neither pointer is dereferenced; observed invalid-enum ABI return must match | [x] |
| 30 | `c2CastRay` | `out == NULL` with a valid discriminator and a selected-shape miss | delegated `0`; no `out` dereference | [x] |
| 31 | `spec_ray` | `cast == NULL` on a hit | process receives `SIGSEGV` in delegated circle hit handling | [x] |
| 32 | `spec_ray` | `cast == NULL` on a miss | `0`; no dereference | [x] |

Not applicable: zero/oversized lengths (there are no length arguments).
