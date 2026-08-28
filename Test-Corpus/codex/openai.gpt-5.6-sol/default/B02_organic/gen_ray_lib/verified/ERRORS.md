# Error and rejection surface

The C source has no error macros, negative error codes, assertions, allocation
failures, length parameters, explicit null checks, or documented numeric
ranges. Its rejection sentinel is `0`. Rows E01-E21 are mechanically derived
from false-return branches and comparisons in `src/lib.c`; E22-E24 cover the
generic FFI boundaries applicable to this API.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|---|
| E01 | `c2RaytoCircle` | `disc = b*b-c < 0` | `0`; `out` unchanged | [x] |
| E02 | `c2RaytoCircle` | `disc >= 0` and `t = -b-sqrt(disc) < 0` | `0`; `out` unchanged | [x] |
| E03 | `c2RaytoCircle` | `disc >= 0` and `t > A.t` | `0`; `out` unchanged | [x] |
| E04 | `c2AABBtoAABB` | `B.max.x < A.min.x` (B strictly left) | `0` | [x] |
| E05 | `c2AABBtoAABB` | `A.max.x < B.min.x` (B strictly right) | `0` | [x] |
| E06 | `c2AABBtoAABB` | `B.max.y < A.min.y` (B strictly below) | `0` | [x] |
| E07 | `c2AABBtoAABB` | `A.max.y < B.min.y` (B strictly above) | `0` | [x] |
| E08 | `c2RaytoAABB` | segment AABB fails `c2AABBtoAABB(a_box, B)` | `0`; `out` unchanged | [x] |
| E09 | `c2RaytoAABB` | segment AABB overlaps B, but separating-axis distance `d > 0` | `0`; `out` unchanged | [x] |
| E10 | `c2RaytoAABB` | all four `tN <= 1.0f` tests are false (NaN plane distances) | `0`; `out` unchanged | [x] |
| E11 | `c2AABBtoPoint` | `B.x < A.min.x` | `0` | [x] |
| E12 | `c2AABBtoPoint` | `B.y < A.min.y` | `0` | [x] |
| E13 | `c2AABBtoPoint` | `B.x > A.max.x` | `0` | [x] |
| E14 | `c2AABBtoPoint` | `B.y > A.max.y` | `0` | [x] |
| E15 | `c2CircleToPoint` | squared distance `d2 >= A.r*A.r` (boundary is rejected) | `0` | [x] |
| E16 | `c2RaytoCapsule` | start is outside body/caps, and side-crossing guard at lines 260-264 is false | `0` after initializing `out` | [x] |
| E17 | `c2RaytoCapsule` | `abs(yAp.x) < r`, `yAp.y < 0`, delegated cap-A circle cast rejects | delegated `0` | [x] |
| E18 | `c2RaytoCapsule` | `abs(yAp.x) < r`, `yAp.y >= 0`, delegated cap-B circle cast rejects | delegated `0` | [x] |
| E19 | `c2RaytoCapsule` | side crossing has `y <= 0`, delegated cap-A circle cast rejects | delegated `0` | [x] |
| E20 | `c2RaytoCapsule` | side crossing has `y >= yBb.y`, delegated cap-B circle cast rejects | delegated `0` | [x] |
| E21 | `c2CastRay` | `typeB` is outside enum values `0..=2` | generated x86_64 C returns incoming `EAX` unchanged | [x] |
| E22 | pointer-output casts | null `out` on a rejection path that performs no write | same rejection sentinel; no dereference | [x] |
| E23 | pointer-output casts | null `out` on a path that writes output | process receives `SIGSEGV` | [x] |
| E24 | `c2CastRay` | null shape pointer for a recognized `typeB` | process receives `SIGSEGV` | [x] |

There are no zero/oversized-length cases because no exported C function takes
a pointer-plus-length, array count, string, or allocation size. One-past-range
enum input is covered by E21.
