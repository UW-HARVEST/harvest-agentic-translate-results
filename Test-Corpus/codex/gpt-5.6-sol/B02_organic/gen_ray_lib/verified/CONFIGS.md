# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, no default features, and no optional
dependencies. `c_src/CMakeLists.txt` has no project options, preprocessor
definitions, or conditional sources. Therefore there is exactly one valid
build-time combination:

| # | Rust feature combination | C configuration | status |
|---|--------------------------|-----------------|-----|
| B01 | `--no-default-features` (empty feature set) | default CMake configuration | [x] |

## Runtime and Input Configurations

The public runtime mode axis is `C2_TYPE` in `c2CastRay`: circle `0`, AABB `1`,
or capsule `2`. Input-shape rows below are derived from comparisons and branch
targets in `c_src/src/lib.c`. Randomized finite values are used within each row;
explicit IEEE-754 boundary rows cover zero, signed zero, infinity, and NaN
where those values change a C comparison.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| C01 | `c2V` | ordinary finite components; positive/negative/signed-zero components | [x] |
| C02 | `c2Dot` | positive, negative, and cancellation dot products | [x] |
| C03 | `c2Len` | nonzero vector and zero vector | [x] |
| C04 | `c2Add` | mixed-sign finite components | [x] |
| C05 | `c2Sub` | mixed-sign finite components | [x] |
| C06 | `c2Mulvs` | positive, negative, and zero scalar | [x] |
| C07 | `c2Div` | nonzero divisor and positive/negative zero divisor | [x] |
| C08 | `c2Norm` | nonzero vector and zero vector | [x] |
| C09 | `c2Minv` | all four independent `<` outcomes across x/y | [x] |
| C10 | `c2Minv` | equal values and unordered NaN operands | [x] |
| C11 | `c2Maxv` | all four independent `>` outcomes across x/y | [x] |
| C12 | `c2Maxv` | equal values and unordered NaN operands | [x] |
| C13 | `c2Skew` | mixed-sign and signed-zero components | [x] |
| C14 | `c2Absv` | all four component sign combinations and signed zero | [x] |
| C15 | `c2AABBtoAABB` | interior/overlapping boxes | [x] |
| C16 | `c2AABBtoAABB` | edge and corner touching (`<`, not `<=`) | [x] |
| C17 | `c2AABBtoAABB` | B strictly left (`d0`) | [x] |
| C18 | `c2AABBtoAABB` | B strictly right (`d1`) | [x] |
| C19 | `c2AABBtoAABB` | B strictly below (`d2`) | [x] |
| C20 | `c2AABBtoAABB` | B strictly above (`d3`) | [x] |
| C21 | `c2CCW90` | mixed-sign and signed-zero components | [x] |
| C22 | `c2MulmvT` | general matrix/vector and cancellation | [x] |
| C23 | `c2AABBtoPoint` | strict interior | [x] |
| C24 | `c2AABBtoPoint` | each edge and corner boundary | [x] |
| C25 | `c2AABBtoPoint` | strictly left (`d0`) | [x] |
| C26 | `c2AABBtoPoint` | strictly below (`d1`) | [x] |
| C27 | `c2AABBtoPoint` | strictly right (`d2`) | [x] |
| C28 | `c2AABBtoPoint` | strictly above (`d3`) | [x] |
| C29 | `c2CircleToPoint` | strict interior | [x] |
| C30 | `c2CircleToPoint` | exact boundary | [x] |
| C31 | `c2CircleToPoint` | strict exterior | [x] |
| C32 | `c2RaytoCircle` | negative discriminant miss | [x] |
| C33 | `c2RaytoCircle` | nonnegative discriminant, intersection behind origin (`t < 0`) | [x] |
| C34 | `c2RaytoCircle` | forward intersection beyond finite ray (`t > A.t`) | [x] |
| C35 | `c2RaytoCircle` | ordinary crossing hit | [x] |
| C36 | `c2RaytoCircle` | tangent hit (`disc == 0`) | [x] |
| C37 | `c2RaytoCircle` | origin on circle boundary (`t == 0`) | [x] |
| C38 | `c2RaytoAABB` | broad-phase segment-box miss | [x] |
| C39 | `c2RaytoAABB` | broad-phase overlap but separating-axis miss near a corner | [x] |
| C40 | `c2RaytoAABB` | left-face normal selected (`t0` maximum) | [x] |
| C41 | `c2RaytoAABB` | right-face normal selected (`t1` maximum) | [x] |
| C42 | `c2RaytoAABB` | bottom-face normal selected (`t2` maximum) | [x] |
| C43 | `c2RaytoAABB` | top-face normal selected (`t3` fallback) | [x] |
| C44 | `c2RaytoAABB` | ray starts inside/on box | [x] |
| C45 | `c2RaytoAABB` | zero-length ray | [x] |
| C46 | `c2RaytoAABB` | unordered NaN plane parameters/no selected plane | [x] |
| C47 | `c2RaytoCapsule` | origin inside rectangular body | [x] |
| C48 | `c2RaytoCapsule` | origin inside cap A only | [x] |
| C49 | `c2RaytoCapsule` | origin inside cap B only | [x] |
| C50 | `c2RaytoCapsule` | no lateral crossing and no approach within radius | [x] |
| C51 | `c2RaytoCapsule` | lateral-inside start with `yAp.y < 0`, delegate cap A | [x] |
| C52 | `c2RaytoCapsule` | lateral-inside start with `yAp.y >= 0`, delegate cap B | [x] |
| C53 | `c2RaytoCapsule` | outside start, side intersection `y <= 0`, delegate cap A | [x] |
| C54 | `c2RaytoCapsule` | outside start, side intersection `y >= yBb.y`, delegate cap B | [x] |
| C55 | `c2RaytoCapsule` | body side hit with positive local x | [x] |
| C56 | `c2RaytoCapsule` | body side hit with negative local x | [x] |
| C57 | `c2RaytoCapsule` | degenerate axis (`a == b`) | [x] |
| C58 | `c2RaytoCapsule` | zero radius and exact boundaries | [x] |
| C59 | `c2CastRay` | mode `C2_TYPE_CIRCLE` (`0`) | [x] |
| C60 | `c2CastRay` | mode `C2_TYPE_AABB` (`1`) | [x] |
| C61 | `c2CastRay` | mode `C2_TYPE_CAPSULE` (`2`) | [x] |
| C62 | `gen_ray` | hit mask `0` | [x] |
| C63 | `gen_ray` | hit mask `1` (circle only) | [x] |
| C64 | `gen_ray` | hit mask `2` (capsule only) | [x] |
| C65 | `gen_ray` | hit mask `3` (circle + capsule) | [x] |
| C66 | `gen_ray` | hit mask `4` (AABB only) | [x] |
| C67 | `gen_ray` | hit mask `5` (circle + AABB) | [x] |
| C68 | `gen_ray` | hit mask `6` (capsule + AABB) | [x] |
| C69 | `gen_ray` | hit mask `7` (all shapes) | [x] |
| C70 | `gen_ray` | zero-length ray (`mouse == ray origin`) | [x] |
| C71 | `gen_ray` | zero/negative radii, degenerate capsule, and reversed AABB bounds | [x] |
