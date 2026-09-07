# Configuration surface

Rows are derived from the public entry points and every `if`/`switch` branch in
`src/lib.c`. Randomized tests use fixed seeds and force each listed branch.
There are no Cargo features and no executable target.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|---|---|:---:|
| 1 | `c2V` | arbitrary two-scalar vector construction | [x] |
| 2 | `c2Mulvs` | arbitrary vector and scalar | [x] |
| 3 | `c2Maxv` | both components selected from `a` | [x] |
| 4 | `c2Maxv` | x from `a`, y from `b` | [x] |
| 5 | `c2Maxv` | x from `b`, y from `a` | [x] |
| 6 | `c2Maxv` | both components selected from `b`, including ties | [x] |
| 7 | `c2Minv` | both components selected from `a` | [x] |
| 8 | `c2Minv` | x from `a`, y from `b` | [x] |
| 9 | `c2Minv` | x from `b`, y from `a` | [x] |
| 10 | `c2Minv` | both components selected from `b`, including ties | [x] |
| 11 | `c2Clampv` | each component independently below/inside/above bounds (3×3) | [x] |
| 12 | `c2Sub` | arbitrary vector subtraction | [x] |
| 13 | `c2Dot` | arbitrary vectors, including cancellation and zero | [x] |
| 14 | `c2RotIdentity`, `c2xIdentity` | identity values | [x] |
| 15 | `c2BBVerts` | arbitrary AABB, including reversed/equal bounds | [x] |
| 16 | `c2MakeProxy` | circle: one vertex and radius | [x] |
| 17 | `c2MakeProxy` | AABB: four vertices and zero radius | [x] |
| 18 | `c2MakeProxy` | capsule: two vertices and radius | [x] |
| 19 | `c2Len` | zero vector | [x] |
| 20 | `c2Len` | nonzero arbitrary vector | [x] |
| 21 | `c2Det2` | arbitrary vectors, including collinear and opposite orientation | [x] |
| 22 | `c2GJKSimplexMetric` | simplex count 1 | [x] |
| 23 | `c2GJKSimplexMetric` | simplex count 2 | [x] |
| 24 | `c2GJKSimplexMetric` | simplex count 3 | [x] |
| 25 | `c2Mulrv` | arbitrary rotation coefficients and vector | [x] |
| 26 | `c2Add` | arbitrary vector addition | [x] |
| 27 | `c2Mulxv` | arbitrary transform and vector | [x] |
| 28 | `c22` | `v <= 0` vertex-A region | [x] |
| 29 | `c22` | `v > 0 && u <= 0` vertex-B region | [x] |
| 30 | `c22` | `v > 0 && u > 0` edge region | [x] |
| 31 | `c23` | vertex-A region | [x] |
| 32 | `c23` | vertex-B region | [x] |
| 33 | `c23` | vertex-C region | [x] |
| 34 | `c23` | edge-AB region | [x] |
| 35 | `c23` | edge-BC region | [x] |
| 36 | `c23` | edge-CA region | [x] |
| 37 | `c23` | triangle interior/fallback region | [x] |
| 38 | `c2Neg`, `c2Skew`, `c2CCW90` | arbitrary vector | [x] |
| 39 | `c2D` | count 1 | [x] |
| 40 | `c2D` | count 2 and determinant positive | [x] |
| 41 | `c2D` | count 2 and determinant nonpositive | [x] |
| 42 | `c2D` | count 3/default | [x] |
| 43 | `c2Support` | one vertex | [x] |
| 44 | `c2Support` | many vertices, first is strict maximum | [x] |
| 45 | `c2Support` | many vertices, later vertex is strict maximum | [x] |
| 46 | `c2Support` | many vertices with a maximum tie (first maximum retained) | [x] |
| 47 | `c2Support` | oversized count (> proxy capacity) backed by a sufficiently large array | [x] |
| 48 | `c2Witness` | count 1 | [x] |
| 49 | `c2Witness` | count 2 weighted result | [x] |
| 50 | `c2Witness` | count 3 weighted result | [x] |
| 51 | `c2Div` | nonzero divisor | [x] |
| 52 | `c2Div` | zero divisor (IEEE infinities/NaNs) | [x] |
| 53 | `c2Norm` | nonzero vector | [x] |
| 54 | `c2Norm` | zero vector (IEEE NaNs) | [x] |
| 55 | `c2L` | count 1 | [x] |
| 56 | `c2L` | count 2 weighted result | [x] |
| 57 | `c2MulrvT` | arbitrary rotation coefficients and vector | [x] |
| 58 | `c2GJK` | each ordered shape pair: circle/circle, circle/AABB, circle/capsule, AABB/circle, AABB/AABB, AABB/capsule, capsule/circle, capsule/AABB, capsule/capsule | [x] |
| 59 | `c2GJK` | `ax_ptr`/`bx_ptr` null-null, provided-null, null-provided, provided-provided | [x] |
| 60 | `c2GJK` | `use_radius == 0` and `use_radius != 0`, separated and overlapping shapes | [x] |
| 61 | `c2GJK` | `outA`, `outB`, and `iterations` each null and non-null | [x] |
| 62 | `c2GJK` | cache null | [x] |
| 63 | `c2GJK` | cache present with count zero (cold cache) | [x] |
| 64 | `c2GJK` | cache reused from a preceding identical call (warm cache) | [x] |
| 65 | `c2GJK` | cache count 1, 2, and 3 metric paths with valid indices | [x] |
| 66 | `c2GJK` | loop termination by simplex count 3 (hit) | [x] |
| 67 | `c2GJK` | loop termination by duplicate support point / tiny direction / distance non-improvement | [x] |
| 68 | `c2GJK` | radius branch `dist > rA+rB && dist > FLT_EPSILON` | [x] |
| 69 | `c2GJK` | radius overlap/near-zero branch, witnesses collapsed to midpoint | [x] |
| 70 | `c2AABBtoAABB` | separated on each of four sides | [x] |
| 71 | `c2AABBtoAABB` | touching and overlapping | [x] |
| 72 | `c2AABBtoCapsule` | separated and collided | [x] |
| 73 | `c2CapsuletoCapsule` | separated and collided | [x] |
| 74 | `c2CircletoCircle` | separated, exactly tangent, and overlapping | [x] |
| 75 | `c2CircletoAABB` | center outside/inside; separated, exactly tangent, overlapping | [x] |
| 76 | `c2CircletoCapsule` | projection before A (`da < 0`) | [x] |
| 77 | `c2CircletoCapsule` | projection on segment (`da >= 0 && db < 0`) | [x] |
| 78 | `c2CircletoCapsule` | projection after B (`db >= 0`) | [x] |
| 79 | `c2Collided` | all 9 ordered valid type pairs | [x] |
| 80 | `aabb` | arbitrary input AABB coordinates, including reversed/equal bounds | [x] |

