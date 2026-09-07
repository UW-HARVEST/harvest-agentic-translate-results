# Configuration surface

There are no Cargo features, C preprocessor feature branches, runtime options,
or binary driver. The mechanically visible axes are float class/sign, component
comparison outcome, geometric relation, ray extent, radius class, and
`c2CastRay` discriminator. Every exported entry point is represented.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | arbitrary finite components, including positive and negative zero | [x] |
| 2 | `c2V` | infinities and NaN payloads | [x] |
| 3 | `c2Dot` | finite vectors | [x] |
| 4 | `c2Dot` | zero, overflow, infinity, and NaN operands | [x] |
| 5 | `c2Len` | finite nonzero vector | [x] |
| 6 | `c2Len` | zero vector | [x] |
| 7 | `c2Len` | overflow, infinity, and NaN components | [x] |
| 8 | `c2Add`, `c2Sub` | finite vectors | [x] |
| 9 | `c2Add`, `c2Sub` | signed zero, overflow, infinity, and NaN components | [x] |
| 10 | `c2Mulvs` | finite vector and finite nonzero scalar | [x] |
| 11 | `c2Mulvs` | zero scalar, signed zero, overflow, infinity, and NaN | [x] |
| 12 | `c2Div` | finite vector and finite nonzero divisor | [x] |
| 13 | `c2Div` | positive/negative zero divisor | [x] |
| 14 | `c2Div` | infinity and NaN operands | [x] |
| 15 | `c2Norm` | finite nonzero vector | [x] |
| 16 | `c2Norm` | zero vector | [x] |
| 17 | `c2Norm` | overflow, infinity, and NaN components | [x] |
| 18 | `c2Minv` | each component selects `a` (`a < b`) | [x] |
| 19 | `c2Minv` | each component selects `b` (including equality) | [x] |
| 20 | `c2Minv` | unordered NaN comparisons | [x] |
| 21 | `c2Maxv` | each component selects `a` (`a > b`) | [x] |
| 22 | `c2Maxv` | each component selects `b` (including equality) | [x] |
| 23 | `c2Maxv` | unordered NaN comparisons | [x] |
| 24 | `c2Skew`, `c2CCW90` | finite, signed-zero, infinity, and NaN components | [x] |
| 25 | `c2Absv` | positive components | [x] |
| 26 | `c2Absv` | negative components | [x] |
| 27 | `c2Absv` | signed zero and unordered NaN components | [x] |
| 28 | `c2MulmvT` | arbitrary finite matrix/vector | [x] |
| 29 | `c2MulmvT` | zero, overflow, infinity, and NaN terms | [x] |
| 30 | `c2AABBtoAABB` | strict overlap | [x] |
| 31 | `c2AABBtoAABB` | edge/corner touching (comparisons are strict) | [x] |
| 32 | `c2AABBtoAABB` | separation on each of the four tested sides | [x] |
| 33 | `c2AABBtoPoint` | point strictly inside | [x] |
| 34 | `c2AABBtoPoint` | point on each boundary/corner | [x] |
| 35 | `c2AABBtoPoint` | point outside each of the four tested sides | [x] |
| 36 | `c2CircleToPoint` | point strictly inside with positive radius and with a negative radius of the same magnitude | [x] |
| 37 | `c2CircleToPoint` | point exactly on radius | [x] |
| 38 | `c2CircleToPoint` | point outside radius, plus zero radius at the center | [x] |
| 39 | `c2RaytoCircle` | two-intersection hit with nearest `t` in `[0, A.t]` | [x] |
| 40 | `c2RaytoCircle` | tangent (`disc == 0`) in ray extent | [x] |
| 41 | `c2RaytoCircle` | negative discriminant miss | [x] |
| 42 | `c2RaytoCircle` | intersection behind origin (`t < 0`) | [x] |
| 43 | `c2RaytoCircle` | intersection beyond finite extent (`t > A.t`) | [x] |
| 44 | `c2RaytoCircle` | ray starts inside/on circle, zero direction, zero/negative extent, and IEEE special values | [x] |
| 45 | `c2RaytoAABB` | hit selecting left-face normal | [x] |
| 46 | `c2RaytoAABB` | hit selecting right-face normal | [x] |
| 47 | `c2RaytoAABB` | hit selecting bottom-face normal | [x] |
| 48 | `c2RaytoAABB` | hit selecting top-face normal | [x] |
| 49 | `c2RaytoAABB` | starts inside/on boundary, corner/tangent contact, and zero-length segment | [x] |
| 50 | `c2RaytoAABB` | broad-phase separated miss | [x] |
| 51 | `c2RaytoAABB` | overlapping segment bounds but separating-axis miss | [x] |
| 52 | `c2RaytoAABB` | reversed/degenerate box and IEEE special values | [x] |
| 53 | `c2RaytoCapsule` | ray starts inside rectangular body | [x] |
| 54 | `c2RaytoCapsule` | ray starts inside endpoint A circle | [x] |
| 55 | `c2RaytoCapsule` | ray starts inside endpoint B circle | [x] |
| 56 | `c2RaytoCapsule` | side hit on positive local-x side | [x] |
| 57 | `c2RaytoCapsule` | side hit on negative local-x side | [x] |
| 58 | `c2RaytoCapsule` | endpoint A cap hit | [x] |
| 59 | `c2RaytoCapsule` | endpoint B cap hit | [x] |
| 60 | `c2RaytoCapsule` | miss without lateral crossing/proximity | [x] |
| 61 | `c2RaytoCapsule` | zero-length/reversed capsule, zero/negative radius, zero ray extent, and IEEE special values | [x] |
| 62 | `c2CastRay` | discriminator `0`, circle shape, hit and miss | [x] |
| 63 | `c2CastRay` | discriminator `1`, AABB shape, hit and miss | [x] |
| 64 | `c2CastRay` | discriminator `2`, capsule shape, hit and miss | [x] |
| 65 | `c2CastRay` | discriminator one step below/above valid range (`-1`, `3`) | [x] |
| 66 | `spec_ray` | normalized mouse ray hits circle | [x] |
| 67 | `spec_ray` | normalized mouse ray misses or ends before circle | [x] |
| 68 | `spec_ray` | tangent, mouse equals ray origin, zero/negative radius, and IEEE special values | [x] |
