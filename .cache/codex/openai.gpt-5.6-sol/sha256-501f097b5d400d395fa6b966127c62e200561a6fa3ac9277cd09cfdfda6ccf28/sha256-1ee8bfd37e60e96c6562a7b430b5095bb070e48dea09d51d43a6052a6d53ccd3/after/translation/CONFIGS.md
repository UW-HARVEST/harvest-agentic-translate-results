# Configuration Surface

No Cargo features, C preprocessor feature flags, or runtime option setters are
present. The axes below come from every public entry point and every branch or
shape distinction in `src/lib.c`. Randomized values within each row are used by
the differential tests; named boundary shapes are also included explicitly.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | arbitrary finite `x,y` construction | [x] |
| 2 | `c2Dot` | arbitrary finite vectors | [x] |
| 3 | `c2Len` | nonzero finite vector | [x] |
| 4 | `c2Len` | zero vector | [x] |
| 5 | `c2Add`, `c2Sub` | arbitrary finite vectors | [x] |
| 6 | `c2Mulvs` | arbitrary finite vector and nonzero scalar | [x] |
| 7 | `c2Mulvs` | zero scalar | [x] |
| 8 | `c2Div` | finite vector and nonzero divisor | [x] |
| 9 | `c2Div` | zero divisor (IEEE infinities/NaNs) | [x] |
| 10 | `c2Norm` | nonzero finite vector | [x] |
| 11 | `c2Norm` | zero vector (IEEE NaNs) | [x] |
| 12 | `c2Minv` | `a.x < b.x`, `a.y < b.y` | [x] |
| 13 | `c2Minv` | `a.x < b.x`, `a.y >= b.y` | [x] |
| 14 | `c2Minv` | `a.x >= b.x`, `a.y < b.y` | [x] |
| 15 | `c2Minv` | `a.x >= b.x`, `a.y >= b.y`, including equality | [x] |
| 16 | `c2Maxv` | `a.x > b.x`, `a.y > b.y` | [x] |
| 17 | `c2Maxv` | `a.x > b.x`, `a.y <= b.y` | [x] |
| 18 | `c2Maxv` | `a.x <= b.x`, `a.y > b.y` | [x] |
| 19 | `c2Maxv` | `a.x <= b.x`, `a.y <= b.y`, including equality | [x] |
| 20 | `c2Skew`, `c2CCW90` | arbitrary finite vector | [x] |
| 21 | `c2Absv` | positive/nonnegative `x`, positive/nonnegative `y` | [x] |
| 22 | `c2Absv` | negative `x`, positive/nonnegative `y` | [x] |
| 23 | `c2Absv` | positive/nonnegative `x`, negative `y` | [x] |
| 24 | `c2Absv` | negative `x`, negative `y` | [x] |
| 25 | `c2AABBtoAABB` | overlapping interiors | [x] |
| 26 | `c2AABBtoAABB` | touching boundary | [x] |
| 27 | `c2AABBtoAABB` | separated left, right, below, or above | [x] |
| 28 | `c2AABBtoPoint` | point in interior | [x] |
| 29 | `c2AABBtoPoint` | point on boundary | [x] |
| 30 | `c2AABBtoPoint` | point outside left, right, below, or above | [x] |
| 31 | `c2CircleToPoint` | point strictly inside circle | [x] |
| 32 | `c2CircleToPoint` | point exactly on circle | [x] |
| 33 | `c2CircleToPoint` | point outside circle | [x] |
| 34 | `c2RaytoCircle` | secant hit with `0 < t < A.t` | [x] |
| 35 | `c2RaytoCircle` | tangent hit (`disc == 0`) in range | [x] |
| 36 | `c2RaytoCircle` | start exactly on boundary (`t == 0`) | [x] |
| 37 | `c2RaytoCircle` | negative discriminant miss | [x] |
| 38 | `c2RaytoCircle` | intersection behind start (`t < 0`) | [x] |
| 39 | `c2RaytoCircle` | intersection beyond segment (`t > A.t`) | [x] |
| 40 | `c2RaytoAABB` | broad-phase segment-box separation | [x] |
| 41 | `c2RaytoAABB` | broad phase overlaps but SAT distance rejects | [x] |
| 42 | `c2RaytoAABB` | hit selected from min-x plane (`t0`) | [x] |
| 43 | `c2RaytoAABB` | hit selected from max-x plane (`t1`) | [x] |
| 44 | `c2RaytoAABB` | hit selected from min-y plane (`t2`) | [x] |
| 45 | `c2RaytoAABB` | hit selected from max-y plane (`t3`) | [x] |
| 46 | `c2RaytoAABB` | segment starts in/on box, including zero length | [x] |
| 47 | `c2MulmvT` | arbitrary finite matrix columns and vector | [x] |
| 48 | `c2RaytoCapsule` | start inside rectangular core | [x] |
| 49 | `c2RaytoCapsule` | start inside cap at `B.a` | [x] |
| 50 | `c2RaytoCapsule` | start inside cap at `B.b` | [x] |
| 51 | `c2RaytoCapsule` | no side crossing/proximity; miss | [x] |
| 52 | `c2RaytoCapsule` | starts within radius strip below `B.a`; delegate to cap A | [x] |
| 53 | `c2RaytoCapsule` | starts within radius strip above `B.b`; delegate to cap B | [x] |
| 54 | `c2RaytoCapsule` | side crossing falls at/below `B.a`; delegate to cap A | [x] |
| 55 | `c2RaytoCapsule` | side crossing falls at/above `B.b`; delegate to cap B | [x] |
| 56 | `c2RaytoCapsule` | right side hit in capsule body | [x] |
| 57 | `c2RaytoCapsule` | left side hit in capsule body | [x] |
| 58 | `c2RotIdentity`, `c2xIdentity` | identity constructors | [x] |
| 59 | `c2Mulrv`, `c2MulrvT` | arbitrary finite rotation coefficients and vector | [x] |
| 60 | `c2MulxvT` | arbitrary finite translation/rotation and vector | [x] |
| 61 | `c2RaytoPoly` | null transform (identity), empty polygon (`count == 0`) | [x] |
| 62 | `c2RaytoPoly` | null transform, negative count (loop skipped) | [x] |
| 63 | `c2RaytoPoly` | null transform, one-plane polygon | [x] |
| 64 | `c2RaytoPoly` | null transform, multi-plane polygon (`count` 3 through 7) | [x] |
| 65 | `c2RaytoPoly` | null transform, maximum in-bounds `count == 8` | [x] |
| 66 | `c2RaytoPoly` | non-null identity transform | [x] |
| 67 | `c2RaytoPoly` | non-null translated/rotated transform | [x] |
| 68 | `c2RaytoPoly` | parallel outside plane (`den == 0 && num < 0`) | [x] |
| 69 | `c2RaytoPoly` | entering plane updates `lo/index` and produces hit | [x] |
| 70 | `c2RaytoPoly` | exiting plane updates `hi` | [x] |
| 71 | `c2RaytoPoly` | clipping interval becomes empty (`hi < lo`) | [x] |
| 72 | `c2RaytoPoly` | no entering plane (`index == ~0`), including start inside | [x] |
| 73 | `c2CastRay` | `typeB == C2_TYPE_CIRCLE (0)` | [x] |
| 74 | `c2CastRay` | `typeB == C2_TYPE_AABB (1)` | [x] |
| 75 | `c2CastRay` | `typeB == C2_TYPE_CAPSULE (2)` | [x] |
| 76 | `c2CastRay` | `typeB == C2_TYPE_POLY (3)`, null and non-null `bx` | [x] |
| 77 | `c2CastRay` | out-of-range enum below/above valid range; other pointers may be null | [x] |
| 78 | `poly_ray` | fixed end-to-end two-ray polygon operation | [x] |
