# Error Surface

This library has no error enum, error macro, assertion, length argument, or
documented min/max range. Its rejection result is the integer no-hit sentinel
`0`. Rows below come from each distinct public source branch that can reject an
input. Delegated capsule-to-circle branches are separate because they are
separate source paths.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| E01 | `c2RaytoCircle` | `disc = dot(m,d)^2 - (dot(m,m)-r^2) < 0` | `0`; `out` unchanged | [x] |
| E02 | `c2RaytoCircle` | `disc >= 0` and `t = -dot(m,d)-sqrt(disc) < 0` | `0`; `out` unchanged | [x] |
| E03 | `c2RaytoCircle` | `disc >= 0` and `t > A.t` | `0`; `out` unchanged | [x] |
| E04 | `c2AABBtoAABB` | `B.max.x < A.min.x` | `0` | [x] |
| E05 | `c2AABBtoAABB` | `A.max.x < B.min.x` | `0` | [x] |
| E06 | `c2AABBtoAABB` | `B.max.y < A.min.y` | `0` | [x] |
| E07 | `c2AABBtoAABB` | `A.max.y < B.min.y` | `0` | [x] |
| E08 | `c2RaytoAABB` | segment AABB does not overlap `B` (`!c2AABBtoAABB(a_box,B)`) | `0`; `out` unchanged | [x] |
| E09 | `c2RaytoAABB` | segment AABB overlaps `B`, but separating-axis distance `d > 0` | `0`; `out` unchanged | [x] |
| E10 | `c2RaytoAABB` | all four plane parameters compare false to `<= 1` (reachable with unordered/NaN input) | `0`; `out` unchanged | [x] |
| E11 | `c2AABBtoPoint` | `B.x < A.min.x` | `0` | [x] |
| E12 | `c2AABBtoPoint` | `B.y < A.min.y` | `0` | [x] |
| E13 | `c2AABBtoPoint` | `B.x > A.max.x` | `0` | [x] |
| E14 | `c2AABBtoPoint` | `B.y > A.max.y` | `0` | [x] |
| E15 | `c2CircleToPoint` | `dot(A.p-B,A.p-B) >= A.r*A.r` (boundary included) | `0` | [x] |
| E16 | `c2RaytoCapsule` | `abs(yAp.x) < B.r`, `yAp.y < 0`, and delegated ray-to-cap-A rejects | delegated exact `0` | [x] |
| E17 | `c2RaytoCapsule` | `abs(yAp.x) < B.r`, `yAp.y >= 0`, and delegated ray-to-cap-B rejects | delegated exact `0` | [x] |
| E18 | `c2RaytoCapsule` | side crossing has `y <= 0`, and delegated ray-to-cap-A rejects | delegated exact `0` | [x] |
| E19 | `c2RaytoCapsule` | side crossing has `y >= yBb.y`, and delegated ray-to-cap-B rejects | delegated exact `0` | [x] |
| E20 | `c2RaytoCapsule` | neither `yAe.x*yAp.x < 0` nor `min(abs(yAe.x),abs(yAp.x)) < B.r` | `0`; `out` remains the function's initialized `{t=0,n=norm(B.b-B.a)}` | [x] |

## Undefined Boundary Inputs

These inputs are not rejection rows because the C source defines no result:

| API boundary | C behavior |
|--------------|------------|
| Null `out` on a path that writes it | null-pointer dereference: undefined behavior |
| Null shape pointer passed to `c2CastRay` with type `0`, `1`, or `2` | null-pointer dereference: undefined behavior |
| `c2CastRay` type other than `0`, `1`, or `2` | control reaches the end of a non-void C function: undefined behavior; the differential test pins the caller's incoming `EAX` to zero and verifies the observed C and Rust result is exactly `0` |
| Null `cast2` passed to `gen_ray` | `c2RaytoCapsule` writes it unconditionally: undefined behavior |

Null output pointers are nevertheless exercised on guaranteed no-write paths
for the APIs where the C source permits that. There are no length parameters,
so zero/oversized-length testing does not apply.
