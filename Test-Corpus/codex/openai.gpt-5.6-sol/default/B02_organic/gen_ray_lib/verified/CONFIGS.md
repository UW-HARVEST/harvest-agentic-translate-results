# Configuration surface

There are no compile-time Cargo features or C preprocessor feature switches.
The runtime surface is the cross-product of exported entry points and the
source branches or input shapes each entry point distinguishes.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `c2V` | arbitrary finite `x`, `y`, including signed zero | [x] |
| 2 | `c2Dot` | finite vectors with positive, negative, zero, and cancellation products | [x] |
| 3 | `c2Len` | zero and nonzero finite vectors | [x] |
| 4 | `c2Add`, `c2Sub` | finite vectors with mixed signs and cancellation | [x] |
| 5 | `c2Mulvs` | negative, zero, and positive scalar | [x] |
| 6 | `c2Div` | negative and positive nonzero divisor, plus zero divisor IEEE result | [x] |
| 7 | `c2Norm` | nonzero vector and zero-vector IEEE result | [x] |
| 8 | `c2Minv` | all four per-axis comparison outcomes (`a.x < b.x` / false × `a.y < b.y` / false) | [x] |
| 9 | `c2Maxv` | all four per-axis comparison outcomes (`a.x > b.x` / false × `a.y > b.y` / false) | [x] |
| 10 | `c2Skew`, `c2CCW90` | arbitrary finite vector | [x] |
| 11 | `c2Absv` | all four per-axis sign outcomes, including `-0.0` | [x] |
| 12 | `c2MulmvT` | arbitrary finite matrix and vector | [x] |
| 13 | `c2RaytoCircle` | secant hit with `0 < t < A.t` | [x] |
| 14 | `c2RaytoCircle` | tangent hit (`disc == 0`) | [x] |
| 15 | `c2RaytoCircle` | ray starts on circle and reports `t == 0` | [x] |
| 16 | `c2AABBtoAABB` | overlapping interiors | [x] |
| 17 | `c2AABBtoAABB` | touching edge/corner (strict separation checks remain false) | [x] |
| 18 | `c2RaytoAABB` | hit chosen by `t0` maximum: min-x normal `(-1, 0)` | [x] |
| 19 | `c2RaytoAABB` | hit chosen by `t1` maximum: max-x normal `(1, 0)` | [x] |
| 20 | `c2RaytoAABB` | hit chosen by `t2` maximum: min-y normal `(0, -1)` | [x] |
| 21 | `c2RaytoAABB` | hit chosen by final branch: max-y normal `(0, 1)` | [x] |
| 22 | `c2RaytoAABB` | corner/tie precedence among equal plane candidates | [x] |
| 23 | `c2AABBtoPoint` | interior point and each inclusive edge/corner boundary | [x] |
| 24 | `c2CircleToPoint` | strict interior point | [x] |
| 25 | `c2RaytoCapsule` | ray starts inside rectangular body (`c2AABBtoPoint`) | [x] |
| 26 | `c2RaytoCapsule` | ray starts inside endpoint circle A only | [x] |
| 27 | `c2RaytoCapsule` | ray starts inside endpoint circle B only | [x] |
| 28 | `c2RaytoCapsule` | near-axis branch selects endpoint A (`yAp.y < 0`) and hits | [x] |
| 29 | `c2RaytoCapsule` | near-axis branch selects endpoint B (`yAp.y >= 0`) and hits | [x] |
| 30 | `c2RaytoCapsule` | side crossing falls below body (`y <= 0`) and endpoint A hits | [x] |
| 31 | `c2RaytoCapsule` | side crossing rises above body (`y >= yBb.y`) and endpoint B hits | [x] |
| 32 | `c2RaytoCapsule` | positive-side body hit (`c > 0`) | [x] |
| 33 | `c2RaytoCapsule` | negative-side body hit (`c <= 0`) | [x] |
| 34 | `c2CastRay` | type `C2_TYPE_CIRCLE` (`0`) | [x] |
| 35 | `c2CastRay` | type `C2_TYPE_AABB` (`1`) | [x] |
| 36 | `c2CastRay` | type `C2_TYPE_CAPSULE` (`2`) | [x] |
| 37 | `gen_ray` | hit mask `0`: no shape hit | [x] |
| 38 | `gen_ray` | hit mask `1`: circle only | [x] |
| 39 | `gen_ray` | hit mask `2`: capsule only | [x] |
| 40 | `gen_ray` | hit mask `3`: circle + capsule | [x] |
| 41 | `gen_ray` | hit mask `4`: AABB only | [x] |
| 42 | `gen_ray` | hit mask `5`: circle + AABB | [x] |
| 43 | `gen_ray` | hit mask `6`: capsule + AABB | [x] |
| 44 | `gen_ray` | hit mask `7`: all three shapes | [x] |
| 45 | all scalar/vector and geometric entry points | IEEE special values actually accepted by the C ABI: `NaN`, infinities, and signed zero | [x] |

Feature combinations: default only (`Cargo.toml` declares no `[features]`).
