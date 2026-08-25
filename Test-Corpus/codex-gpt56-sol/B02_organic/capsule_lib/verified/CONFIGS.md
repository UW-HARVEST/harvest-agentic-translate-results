# Configuration Surface

Build-time configuration has one valid combination:

| # | Cargo features | CMake options | checked |
|---|----------------|---------------|---------|
| B1 | empty set (`--no-default-features`) | default; the CMake file declares no options or preprocessor definitions | [x] |

Runtime rows below come from every exported C entry point and every branch or
switch state reachable through its public ABI. "Random" means a fixed-seed
batch containing finite ordinary values plus signed zero and boundary-focused
values where the operation supports them.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|--------|
| 1 | `c2V` | random scalar pair | [x] |
| 2 | `c2Mulvs` | random vector and scalar, including scalar zero | [x] |
| 3 | `c2Maxv` | all 4 independent x/y winner combinations, including equal components | [x] |
| 4 | `c2Minv` | all 4 independent x/y winner combinations, including equal components | [x] |
| 5 | `c2Clampv` | each component below, inside, and above its interval (3 x 3 matrix), including endpoints | [x] |
| 6 | `c2Sub` | random vector pairs | [x] |
| 7 | `c2Dot` | random vector pairs, including orthogonal and zero vectors | [x] |
| 8 | `c2RotIdentity`, `c2xIdentity` | no-input identity constructors | [x] |
| 9 | `c2BBVerts` | random AABB, including zero-width/height and reversed endpoints | [x] |
| 10 | `c2MakeProxy` | circle shape (`type == 0`) | [x] |
| 11 | `c2MakeProxy` | AABB shape (`type == 1`) | [x] |
| 12 | `c2MakeProxy` | capsule shape (`type == 2`) | [x] |
| 13 | `c2Len` | zero and random nonzero vectors | [x] |
| 14 | `c2Det2` | random, parallel, and perpendicular vector pairs | [x] |
| 15 | `c2GJKSimplexMetric` | `count == 1` and out-of-range/default counts | [x] |
| 16 | `c2GJKSimplexMetric` | `count == 2` segment metric | [x] |
| 17 | `c2GJKSimplexMetric` | `count == 3` signed triangle metric | [x] |
| 18 | `c2Mulrv` | random rotation coefficients and vector | [x] |
| 19 | `c2Add` | random vector pairs | [x] |
| 20 | `c2Mulxv` | random transform and vector | [x] |
| 21 | `c22` | `v <= 0` selects vertex A | [x] |
| 22 | `c22` | `v > 0 && u <= 0` selects vertex B | [x] |
| 23 | `c22` | `v > 0 && u > 0` retains edge AB | [x] |
| 24 | `c23` | `vAB <= 0 && uCA <= 0` selects vertex A | [x] |
| 25 | `c23` | `uAB <= 0 && vBC <= 0` selects vertex B | [x] |
| 26 | `c23` | `uBC <= 0 && vCA <= 0` selects vertex C | [x] |
| 27 | `c23` | positive AB weights and `wABC <= 0` selects edge AB | [x] |
| 28 | `c23` | positive BC weights and `uABC <= 0` selects edge BC | [x] |
| 29 | `c23` | positive CA weights and `vABC <= 0` selects edge CA | [x] |
| 30 | `c23` | otherwise retains triangle ABC | [x] |
| 31 | `c2Neg`, `c2Skew`, `c2CCW90` | random vectors, including signed zero | [x] |
| 32 | `c2D` | `count == 1` | [x] |
| 33 | `c2D` | `count == 2` and determinant is positive | [x] |
| 34 | `c2D` | `count == 2` and determinant is nonpositive | [x] |
| 35 | `c2D` | `count == 3` and out-of-range/default counts | [x] |
| 36 | `c2Support` | `count == 0` or `count == 1` (first element is still read) | [x] |
| 37 | `c2Support` | many vertices; maximum at first, interior, or last index, including ties | [x] |
| 38 | `c2Witness` | `count == 1` | [x] |
| 39 | `c2Witness` | `count == 2` weighted edge | [x] |
| 40 | `c2Witness` | `count == 3` weighted triangle | [x] |
| 41 | `c2Witness` | out-of-range/default count | [x] |
| 42 | `c2Div` | random vector with nonzero divisor | [x] |
| 43 | `c2Div` | zero divisor, including signed zero (IEEE infinities/NaNs) | [x] |
| 44 | `c2Norm` | random nonzero vector | [x] |
| 45 | `c2Norm` | zero vector (IEEE NaNs) | [x] |
| 46 | `c2L` | `count == 1` | [x] |
| 47 | `c2L` | `count == 2` weighted edge | [x] |
| 48 | `c2L` | `count == 3` and out-of-range/default counts | [x] |
| 49 | `c2MulrvT` | random rotation coefficients and vector | [x] |
| 50 | `c2GJK` | all 9 ordered circle/AABB/capsule type pairs, identity transforms (`ax_ptr == bx_ptr == NULL`), `use_radius == 0` | [x] |
| 51 | `c2GJK` | all 9 ordered type pairs, identity transforms, `use_radius != 0`, disjoint/overlap/touching shapes | [x] |
| 52 | `c2GJK` | all 9 ordered type pairs crossed with explicit transform pointer states: A only, B only, and both | [x] |
| 53 | `c2GJK` | optional outputs crossed independently: `outA`, `outB`, and `iterations` null/non-null | [x] |
| 54 | `c2GJK` | cache pointer null, empty (`count == 0`), reusable populated cache, and metric-forced cache rejection | [x] |
| 55 | `c2GJK` | loop exits: simplex hit, increasing distance, tiny direction, duplicate support, and 20-iteration cap when reachable | [x] |
| 56 | `c2AABBtoAABB` | overlap/containment and exact touching | [x] |
| 57 | `c2AABBtoAABB` | separated in each of the four tested directions | [x] |
| 58 | `c2AABBtoCapsule` | separated, overlapping, and exactly touching | [x] |
| 59 | `c2CapsuletoCapsule` | separated, overlapping, and exactly touching | [x] |
| 60 | `c2CircletoCircle` | separated, overlapping/contained, and exactly tangent | [x] |
| 61 | `c2CircletoAABB` | center inside, nearest edge/corner outside, and exact tangent | [x] |
| 62 | `c2CircletoCapsule` | `da < 0`: nearest endpoint A; separated/overlap/tangent | [x] |
| 63 | `c2CircletoCapsule` | `da >= 0 && db < 0`: nearest segment interior; separated/overlap/tangent | [x] |
| 64 | `c2CircletoCapsule` | `da >= 0 && db >= 0`: nearest endpoint B; separated/overlap/tangent | [x] |
| 65 | `c2Collided` | all 9 ordered valid type pairs | [x] |
| 66 | `capsule` | random capsule endpoints/radius across all observed 3-bit collision masks | [x] |

Build/runtime options not present in the C source: preprocessor backends, byte
order, element format, allocation mode, and caller-set global state.

