# Configuration surface

Rows are derived from every externally linked function and each `if`/`switch`
branch, option, shape kind, count, and boundary represented in
`c_src/src/lib.c`. Randomized tests use a fixed seed.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | arbitrary finite `x`, `y` | [x] |
| 2 | `c2Mulvs` | arbitrary vector; negative, zero, and positive scalar | [x] |
| 3 | `c2Maxv` | each component covers left greater, right greater, and equality | [x] |
| 4 | `c2Minv` | each component covers left less, right less, and equality | [x] |
| 5 | `c2Clampv` | ordinary and inverted per-component bounds | [x] |
| 6 | `c2Sub` | arbitrary finite vectors | [x] |
| 7 | `c2Dot` | negative, zero, and positive dot products | [x] |
| 8 | `c2RotIdentity`, `c2xIdentity` | no-input identity constructors | [x] |
| 9 | `c2BBVerts` | ordinary and inverted AABB endpoints | [x] |
| 10 | `c2MakeProxy` | circle: one vertex and radius | [x] |
| 11 | `c2MakeProxy` | AABB: four vertices and zero radius | [x] |
| 12 | `c2MakeProxy` | capsule: two vertices and radius | [x] |
| 13 | `c2Len` | zero and nonzero vectors | [x] |
| 14 | `c2Det2` | negative, zero, and positive determinant | [x] |
| 15 | `c2GJKSimplexMetric` | simplex count 1 | [x] |
| 16 | `c2GJKSimplexMetric` | simplex count 2 | [x] |
| 17 | `c2GJKSimplexMetric` | simplex count 3 | [x] |
| 18 | `c2Mulrv` | arbitrary rotation coefficients and vector | [x] |
| 19 | `c2Add` | arbitrary finite vectors | [x] |
| 20 | `c2Mulxv` | arbitrary translation/rotation coefficients and vector | [x] |
| 21 | `c22` | `v <= 0` selects vertex A | [x] |
| 22 | `c22` | `v > 0 && u <= 0` selects vertex B | [x] |
| 23 | `c22` | `v > 0 && u > 0` retains edge AB | [x] |
| 24 | `c23` | A vertex Voronoi region | [x] |
| 25 | `c23` | B vertex Voronoi region | [x] |
| 26 | `c23` | C vertex Voronoi region | [x] |
| 27 | `c23` | AB edge Voronoi region | [x] |
| 28 | `c23` | BC edge Voronoi region | [x] |
| 29 | `c23` | CA edge Voronoi region | [x] |
| 30 | `c23` | triangle interior/fallback | [x] |
| 31 | `c2Neg`, `c2Skew`, `c2CCW90` | arbitrary vectors | [x] |
| 32 | `c2D` | simplex count 1 | [x] |
| 33 | `c2D` | count 2 with positive determinant branch | [x] |
| 34 | `c2D` | count 2 with nonpositive determinant branch | [x] |
| 35 | `c2D` | simplex count 3 default zero direction | [x] |
| 36 | `c2Support` | count 1 | [x] |
| 37 | `c2Support` | counts 2 through 8; unique later maximum and equal-dot first-index tie | [x] |
| 38 | `c2Witness` | simplex count 1 | [x] |
| 39 | `c2Witness` | simplex count 2 with weighted interpolation | [x] |
| 40 | `c2Witness` | simplex count 3 with weighted interpolation | [x] |
| 41 | `c2Div` | positive and negative nonzero divisor | [x] |
| 42 | `c2Norm` | nonzero vector | [x] |
| 43 | `c2L` | simplex count 1 | [x] |
| 44 | `c2L` | simplex count 2 with weighted interpolation | [x] |
| 45 | `c2L` | simplex count 3 default zero result | [x] |
| 46 | `c2MulrvT` | arbitrary rotation coefficients and vector | [x] |
| 47 | `c2GJK` | circle-circle; identity transforms, no radius, no cache, all outputs | [x] |
| 48 | `c2GJK` | circle-AABB; identity transforms, no radius, no cache, all outputs | [x] |
| 49 | `c2GJK` | circle-capsule; identity transforms, no radius, no cache, all outputs | [x] |
| 50 | `c2GJK` | AABB-circle; identity transforms, no radius, no cache, all outputs | [x] |
| 51 | `c2GJK` | AABB-AABB; identity transforms, no radius, no cache, all outputs | [x] |
| 52 | `c2GJK` | AABB-capsule; identity transforms, no radius, no cache, all outputs | [x] |
| 53 | `c2GJK` | capsule-circle; identity transforms, no radius, no cache, all outputs | [x] |
| 54 | `c2GJK` | capsule-AABB; identity transforms, no radius, no cache, all outputs | [x] |
| 55 | `c2GJK` | capsule-capsule; identity transforms, no radius, no cache, all outputs | [x] |
| 56 | `c2GJK` | circle-circle; identity transforms, radius enabled | [x] |
| 57 | `c2GJK` | circle-AABB; identity transforms, radius enabled | [x] |
| 58 | `c2GJK` | circle-capsule; identity transforms, radius enabled | [x] |
| 59 | `c2GJK` | AABB-circle; identity transforms, radius enabled | [x] |
| 60 | `c2GJK` | AABB-AABB; identity transforms, radius enabled | [x] |
| 61 | `c2GJK` | AABB-capsule; identity transforms, radius enabled | [x] |
| 62 | `c2GJK` | capsule-circle; identity transforms, radius enabled | [x] |
| 63 | `c2GJK` | capsule-AABB; identity transforms, radius enabled | [x] |
| 64 | `c2GJK` | capsule-capsule; identity transforms, radius enabled | [x] |
| 65 | `c2GJK` | all nine shape pairs with non-null arbitrary transforms | [x] |
| 66 | `c2GJK` | supplied cache with `count == 0` takes fresh-simplex path | [x] |
| 67 | `c2GJK` | warm cache from a prior identical call takes cache-read path | [x] |
| 68 | `c2GJK` | valid shapes with all optional output pointers null | [x] |
| 69 | `c2GJK` | null/non-null optional outputs in mixed combinations | [x] |
| 70 | `c2GJK` | separated, touching, and overlapping shapes exercise radius separation/collapse and hit paths | [x] |
| 71 | `c2GJK` | zero radii and zero-length capsule segments | [x] |
| 72 | `c2GJK` | ordinary and inverted AABB endpoints | [x] |
| 73 | `c2AABBtoAABB` | separated, touching, and overlapping on both axes | [x] |
| 74 | `c2AABBtoCapsule` | separated, touching, and overlapping | [x] |
| 75 | `c2CapsuletoCapsule` | separated, touching, overlapping, and zero-length segments | [x] |
| 76 | `c2CircletoCircle` | separated, tangent, overlapping, and zero/negative radii | [x] |
| 77 | `c2CircletoAABB` | center inside/outside/on edge; tangent and inverted AABB | [x] |
| 78 | `c2CircletoCapsule` | `da < 0`, middle projection, and endpoint-B branches; tangent and zero-length segment | [x] |
| 79 | `c2Collided` | circle-circle | [x] |
| 80 | `c2Collided` | circle-AABB | [x] |
| 81 | `c2Collided` | circle-capsule | [x] |
| 82 | `c2Collided` | AABB-circle (reversed dispatch) | [x] |
| 83 | `c2Collided` | AABB-AABB | [x] |
| 84 | `c2Collided` | AABB-capsule | [x] |
| 85 | `c2Collided` | capsule-circle (reversed dispatch) | [x] |
| 86 | `c2Collided` | capsule-AABB (reversed dispatch) | [x] |
| 87 | `c2Collided` | capsule-capsule | [x] |
| 88 | `ptr_from_parts` | circle allocation/layout | [x] |
| 89 | `ptr_from_parts` | AABB allocation/layout | [x] |
| 90 | `ptr_from_parts` | capsule allocation/layout | [x] |
| 91 | `omni_collide` | circle-circle | [x] |
| 92 | `omni_collide` | circle-AABB | [x] |
| 93 | `omni_collide` | circle-capsule | [x] |
| 94 | `omni_collide` | AABB-circle | [x] |
| 95 | `omni_collide` | AABB-AABB | [x] |
| 96 | `omni_collide` | AABB-capsule | [x] |
| 97 | `omni_collide` | capsule-circle | [x] |
| 98 | `omni_collide` | capsule-AABB | [x] |
| 99 | `omni_collide` | capsule-capsule | [x] |
| 100 | arithmetic/vector helpers and collision predicates | signed zero, infinities, and quiet NaNs where no pointer/index UB results | [x] |

There is no executable target and no Cargo feature is declared. The build
surface therefore has one effective feature configuration; both default and
`--no-default-features` test invocations are still run at the completion gate.
