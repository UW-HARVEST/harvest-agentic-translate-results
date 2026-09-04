# Configuration surface

The CMake build defines no compile-time feature switches. Cargo also declares
no features, so the feature matrix is the single empty/default feature set
(also exercised with `--no-default-features`).

Rows below are derived from the exported entry points plus every runtime
branching axis in `src/lib.c`: scalar/vector boundary shapes, overlap
orientation, ray hit/miss location, capsule region, polygon count and clipping
state, optional transform, dispatcher enum, and the fixed convenience wrapper.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | arbitrary finite `x,y` | [x] |
| 2 | `c2Dot` | mixed-sign finite vectors | [x] |
| 3 | `c2Len` | zero vector | [x] |
| 4 | `c2Len` | nonzero finite vector | [x] |
| 5 | `c2Add` | arbitrary finite vectors | [x] |
| 6 | `c2Sub` | arbitrary finite vectors | [x] |
| 7 | `c2Mulvs` | zero scalar | [x] |
| 8 | `c2Mulvs` | positive/negative nonzero scalar | [x] |
| 9 | `c2Div` | nonzero divisor | [x] |
| 10 | `c2Div` | zero divisor (C floating-point infinities/NaNs) | [x] |
| 11 | `c2Norm` | nonzero vector | [x] |
| 12 | `c2Norm` | zero vector (C floating-point NaNs) | [x] |
| 13 | `c2Minv` | A lower on both components | [x] |
| 14 | `c2Minv` | mixed component ordering and equality | [x] |
| 15 | `c2Maxv` | A higher on both components | [x] |
| 16 | `c2Maxv` | mixed component ordering and equality | [x] |
| 17 | `c2Skew` | arbitrary finite vector | [x] |
| 18 | `c2Absv` | positive, negative, and signed-zero components | [x] |
| 19 | `c2AABBtoAABB` | overlapping interiors | [x] |
| 20 | `c2AABBtoAABB` | touching boundary (inclusive overlap) | [x] |
| 21 | `c2AABBtoAABB` | B strictly left of A | [x] |
| 22 | `c2AABBtoAABB` | B strictly right of A | [x] |
| 23 | `c2AABBtoAABB` | B strictly below A | [x] |
| 24 | `c2AABBtoAABB` | B strictly above A | [x] |
| 25 | `c2AABBtoPoint` | point in interior | [x] |
| 26 | `c2AABBtoPoint` | point on each boundary/corner (inclusive) | [x] |
| 27 | `c2AABBtoPoint` | point below each of the four limits | [x] |
| 28 | `c2CircleToPoint` | point strictly inside | [x] |
| 29 | `c2CircleToPoint` | point exactly on radius (exclusive) | [x] |
| 30 | `c2CircleToPoint` | point strictly outside | [x] |
| 31 | `c2RaytoCircle` | forward hit inside `A.t` | [x] |
| 32 | `c2RaytoCircle` | tangent hit (`disc == 0`) | [x] |
| 33 | `c2RaytoCircle` | negative discriminant miss | [x] |
| 34 | `c2RaytoCircle` | intersection behind origin (`t < 0`) | [x] |
| 35 | `c2RaytoCircle` | intersection beyond segment (`t > A.t`) | [x] |
| 36 | `c2RaytoAABB` | hit left face | [x] |
| 37 | `c2RaytoAABB` | hit right face | [x] |
| 38 | `c2RaytoAABB` | hit bottom face | [x] |
| 39 | `c2RaytoAABB` | hit top face | [x] |
| 40 | `c2RaytoAABB` | ray starts inside box | [x] |
| 41 | `c2RaytoAABB` | segment bounding box rejects | [x] |
| 42 | `c2RaytoAABB` | segment bounding boxes overlap but separating-axis `d > 0` | [x] |
| 43 | `c2RaytoAABB` | plane-parameter set has no `t <= 1` | [x] |
| 44 | `c2CCW90` | arbitrary finite vector | [x] |
| 45 | `c2MulmvT` | arbitrary matrix/vector | [x] |
| 46 | `c2RaytoCapsule` | start inside rectangular body | [x] |
| 47 | `c2RaytoCapsule` | start inside A endcap only | [x] |
| 48 | `c2RaytoCapsule` | start inside B endcap only | [x] |
| 49 | `c2RaytoCapsule` | hit positive local-x side | [x] |
| 50 | `c2RaytoCapsule` | hit negative local-x side | [x] |
| 51 | `c2RaytoCapsule` | hit A circular end | [x] |
| 52 | `c2RaytoCapsule` | hit B circular end | [x] |
| 53 | `c2RaytoCapsule` | complete miss | [x] |
| 54 | `c2RotIdentity` | no inputs; exact identity rotation | [x] |
| 55 | `c2xIdentity` | no inputs; exact identity transform | [x] |
| 56 | `c2Mulrv` | identity and nontrivial rotation values | [x] |
| 57 | `c2MulrvT` | identity and nontrivial rotation values | [x] |
| 58 | `c2MulxvT` | translation plus nontrivial rotation | [x] |
| 59 | `c2RaytoPoly` | `bx_ptr == NULL` identity mode, count 4, entering hit | [x] |
| 60 | `c2RaytoPoly` | non-null translated/rotated transform, entering hit | [x] |
| 61 | `c2RaytoPoly` | count 0 and negative count | [x] |
| 62 | `c2RaytoPoly` | counts 1 and 8 | [x] |
| 63 | `c2RaytoPoly` | over-capacity count 9 with over-allocated backing storage | [x] |
| 64 | `c2RaytoPoly` | parallel-outside rejection | [x] |
| 65 | `c2RaytoPoly` | interval rejection (`hi < lo`) | [x] |
| 66 | `c2RaytoPoly` | no entering plane (`index == ~0`) | [x] |
| 67 | `c2CastRay` | `typeB == CIRCLE (0)` | [x] |
| 68 | `c2CastRay` | `typeB == AABB (1)` | [x] |
| 69 | `c2CastRay` | `typeB == CAPSULE (2)` | [x] |
| 70 | `c2CastRay` | `typeB == POLY (3)`, null transform | [x] |
| 71 | `c2CastRay` | `typeB == POLY (3)`, non-null transform | [x] |
| 72 | `c2CastRay` | invalid enum values (`-1`, `4`, `INT_MIN`, `INT_MAX`) | [x] |
| 73 | `poly_ray` | fixed two-ray composed operation | [x] |

