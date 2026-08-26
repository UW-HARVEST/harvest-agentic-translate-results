# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no build
options or conditional sources. There is exactly one valid combination:

| # | Cargo invocation feature set | CMake configuration | tested |
|---|------------------------------|---------------------|--------|
| 1 | `--no-default-features` (empty set) | default | [x] |

## Runtime and Input Configurations

Rows are derived from the exported functions and the `if`, ternary, loop, and
`switch` branches in `c_src/src/lib.c`. Floating-point inputs are finite unless
a row explicitly names zero, infinity, or NaN. Randomized rows exercise
positive, negative, and signed-zero components where those values do not select
a separately listed branch.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|-------------------------------------------|--------|
| 1 | `c2V` | arbitrary scalar component pair | [x] |
| 2 | `c2Dot` | arbitrary vector pair | [x] |
| 3 | `c2Len` | nonzero vector and zero vector | [x] |
| 4 | `c2Add` | arbitrary vector pair | [x] |
| 5 | `c2Sub` | arbitrary vector pair | [x] |
| 6 | `c2Mulvs` | vector times positive, zero, and negative scalar | [x] |
| 7 | `c2Div` | positive and negative nonzero divisor | [x] |
| 8 | `c2Div` | positive and negative zero divisor (IEEE infinity/NaN path) | [x] |
| 9 | `c2Norm` | nonzero vector | [x] |
| 10 | `c2Norm` | zero vector | [x] |
| 11 | `c2Minv` | all four independent outcomes of `a.x < b.x` and `a.y < b.y`, including equality | [x] |
| 12 | `c2Maxv` | all four independent outcomes of `a.x > b.x` and `a.y > b.y`, including equality | [x] |
| 13 | `c2Skew` | arbitrary vector | [x] |
| 14 | `c2Absv` | all four component sign combinations, including signed zero | [x] |
| 15 | `c2RaytoCircle` | `disc >= 0` and root `0 <= t <= A.t` (tangent and secant hits) | [x] |
| 16 | `c2RaytoCircle` | `disc < 0` miss | [x] |
| 17 | `c2RaytoCircle` | real root before origin (`t < 0`) | [x] |
| 18 | `c2RaytoCircle` | real root beyond segment (`t > A.t`) | [x] |
| 19 | `c2AABBtoAABB` | interior overlap, containment, and edge/corner touching | [x] |
| 20 | `c2AABBtoAABB` | separated on each of negative/positive x/y sides | [x] |
| 21 | `c2RaytoAABB` | broad-phase AABB miss | [x] |
| 22 | `c2RaytoAABB` | broad phase overlaps but separating-axis `d > 0` | [x] |
| 23 | `c2RaytoAABB` | hit selecting min-x normal (`t0` maximum) | [x] |
| 24 | `c2RaytoAABB` | hit selecting max-x normal (`t1` maximum) | [x] |
| 25 | `c2RaytoAABB` | hit selecting min-y normal (`t2` maximum) | [x] |
| 26 | `c2RaytoAABB` | hit selecting max-y normal (`t3` maximum/final `else`) | [x] |
| 27 | `c2RaytoAABB` | all plane-hit comparisons false (NaN-bearing input) | [x] |
| 28 | `c2CCW90` | arbitrary vector | [x] |
| 29 | `c2MulmvT` | arbitrary 2x2 matrix and vector | [x] |
| 30 | `c2AABBtoPoint` | point in interior and on each boundary/corner | [x] |
| 31 | `c2AABBtoPoint` | point outside each of negative/positive x/y bounds | [x] |
| 32 | `c2CircleToPoint` | point inside | [x] |
| 33 | `c2CircleToPoint` | point exactly on boundary and outside | [x] |
| 34 | `c2RaytoCapsule` | ray starts inside rectangular body | [x] |
| 35 | `c2RaytoCapsule` | ray starts inside cap A | [x] |
| 36 | `c2RaytoCapsule` | ray starts inside cap B | [x] |
| 37 | `c2RaytoCapsule` | `abs(yAp.x) < r`, route to cap A (`yAp.y < 0`) | [x] |
| 38 | `c2RaytoCapsule` | `abs(yAp.x) < r`, route to cap B (`yAp.y >= 0`) | [x] |
| 39 | `c2RaytoCapsule` | side crossing projects before A (`y <= 0`) | [x] |
| 40 | `c2RaytoCapsule` | side crossing projects after B (`y >= yBb.y`) | [x] |
| 41 | `c2RaytoCapsule` | body-side hit with `yAp.x > 0` | [x] |
| 42 | `c2RaytoCapsule` | body-side hit with `yAp.x <= 0` | [x] |
| 43 | `c2RaytoCapsule` | no body or endcap crossing | [x] |
| 44 | `c2RotIdentity` | no inputs | [x] |
| 45 | `c2xIdentity` | no inputs | [x] |
| 46 | `c2Mulrv` | identity and arbitrary sine/cosine pair | [x] |
| 47 | `c2MulrvT` | identity and arbitrary sine/cosine pair | [x] |
| 48 | `c2MulxvT` | identity, translation-only, and rotation-plus-translation transform | [x] |
| 49 | `c2RaytoPoly` | null `bx_ptr` identity transform; entering-edge hit; counts `1`, `4`, and `8` | [x] |
| 50 | `c2RaytoPoly` | nonnull translated/rotated `bx_ptr`; entering-edge hit | [x] |
| 51 | `c2RaytoPoly` | parallel outside edge (`den == 0 && num < 0`) | [x] |
| 52 | `c2RaytoPoly` | clipping interval becomes empty (`hi < lo`) | [x] |
| 53 | `c2RaytoPoly` | no entering edge (`index == ~0`), including ray starting inside | [x] |
| 54 | `c2RaytoPoly` | zero and negative `count` | [x] |
| 55 | `c2CastRay` | `typeB = C2_TYPE_CIRCLE (0)` | [x] |
| 56 | `c2CastRay` | `typeB = C2_TYPE_AABB (1)` | [x] |
| 57 | `c2CastRay` | `typeB = C2_TYPE_CAPSULE (2)` | [x] |
| 58 | `c2CastRay` | `typeB = C2_TYPE_POLY (3)` with null and nonnull `bx` | [x] |
| 59 | `c2CastRay` | out-of-range enum values (negative and greater than `3`), with null pointers | [x] |
| 60 | `poly_ray` | fixed composed two-ray operation through `c2CastRay` and `c2RaytoPoly` | [x] |
