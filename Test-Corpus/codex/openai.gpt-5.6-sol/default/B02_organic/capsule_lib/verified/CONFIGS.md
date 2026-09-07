# Configuration surface

Rows are derived from every C-exported entry point and the `if`/`switch`
branches in `src/lib.c`. Randomized rows include zero, signed values, boundary
comparisons, and finite values spanning small and large magnitudes.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| 1 | `c2V` | arbitrary scalar pair, including zero and signed values | [x] |
| 2 | `c2Mulvs` | arbitrary vector and scalar | [x] |
| 3 | `c2Maxv` | independently exercise `a > b` and else branches on both axes, including ties/NaN | [x] |
| 4 | `c2Minv` | independently exercise `a < b` and else branches on both axes, including ties/NaN | [x] |
| 5 | `c2Clampv` | below, inside, and above `[lo, hi]` independently per axis | [x] |
| 6 | `c2Sub` | arbitrary vector pair | [x] |
| 7 | `c2Dot` | arbitrary vector pair, including orthogonal and zero vectors | [x] |
| 8 | `c2RotIdentity`, `c2xIdentity` | no-input identity constructors | [x] |
| 9 | `c2BBVerts` | arbitrary AABB, including zero-area and reversed bounds | [x] |
| 10 | `c2MakeProxy` | circle (`type == 0`) | [x] |
| 11 | `c2MakeProxy` | AABB (`type == 1`) | [x] |
| 12 | `c2MakeProxy` | capsule (`type == 2`) | [x] |
| 13 | `c2MakeProxy` | out-of-range type; proxy remains byte-for-byte untouched | [x] |
| 14 | `c2Len` | zero and nonzero vectors | [x] |
| 15 | `c2Det2` | positive, negative, and zero determinant | [x] |
| 16 | `c2GJKSimplexMetric` | `count` default/0/1 returns zero | [x] |
| 17 | `c2GJKSimplexMetric` | `count == 2` segment length | [x] |
| 18 | `c2GJKSimplexMetric` | `count == 3` signed triangle determinant | [x] |
| 19 | `c2Mulrv` | arbitrary rotation coefficients and vector | [x] |
| 20 | `c2Add` | arbitrary vector pair | [x] |
| 21 | `c2Mulxv` | arbitrary transform and vector | [x] |
| 22 | `c22` | `v <= 0`, retain vertex A | [x] |
| 23 | `c22` | `v > 0 && u <= 0`, replace A with B | [x] |
| 24 | `c22` | `v > 0 && u > 0`, retain segment | [x] |
| 25 | `c23` | A vertex Voronoi region | [x] |
| 26 | `c23` | B vertex Voronoi region | [x] |
| 27 | `c23` | C vertex Voronoi region | [x] |
| 28 | `c23` | AB edge Voronoi region | [x] |
| 29 | `c23` | BC edge Voronoi region | [x] |
| 30 | `c23` | CA edge Voronoi region | [x] |
| 31 | `c23` | triangle interior | [x] |
| 32 | `c2Neg`, `c2Skew`, `c2CCW90` | arbitrary vector | [x] |
| 33 | `c2D` | `count == 1` | [x] |
| 34 | `c2D` | `count == 2`, positive determinant branch | [x] |
| 35 | `c2D` | `count == 2`, nonpositive determinant branch | [x] |
| 36 | `c2D` | `count` default/3 returns zero | [x] |
| 37 | `c2Support` | `count == 1` | [x] |
| 38 | `c2Support` | multiple vertices with a unique maximum | [x] |
| 39 | `c2Support` | multiple vertices with tied maximum; first index wins | [x] |
| 40 | `c2Support` | zero count and allocated one-element storage; C still reads element 0 and returns 0 | [x] |
| 41 | `c2Support` | count 9 (one past proxy storage width) with caller-provided 9-element storage | [x] |
| 42 | `c2Witness` | `count == 1` | [x] |
| 43 | `c2Witness` | `count == 2` weighted witness | [x] |
| 44 | `c2Witness` | `count == 3` weighted witness | [x] |
| 45 | `c2Witness` | default count writes zero vectors | [x] |
| 46 | `c2Div` | nonzero divisor | [x] |
| 47 | `c2Div` | zero divisor, preserving IEEE-754 infinities/NaNs | [x] |
| 48 | `c2Norm` | nonzero vector | [x] |
| 49 | `c2Norm` | zero vector, preserving C NaN result | [x] |
| 50 | `c2L` | `count == 1` | [x] |
| 51 | `c2L` | `count == 2` weighted closest point | [x] |
| 52 | `c2L` | default count returns zero | [x] |
| 53 | `c2MulrvT` | arbitrary rotation coefficients and vector | [x] |
| 54 | `c2GJK` | circle-circle; cross product of `use_radius` 0/nonzero, null/nontrivial transforms, null/empty/warm cache, and present/null optional outputs | [x] |
| 55 | `c2GJK` | circle-AABB; same option cross product | [x] |
| 56 | `c2GJK` | circle-capsule; same option cross product | [x] |
| 57 | `c2GJK` | AABB-circle; same option cross product | [x] |
| 58 | `c2GJK` | AABB-AABB; same option cross product | [x] |
| 59 | `c2GJK` | AABB-capsule; same option cross product | [x] |
| 60 | `c2GJK` | capsule-circle; same option cross product | [x] |
| 61 | `c2GJK` | capsule-AABB; same option cross product | [x] |
| 62 | `c2GJK` | capsule-capsule; same option cross product | [x] |
| 63 | `c2GJK` | cache metric rejection condition causes fallback to initial simplex | [x] |
| 64 | `c2GJK` | separated, touching/overlapping-radius, and simplex-hit data shapes | [x] |
| 65 | `c2AABBtoAABB` | separated on each side, overlap, containment, and touching boundary | [x] |
| 66 | `c2AABBtoCapsule` | colliding and separated shapes | [x] |
| 67 | `c2CapsuletoCapsule` | colliding and separated shapes | [x] |
| 68 | `c2CircletoCircle` | overlap, exact tangent (strict false), and separated | [x] |
| 69 | `c2CircletoAABB` | center inside, edge/corner overlap, exact tangent (strict false), separated | [x] |
| 70 | `c2CircletoCapsule` | projection before endpoint A (`da < 0`) | [x] |
| 71 | `c2CircletoCapsule` | projection on segment (`da >= 0 && db < 0`) | [x] |
| 72 | `c2CircletoCapsule` | projection beyond endpoint B (`db >= 0`) | [x] |
| 73 | `c2Collided` | ordered circle-circle dispatch | [x] |
| 74 | `c2Collided` | ordered circle-AABB dispatch | [x] |
| 75 | `c2Collided` | ordered circle-capsule dispatch | [x] |
| 76 | `c2Collided` | ordered AABB-circle dispatch | [x] |
| 77 | `c2Collided` | ordered AABB-AABB dispatch | [x] |
| 78 | `c2Collided` | ordered AABB-capsule dispatch | [x] |
| 79 | `c2Collided` | ordered capsule-circle dispatch | [x] |
| 80 | `c2Collided` | ordered capsule-AABB dispatch | [x] |
| 81 | `c2Collided` | ordered capsule-capsule dispatch | [x] |
| 82 | `capsule` | randomized endpoints/radius covering all 3-bit collision-result combinations reachable by the wrapper | [x] |

Cargo feature combinations: one (`default`; `Cargo.toml` has no `[features]`).

Binary targets: none.
