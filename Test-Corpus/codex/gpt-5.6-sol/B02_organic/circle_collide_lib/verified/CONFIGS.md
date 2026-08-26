# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no CMake
option or conditional source selection. Consequently there is exactly one valid
build-time combination:

| # | Cargo features | C configuration |
|---|----------------|-----------------|
| 1 | empty set (`--no-default-features`) | default CMake configuration |

## Runtime and Input Configurations

Rows below are derived from all 12 exported C entry points, the `>`/`<`
conditional operators in the vector helpers, both capsule `if` branches, the
shape `switch`, and the final collision comparisons. "IEEE-special" means
signed zero, infinity, and NaN payloads; these are included because C
floating-point comparisons and arithmetic distinguish them.

| # | entry point(s) | configuration (options set + input shape) | passed |
|---|----------------|--------------------------------------------|--------|
| 1 | `c2V` | arbitrary finite and IEEE-special `x`, `y` values | [x] |
| 2 | `c2Mulvs` | arbitrary vectors and finite/IEEE-special scalar multipliers | [x] |
| 3 | `c2Maxv` | `a > b` true independently on each coordinate (all four true/false combinations) | [x] |
| 4 | `c2Maxv` | equal and unordered/NaN coordinate comparisons, for which the C ternary selects `b` | [x] |
| 5 | `c2Minv` | `a < b` true independently on each coordinate (all four true/false combinations) | [x] |
| 6 | `c2Minv` | equal and unordered/NaN coordinate comparisons, for which the C ternary selects `b` | [x] |
| 7 | `c2Clampv` | each coordinate below, within, or above ordered bounds (3 x 3 cross-product) | [x] |
| 8 | `c2Clampv` | equal/reversed bounds and unordered/NaN comparisons | [x] |
| 9 | `c2Sub` | arbitrary finite and IEEE-special vector pairs | [x] |
| 10 | `c2Dot` | arbitrary finite and IEEE-special vector pairs, including overflow/underflow | [x] |
| 11 | `c2CircletoCircle` | finite circles with `d2 < (A.r+B.r)^2` (overlap) | [x] |
| 12 | `c2CircletoCircle` | finite circles with equality at the strict boundary (tangent) | [x] |
| 13 | `c2CircletoCircle` | finite separated circles and negative radii | [x] |
| 14 | `c2CircletoCircle` | IEEE-special coordinates/radii | [x] |
| 15 | `c2CircletoAABB` | center region below/below relative to ordered box bounds; randomized radius covers collision comparison outcomes | [x] |
| 16 | `c2CircletoAABB` | center region below/within relative to ordered box bounds; randomized radius covers collision comparison outcomes | [x] |
| 17 | `c2CircletoAABB` | center region below/above relative to ordered box bounds; randomized radius covers collision comparison outcomes | [x] |
| 18 | `c2CircletoAABB` | center region within/below relative to ordered box bounds; randomized radius covers collision comparison outcomes | [x] |
| 19 | `c2CircletoAABB` | center region within/within relative to ordered box bounds; randomized radius covers collision comparison outcomes | [x] |
| 20 | `c2CircletoAABB` | center region within/above relative to ordered box bounds; randomized radius covers collision comparison outcomes | [x] |
| 21 | `c2CircletoAABB` | center region above/below relative to ordered box bounds; randomized radius covers collision comparison outcomes | [x] |
| 22 | `c2CircletoAABB` | center region above/within relative to ordered box bounds; randomized radius covers collision comparison outcomes | [x] |
| 23 | `c2CircletoAABB` | center region above/above relative to ordered box bounds; randomized radius covers collision comparison outcomes | [x] |
| 24 | `c2CircletoAABB` | equal/reversed bounds, degenerate boxes, negative radii, and IEEE-special values | [x] |
| 25 | `c2CircletoCapsule` | `da < 0`: circle center in the endpoint-A region | [x] |
| 26 | `c2CircletoCapsule` | `da >= 0` and `db < 0`: circle center in the segment-interior region | [x] |
| 27 | `c2CircletoCapsule` | `da >= 0` and `db >= 0`: circle center in the endpoint-B region | [x] |
| 28 | `c2CircletoCapsule` | zero-length capsule, negative radii, strict tangent boundary, and IEEE-special values | [x] |
| 29 | `c2Collided` | `typeB = C2_TYPE_CIRCLE` (0), with valid circle pointers | [x] |
| 30 | `c2Collided` | `typeB = C2_TYPE_AABB` (1), with valid circle/AABB pointers | [x] |
| 31 | `c2Collided` | `typeB = C2_TYPE_CAPSULE` (2), with valid circle/capsule pointers | [x] |
| 32 | `circle_collide` | randomized finite and IEEE-special `(x, y, r)` through the complete three-shape bitmask pipeline | [x] |
