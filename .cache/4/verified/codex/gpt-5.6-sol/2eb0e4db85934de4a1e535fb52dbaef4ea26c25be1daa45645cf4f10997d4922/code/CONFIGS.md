# Configuration surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` defines no
options or conditional compilation. There is exactly one valid build
configuration:

| # | Cargo invocation | CMake configuration | [x] |
|---|------------------|---------------------|-----|
| 1 | `--no-default-features` (enabled feature set is empty) | defaults plus `-DCMAKE_POSITION_INDEPENDENT_CODE=ON` | [x] |

## Runtime configurations

Rows are derived from every `if`/`else`, `switch`, and loop boundary in the C
implementation. "Random floats" includes signs, zero, ordinary finite values,
subnormals, infinities, and NaNs where the operation has no pointer precondition.
Each ordered `c2GJK` shape-pair row covers the cross-product of: separated,
touching, and overlapping geometry; `use_radius` zero/nonzero; null/explicit A
transform; null/explicit B transform; null/empty/warm cache; and randomized
finite coordinates/radii.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random `x`, `y` bit patterns | [x] |
| 2 | `c2Mulvs` | random vector and scalar, including zero/sign/special values | [x] |
| 3 | `c2Maxv` | each greater-than branch, equality, and unordered NaN operands | [x] |
| 4 | `c2Minv` | each less-than branch, equality, and unordered NaN operands | [x] |
| 5 | `c2Clampv` | below/in/above range, equal bounds, inverted bounds, and NaNs | [x] |
| 6 | `c2Sub` | random vectors, including cancellation and special values | [x] |
| 7 | `c2Dot` | random vectors, including zero, mixed signs, and special values | [x] |
| 8 | `c2RotIdentity`, `c2xIdentity` | no-input identity constructors | [x] |
| 9 | `c2BBVerts` | random finite min/max, including degenerate and inverted boxes | [x] |
| 10 | `c2MakeProxy` | circle (`type == 0`) | [x] |
| 11 | `c2MakeProxy` | AABB (`type == 1`) | [x] |
| 12 | `c2MakeProxy` | capsule (`type == 2`) | [x] |
| 13 | `c2Len` | zero, finite, subnormal, infinite, and NaN vectors | [x] |
| 14 | `c2Det2` | positive, negative, zero determinants and special values | [x] |
| 15 | `c2GJKSimplexMetric` | `count == 1` and default counts outside `2..=3` | [x] |
| 16 | `c2GJKSimplexMetric` | `count == 2` segment metric | [x] |
| 17 | `c2GJKSimplexMetric` | `count == 3` signed triangle metric | [x] |
| 18 | `c2Mulrv`, `c2MulrvT` | random rotations/vectors, including identity and special values | [x] |
| 19 | `c2Add` | random vectors, including cancellation and special values | [x] |
| 20 | `c2Mulxv` | random transform/vector, including identity and special values | [x] |
| 21 | `c22` | `v <= 0` selects vertex A | [x] |
| 22 | `c22` | `v > 0 && u <= 0` selects vertex B | [x] |
| 23 | `c22` | `v > 0 && u > 0` retains edge AB | [x] |
| 24 | `c23` | `vAB <= 0 && uCA <= 0` selects vertex A | [x] |
| 25 | `c23` | `uAB <= 0 && vBC <= 0` selects vertex B | [x] |
| 26 | `c23` | `uBC <= 0 && vCA <= 0` selects vertex C | [x] |
| 27 | `c23` | positive AB weights and `wABC <= 0` select edge AB | [x] |
| 28 | `c23` | positive BC weights and `uABC <= 0` select edge BC | [x] |
| 29 | `c23` | positive CA weights and `vABC <= 0` select edge CA | [x] |
| 30 | `c23` | otherwise retains triangle ABC | [x] |
| 31 | `c2Neg`, `c2Skew`, `c2CCW90` | random vectors, including signed zero and special values | [x] |
| 32 | `c2D` | `count == 1` | [x] |
| 33 | `c2D` | `count == 2`, determinant positive | [x] |
| 34 | `c2D` | `count == 2`, determinant zero/nonpositive | [x] |
| 35 | `c2D` | `count == 3` and default counts | [x] |
| 36 | `c2Support` | `count == 1` | [x] |
| 37 | `c2Support` | `count > 1`, first remains maximum including ties | [x] |
| 38 | `c2Support` | `count > 1`, a later strict maximum replaces index | [x] |
| 39 | `c2Witness` | `count == 1` | [x] |
| 40 | `c2Witness` | `count == 2` weighted edge | [x] |
| 41 | `c2Witness` | `count == 3` weighted triangle | [x] |
| 42 | `c2Witness` | default count writes zero witnesses | [x] |
| 43 | `c2Div` | positive/negative/zero/special divisor | [x] |
| 44 | `c2Norm` | nonzero finite, zero, infinite, and NaN vectors | [x] |
| 45 | `c2L` | `count == 1` | [x] |
| 46 | `c2L` | `count == 2` weighted edge | [x] |
| 47 | `c2L` | default count returns zero | [x] |
| 48 | `c2GJK` | circle -> circle; full geometry/transform/radius/cache cross-product | [x] |
| 49 | `c2GJK` | circle -> AABB; full geometry/transform/radius/cache cross-product | [x] |
| 50 | `c2GJK` | circle -> capsule; full geometry/transform/radius/cache cross-product | [x] |
| 51 | `c2GJK` | AABB -> circle; full geometry/transform/radius/cache cross-product | [x] |
| 52 | `c2GJK` | AABB -> AABB; full geometry/transform/radius/cache cross-product | [x] |
| 53 | `c2GJK` | AABB -> capsule; full geometry/transform/radius/cache cross-product | [x] |
| 54 | `c2GJK` | capsule -> circle; full geometry/transform/radius/cache cross-product | [x] |
| 55 | `c2GJK` | capsule -> AABB; full geometry/transform/radius/cache cross-product | [x] |
| 56 | `c2GJK` | capsule -> capsule; full geometry/transform/radius/cache cross-product | [x] |
| 57 | `c2GJK` | optional outputs: all null/non-null combinations of `outA`, `outB`, and `iterations` | [x] |
| 58 | `c2GJK` | cache validation branch: zero count, reusable count, and forced rejection condition | [x] |
| 59 | `c2GJK` | loop exits: simplex hit, increasing distance, near-zero direction, duplicate support, and 20-iteration bound where reachable | [x] |
| 60 | `gjk_cache` | `reverse == 0` (AABB -> capsule), random finite arguments; output pointers remain untouched | [x] |
| 61 | `gjk_cache` | `reverse != 0` (capsule -> AABB), random finite arguments; output pointers remain untouched | [x] |
| 62 | `c2Support` | `count <= 0` with a readable `verts[0]`; returns index zero without entering the loop | [x] |
| 63 | `c2Support` | count above the proxy capacity (9 and 1024) with equally sized readable backing storage | [x] |
