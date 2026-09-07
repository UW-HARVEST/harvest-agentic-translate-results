# Error surface

The library uses `0` as the no-hit/rejection result. It has no error enum,
assertion, `-1`, or `NULL` return. Pointer parameters other than the optional
`bx_ptr` are dereferenced without validation in C; passing null to those
dereferences is C undefined behavior rather than a defined rejection.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `c2RaytoCircle` | discriminant `b*b-c < 0` | `0`; `out` untouched | [x] |
| 2 | `c2RaytoCircle` | intersection parameter `t < 0` | `0`; `out` untouched | [x] |
| 3 | `c2RaytoCircle` | intersection parameter `t > A.t` | `0`; `out` untouched | [x] |
| 4 | `c2AABBtoAABB` | `B.max.x < A.min.x` | `0` | [x] |
| 5 | `c2AABBtoAABB` | `A.max.x < B.min.x` | `0` | [x] |
| 6 | `c2AABBtoAABB` | `B.max.y < A.min.y` | `0` | [x] |
| 7 | `c2AABBtoAABB` | `A.max.y < B.min.y` | `0` | [x] |
| 8 | `c2RaytoAABB` | ray segment AABB does not overlap B (`!c2AABBtoAABB(a_box, B)`) | `0`; `out` untouched | [x] |
| 9 | `c2RaytoAABB` | segment-vs-box separating-axis distance `d > 0` | `0`; `out` untouched | [x] |
| 10 | `c2RaytoAABB` | no plane helper result satisfies `tN <= 1` (reachable with unordered/NaN inputs) | `0`; `out` untouched | [x] |
| 11 | `c2AABBtoPoint` | `B.x < A.min.x` | `0` | [x] |
| 12 | `c2AABBtoPoint` | `B.y < A.min.y` | `0` | [x] |
| 13 | `c2AABBtoPoint` | `B.x > A.max.x` | `0` | [x] |
| 14 | `c2AABBtoPoint` | `B.y > A.max.y` | `0` | [x] |
| 15 | `c2CircleToPoint` | squared distance is equal to `A.r*A.r` (strict comparison) | `0` | [x] |
| 16 | `c2CircleToPoint` | squared distance is greater than `A.r*A.r` | `0` | [x] |
| 17 | `c2RaytoCapsule` | ray fails body crossing test and starts outside body/end circles | `0` (after initializing `out`) | [x] |
| 18 | `c2RaytoCapsule` | body candidate selects endpoint A, delegated `c2RaytoCircle` rejects | `0` | [x] |
| 19 | `c2RaytoCapsule` | body candidate selects endpoint B, delegated `c2RaytoCircle` rejects | `0` | [x] |
| 20 | `c2RaytoPoly` | for an edge, `den == 0 && num < 0` (parallel and outside) | `0`; `out` untouched | [x] |
| 21 | `c2RaytoPoly` | clipping makes `hi < lo` | `0`; `out` untouched | [x] |
| 22 | `c2RaytoPoly` | no entering plane was selected (`index == ~0`), including `count <= 0` and starts-inside/no-entry cases | `0`; `out` untouched | [x] |
| 23 | `c2CastRay` | `typeB` is outside enum values `0..=3` | `0`; neither `B` nor `out` is dereferenced | [x] |
| 24 | `c2CastRay` | circle dispatch delegates to a `c2RaytoCircle` rejection | exact delegated result `0` | [x] |
| 25 | `c2CastRay` | AABB dispatch delegates to a `c2RaytoAABB` rejection | exact delegated result `0` | [x] |
| 26 | `c2CastRay` | capsule dispatch delegates to a `c2RaytoCapsule` rejection | exact delegated result `0` | [x] |
| 27 | `c2CastRay` | polygon dispatch delegates to a `c2RaytoPoly` rejection | exact delegated result `0` | [x] |
| 28 | `c2RaytoPoly` | `count == 9`, one past the declared 8-element arrays; C performs no range rejection (harness supplies padded zero backing) | `0`; `out` untouched | [x] |
| 29 | `c2RaytoCircle` | `out == NULL` on a discriminant miss, so the pointer is not dereferenced | `0` | [x] |
| 30 | `c2RaytoAABB` | `out == NULL` on a broad-phase miss, so the pointer is not dereferenced | `0` | [x] |
| 31 | `c2RaytoPoly` | `out == NULL` on a parallel-outside miss and `bx_ptr == NULL` | `0` | [x] |
| 32 | `c2CastRay` | invalid enum with `B == NULL`, `bx == NULL`, and `out == NULL`; switch default dereferences none | `0` | [x] |
