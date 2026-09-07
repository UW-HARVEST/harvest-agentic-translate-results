# Error and rejection surface

The C API has no error enum and performs no explicit argument validation. Its
geometric predicates use `0` as the rejection/no-hit sentinel. Rows below map
each distinct source branch that returns that sentinel. Unchecked FFI inputs
are listed after those branches because the generic boundary audit requires
them even though the C source does not reject them safely.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `c2RaytoCircle` | `disc < 0` | `0` | [x] |
| 2 | `c2RaytoCircle` | `disc >= 0` and computed `t < 0` | `0` | [x] |
| 3 | `c2RaytoCircle` | `disc >= 0` and computed `t > A.t` | `0` | [x] |
| 4 | `c2AABBtoAABB` | `B.max.x < A.min.x` | `0` | [x] |
| 5 | `c2AABBtoAABB` | `A.max.x < B.min.x` | `0` | [x] |
| 6 | `c2AABBtoAABB` | `B.max.y < A.min.y` | `0` | [x] |
| 7 | `c2AABBtoAABB` | `A.max.y < B.min.y` | `0` | [x] |
| 8 | `c2RaytoAABB` | ray segment AABB does not overlap `B`: `!c2AABBtoAABB(a_box, B)` | `0` | [x] |
| 9 | `c2RaytoAABB` | segment and box AABBs overlap but separating-axis distance `d > 0` | `0` | [x] |
| 10 | `c2RaytoAABB` | all four plane candidates fail `tN <= 1.0f` (reachable with unordered/NaN input) | `0` | [x] |
| 11 | `c2AABBtoPoint` | `B.x < A.min.x` | `0` | [x] |
| 12 | `c2AABBtoPoint` | `B.y < A.min.y` | `0` | [x] |
| 13 | `c2AABBtoPoint` | `B.x > A.max.x` | `0` | [x] |
| 14 | `c2AABBtoPoint` | `B.y > A.max.y` | `0` | [x] |
| 15 | `c2CircleToPoint` | `d2 >= A.r * A.r` (boundary and exterior are rejected) | `0` | [x] |
| 16 | `c2RaytoCapsule` | neither initial-overlap branch nor approach/crossing condition is true | `0` | [x] |
| 17 | `c2RaytoCapsule` | near-axis, below endpoint: delegated `c2RaytoCircle(A, Ca, out)` rejects | delegated `0` | [x] |
| 18 | `c2RaytoCapsule` | near-axis, at/above endpoint: delegated `c2RaytoCircle(A, Cb, out)` rejects | delegated `0` | [x] |
| 19 | `c2RaytoCapsule` | side crossing gives `y <= 0`: delegated `c2RaytoCircle(A, Ca, out)` rejects | delegated `0` | [x] |
| 20 | `c2RaytoCapsule` | side crossing gives `y >= yBb.y`: delegated `c2RaytoCircle(A, Cb, out)` rejects | delegated `0` | [x] |
| 21 | `c2CastRay` | `typeB` is outside `0..=2` | falls off non-void C function; compiler/runtime result is compared exactly | [x] |
| 22 | raycast functions | `out == NULL` on a path that writes output | unchecked dereference; process terminates by signal | [x] |
| 23 | `c2CastRay` | `B == NULL` with a valid type | unchecked dereference; process terminates by signal | [x] |
| 24 | `gen_ray` | any required output pointer is `NULL` on a path that writes it | unchecked dereference; process terminates by signal | [x] |

There are no length parameters, documented numeric ranges, asserts,
`RETURN_ERROR` macros, `return -1`, or `return NULL` statements in this source.

