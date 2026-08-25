# Configuration Surface

The manifest has no `[features]` table and CMake has no options, preprocessor
configuration, or alternate sources. The complete build-time matrix is:

| # | Cargo features | CMake options | [ ] |
|---|----------------|---------------|-----|
| B1 | empty feature set (`--no-default-features`) | defaults | [x] |

The rows below come from all 39 symbols exported by the C shared object and
all `if`/`switch` branches in `c_src/src/lib.c`. Each randomized test row also
includes finite values, signed zero, infinities, and NaNs where the operation
accepts arbitrary floats.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | arbitrary two-float construction | [x] |
| 2 | `c2Mulvs` | arbitrary vector and scalar | [x] |
| 3 | `c2Maxv` | Cartesian product of `a < b`, `a == b`, `a > b`, and unordered per lane | [x] |
| 4 | `c2Minv` | Cartesian product of `a < b`, `a == b`, `a > b`, and unordered per lane | [x] |
| 5 | `c2Clampv` | Cartesian product of below, inside, and above bounds per lane | [x] |
| 6 | `c2Sub` | arbitrary vectors | [x] |
| 7 | `c2Dot` | arbitrary vectors | [x] |
| 8 | `c2RotIdentity` | no-input identity constructor | [x] |
| 9 | `c2xIdentity` | no-input identity constructor | [x] |
| 10 | `c2BBVerts` | arbitrary AABB, four output vertices | [x] |
| 11 | `c2MakeProxy` | circle: radius plus one vertex | [x] |
| 12 | `c2MakeProxy` | AABB: zero radius plus four vertices | [x] |
| 13 | `c2MakeProxy` | capsule: radius plus two vertices | [x] |
| 14 | `c2Len` | arbitrary vector, including zero and non-finite lanes | [x] |
| 15 | `c2Det2` | arbitrary vectors | [x] |
| 16 | `c2GJKSimplexMetric` | simplex count 1 | [x] |
| 17 | `c2GJKSimplexMetric` | simplex count 2 | [x] |
| 18 | `c2GJKSimplexMetric` | simplex count 3 | [x] |
| 19 | `c2Mulrv` | arbitrary rotation coefficients and vector | [x] |
| 20 | `c2Add` | arbitrary vectors | [x] |
| 21 | `c2Mulxv` | arbitrary transform and vector | [x] |
| 22 | `c22` | Voronoi region A (`v <= 0`) | [x] |
| 23 | `c22` | Voronoi region B (`v > 0 && u <= 0`) | [x] |
| 24 | `c22` | edge AB (`v > 0 && u > 0`) | [x] |
| 25 | `c23` | vertex A (`vAB <= 0 && uCA <= 0`) | [x] |
| 26 | `c23` | vertex B (`uAB <= 0 && vBC <= 0`) | [x] |
| 27 | `c23` | vertex C (`uBC <= 0 && vCA <= 0`) | [x] |
| 28 | `c23` | edge AB (`uAB > 0 && vAB > 0 && wABC <= 0`) | [x] |
| 29 | `c23` | edge BC (`uBC > 0 && vBC > 0 && uABC <= 0`) | [x] |
| 30 | `c23` | edge CA (`uCA > 0 && vCA > 0 && vABC <= 0`) | [x] |
| 31 | `c23` | triangle interior (final branch) | [x] |
| 32 | `c2Neg`, `c2Skew`, `c2CCW90` | arbitrary vector for each orientation operation | [x] |
| 33 | `c2D` | simplex count 1 | [x] |
| 34 | `c2D` | simplex count 2 and positive determinant | [x] |
| 35 | `c2D` | simplex count 2 and non-positive determinant | [x] |
| 36 | `c2Support` | one vertex | [x] |
| 37 | `c2Support` | 2, 4, and 8 vertices with a unique maximum | [x] |
| 38 | `c2Support` | 2, 4, and 8 vertices with tied maxima; first index wins | [x] |
| 39 | `c2Witness` | simplex count 1 | [x] |
| 40 | `c2Witness` | simplex count 2 and arbitrary weights/divisor | [x] |
| 41 | `c2Witness` | simplex count 3 and arbitrary weights/divisor | [x] |
| 42 | `c2Div` | nonzero, signed-zero, infinite, and NaN divisor | [x] |
| 43 | `c2Norm` | nonzero, zero, infinite, and NaN vector | [x] |
| 44 | `c2L` | simplex count 1 | [x] |
| 45 | `c2L` | simplex count 2 and arbitrary weights/divisor | [x] |
| 46 | `c2MulrvT` | arbitrary rotation coefficients and vector | [x] |
| 47 | `c2GJK` | all 3 x 3 ordered shape-type pairs | [x] |
| 48 | `c2GJK` | `ax_ptr`/`bx_ptr`: all four NULL/non-NULL combinations | [x] |
| 49 | `c2GJK` | `use_radius == 0` and `use_radius != 0` | [x] |
| 50 | `c2GJK` | separated, exactly touching, and overlapping shapes | [x] |
| 51 | `c2GJK` | all eight NULL/non-NULL combinations of `outA`, `outB`, and `iterations` | [x] |
| 52 | `c2GJK` | cache NULL, zero-count cache, reusable warm cache, and rejected warm cache | [x] |
| 53 | `c2AABBtoAABB` | separated on each side/axis, touching, and overlapping | [x] |
| 54 | `c2AABBtoCapsule` | separated, touching, and overlapping | [x] |
| 55 | `c2CapsuletoCapsule` | separated, touching, and overlapping | [x] |
| 56 | `c2CircletoCircle` | separated, touching, and overlapping | [x] |
| 57 | `c2CircletoAABB` | center below/inside/above each axis; separated, touching, and overlapping | [x] |
| 58 | `c2CircletoCapsule` | nearest feature A endpoint, segment interior, or B endpoint; separated/touching/overlapping | [x] |
| 59 | `c2Collided` | all 3 x 3 ordered valid type pairs | [x] |
| 60 | `ptr_from_parts` | circle allocation and field layout | [x] |
| 61 | `ptr_from_parts` | AABB allocation and field layout | [x] |
| 62 | `ptr_from_parts` | capsule allocation and field layout | [x] |
| 63 | `omni_collide` | all 3 x 3 ordered valid type pairs, randomized shape fields | [x] |
