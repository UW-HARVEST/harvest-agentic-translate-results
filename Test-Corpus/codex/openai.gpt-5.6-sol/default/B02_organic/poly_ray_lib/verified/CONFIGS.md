# Configuration surface

There are no Cargo features and no C preprocessor feature switches. The build
matrix is therefore the default build and the equivalent
`--no-default-features` build.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | arbitrary finite `x,y` constructor inputs | [x] |
| 2 | `c2Dot` | arbitrary finite vector pair | [x] |
| 3 | `c2Len` | nonzero finite vector | [x] |
| 4 | `c2Len` | zero vector | [x] |
| 5 | `c2Add` | arbitrary finite vector pair | [x] |
| 6 | `c2Sub` | arbitrary finite vector pair | [x] |
| 7 | `c2Mulvs` | arbitrary vector with positive, zero, and negative scalar | [x] |
| 8 | `c2Div` | arbitrary vector with nonzero positive and negative divisor | [x] |
| 9 | `c2Div` | zero divisor (IEEE infinity/NaN result, not rejected) | [x] |
| 10 | `c2Norm` | nonzero finite vector | [x] |
| 11 | `c2Norm` | zero vector (IEEE NaN components, not rejected) | [x] |
| 12 | `c2Minv` | x from A, y from A | [x] |
| 13 | `c2Minv` | x from A, y from B | [x] |
| 14 | `c2Minv` | x from B, y from A | [x] |
| 15 | `c2Minv` | x from B, y from B, including equal components | [x] |
| 16 | `c2Maxv` | x from A, y from A | [x] |
| 17 | `c2Maxv` | x from A, y from B | [x] |
| 18 | `c2Maxv` | x from B, y from A | [x] |
| 19 | `c2Maxv` | x from B, y from B, including equal components | [x] |
| 20 | `c2Skew` | arbitrary finite vector | [x] |
| 21 | `c2Absv` | positive x, positive y | [x] |
| 22 | `c2Absv` | negative x, positive y | [x] |
| 23 | `c2Absv` | positive x, negative y | [x] |
| 24 | `c2Absv` | negative x, negative y, including signed zero boundaries | [x] |
| 25 | `c2RaytoCircle` | secant hit with `0 < t < A.t` | [x] |
| 26 | `c2RaytoCircle` | tangent hit (`disc == 0`) | [x] |
| 27 | `c2RaytoCircle` | hit at lower boundary `t == 0` | [x] |
| 28 | `c2RaytoCircle` | hit at upper boundary `t == A.t` | [x] |
| 29 | `c2AABBtoAABB` | positive-area overlap | [x] |
| 30 | `c2AABBtoAABB` | edge/corner contact (strict separation checks remain false) | [x] |
| 31 | `c2RaytoAABB` | hit selected from min-x plane (`n=(-1,0)`) | [x] |
| 32 | `c2RaytoAABB` | hit selected from max-x plane (`n=(1,0)`) | [x] |
| 33 | `c2RaytoAABB` | hit selected from min-y plane (`n=(0,-1)`) | [x] |
| 34 | `c2RaytoAABB` | hit selected from max-y plane (`n=(0,1)`) | [x] |
| 35 | `c2RaytoAABB` | edge/corner contact and plane-time tie ordering | [x] |
| 36 | `c2CCW90` | arbitrary finite vector | [x] |
| 37 | `c2MulmvT` | arbitrary finite 2x2 matrix and vector | [x] |
| 38 | `c2AABBtoPoint` | point strictly inside | [x] |
| 39 | `c2AABBtoPoint` | point on min/max edge or corner | [x] |
| 40 | `c2CircleToPoint` | point strictly inside | [x] |
| 41 | `c2RaytoCapsule` | ray starts inside rectangular body | [x] |
| 42 | `c2RaytoCapsule` | ray starts inside endpoint A circle | [x] |
| 43 | `c2RaytoCapsule` | ray starts inside endpoint B circle | [x] |
| 44 | `c2RaytoCapsule` | ray hits endpoint A from outside | [x] |
| 45 | `c2RaytoCapsule` | ray hits endpoint B from outside | [x] |
| 46 | `c2RaytoCapsule` | ray hits positive-radius side | [x] |
| 47 | `c2RaytoCapsule` | ray hits negative-radius side | [x] |
| 48 | `c2RotIdentity` | no inputs; identity rotation | [x] |
| 49 | `c2xIdentity` | no inputs; identity transform | [x] |
| 50 | `c2Mulrv` | arbitrary finite rotation coefficients/vector | [x] |
| 51 | `c2MulrvT` | arbitrary finite rotation coefficients/vector | [x] |
| 52 | `c2MulxvT` | arbitrary finite translation/rotation/vector | [x] |
| 53 | `c2RaytoPoly` | `bx_ptr == NULL`, enter through edge 0 | [x] |
| 54 | `c2RaytoPoly` | `bx_ptr == NULL`, enter through edge 1 | [x] |
| 55 | `c2RaytoPoly` | `bx_ptr == NULL`, enter through edge 2 | [x] |
| 56 | `c2RaytoPoly` | `bx_ptr == NULL`, enter through edge 3 | [x] |
| 57 | `c2RaytoPoly` | non-null identity transform | [x] |
| 58 | `c2RaytoPoly` | non-null translated transform | [x] |
| 59 | `c2RaytoPoly` | non-null rotated transform | [x] |
| 60 | `c2RaytoPoly` | maximum in-struct `count == 8` | [x] |
| 61 | `c2CastRay` | `typeB=0` circle dispatch; `bx` ignored/null | [x] |
| 62 | `c2CastRay` | `typeB=1` AABB dispatch; `bx` ignored/null | [x] |
| 63 | `c2CastRay` | `typeB=2` capsule dispatch; `bx` ignored/null | [x] |
| 64 | `c2CastRay` | `typeB=3` polygon dispatch with null transform | [x] |
| 65 | `c2CastRay` | `typeB=3` polygon dispatch with non-null transform | [x] |
| 66 | `poly_ray` | fixed two-ray composed operation and hit bitmask | [x] |
| 67 | `c2RaytoPoly` | unchecked one-past-capacity `count == 9` with caller-provided padded backing storage | [x] |
