# Error Surface

The C API has no error enum, negative error return, assertion, explicit null
check, or length argument. Collision predicates and ray casts reject inputs by
returning `0`. The rows below mechanically enumerate each distinct rejecting
branch or range predicate in `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `c2RaytoCircle` | `disc = b*b - c < 0` | `0`; `out` unchanged | [x] |
| 2 | `c2RaytoCircle` | discriminant is nonnegative but `t = -b - sqrtf(disc) < 0` | `0`; `out` unchanged | [x] |
| 3 | `c2RaytoCircle` | discriminant is nonnegative but `t > A.t` | `0`; `out` unchanged | [x] |
| 4 | `c2AABBtoAABB` | `B.max.x < A.min.x` | `0` | [x] |
| 5 | `c2AABBtoAABB` | `A.max.x < B.min.x` | `0` | [x] |
| 6 | `c2AABBtoAABB` | `B.max.y < A.min.y` | `0` | [x] |
| 7 | `c2AABBtoAABB` | `A.max.y < B.min.y` | `0` | [x] |
| 8 | `c2RaytoAABB` | ray segment AABB does not overlap `B` (`!c2AABBtoAABB(a_box, B)`) | `0`; `out` unchanged | [x] |
| 9 | `c2RaytoAABB` | segment-vs-box separating-axis distance `d > 0` | `0`; `out` unchanged | [x] |
| 10 | `c2RaytoAABB` | all four one-dimensional plane parameters compare greater than `1.0f` | `0`; `out` unchanged | [x] |
| 11 | `c2AABBtoPoint` | `B.x < A.min.x` | `0` | [x] |
| 12 | `c2AABBtoPoint` | `B.y < A.min.y` | `0` | [x] |
| 13 | `c2AABBtoPoint` | `B.x > A.max.x` | `0` | [x] |
| 14 | `c2AABBtoPoint` | `B.y > A.max.y` | `0` | [x] |
| 15 | `c2CircleToPoint` | squared distance is equal to or greater than `A.r * A.r` | `0` | [x] |
| 16 | `c2RaytoCapsule` | neither endpoint/shaft start-inside branch applies and `yAe.x * yAp.x < 0 \|\| min(abs(yAe.x), abs(yAp.x)) < B.r` is false | `0` | [x] |
| 17 | `c2RaytoCapsule` | endpoint-A delegation to `c2RaytoCircle` rejects the ray | delegated `0` | [x] |
| 18 | `c2RaytoCapsule` | endpoint-B delegation to `c2RaytoCircle` rejects the ray | delegated `0` | [x] |
| 19 | `c2CastRay` | `typeB` is outside `0..=2` | C falls off the non-void function; the built x86_64 ABI returns incoming `EAX` | [x] |
| 20 | `spec_ray` | delegated circle cast satisfies any `c2RaytoCircle` rejection above | `0`; `cast` unchanged | [x] |

## Unchecked FFI Boundaries

The C source performs no pointer validation. A null `out` pointer is tolerated
only on a path that does not write it; a hit path dereferences it. A null `B`
pointer passed to `c2CastRay` with a valid type is always dereferenced. Those
cases have undefined C behavior rather than an error result. There are no
lengths, documented numeric ranges, or public enum declarations in the header.
