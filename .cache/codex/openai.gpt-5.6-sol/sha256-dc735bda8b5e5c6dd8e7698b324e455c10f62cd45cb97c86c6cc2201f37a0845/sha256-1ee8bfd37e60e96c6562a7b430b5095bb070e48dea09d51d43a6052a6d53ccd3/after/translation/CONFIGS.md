# Configuration Surface

Rows are derived from every exported C entry point and every branch on runtime
state, shape type, simplex count, optional pointer, and geometric relation.
Randomized cases include finite values, signed zero, and representable boundary
values where the operation is defined.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---:|----------------|-------------------------------------------|----------|
| 1 | `c2V` | arbitrary scalar pair | [x] |
| 2 | `c2Mulvs` | arbitrary vector and scalar | [x] |
| 3 | `c2Maxv` | each component selected from either operand, including equality | [x] |
| 4 | `c2Minv` | each component selected from either operand, including equality | [x] |
| 5 | `c2Clampv` | components below, inside, and above bounds | [x] |
| 6 | `c2Sub` | arbitrary vector pair | [x] |
| 7 | `c2Dot` | arbitrary vector pair | [x] |
| 8 | `c2RotIdentity` | no inputs | [x] |
| 9 | `c2xIdentity` | no inputs | [x] |
| 10 | `c2BBVerts` | normalized, degenerate, and inverted AABB extents | [x] |
| 11 | `c2MakeProxy` | circle shape (one vertex, radius retained) | [x] |
| 12 | `c2MakeProxy` | AABB shape (four vertices, zero radius) | [x] |
| 13 | `c2MakeProxy` | capsule shape (two vertices, radius retained) | [x] |
| 14 | `c2Len` | zero and nonzero vectors | [x] |
| 15 | `c2Det2` | positive, negative, and zero determinant | [x] |
| 16 | `c2GJKSimplexMetric` | simplex count 1 | [x] |
| 17 | `c2GJKSimplexMetric` | simplex count 2 | [x] |
| 18 | `c2GJKSimplexMetric` | simplex count 3 | [x] |
| 19 | `c2Mulrv` | identity and arbitrary rotation coefficients | [x] |
| 20 | `c2Add` | arbitrary vector pair | [x] |
| 21 | `c2Mulxv` | identity and translated/rotated transforms | [x] |
| 22 | `c22` | vertex-A region (`v <= 0`) | [x] |
| 23 | `c22` | vertex-B region (`u <= 0`) | [x] |
| 24 | `c22` | edge-AB region (`u > 0 && v > 0`) | [x] |
| 25 | `c23` | vertex-A region | [x] |
| 26 | `c23` | vertex-B region | [x] |
| 27 | `c23` | vertex-C region | [x] |
| 28 | `c23` | edge-AB region | [x] |
| 29 | `c23` | edge-BC region | [x] |
| 30 | `c23` | edge-CA region | [x] |
| 31 | `c23` | triangle interior region | [x] |
| 32 | `c2Neg` | arbitrary vector | [x] |
| 33 | `c2Skew` | arbitrary vector | [x] |
| 34 | `c2CCW90` | arbitrary vector | [x] |
| 35 | `c2D` | simplex count 1 | [x] |
| 36 | `c2D` | simplex count 2, positive determinant branch | [x] |
| 37 | `c2D` | simplex count 2, nonpositive determinant branch | [x] |
| 38 | `c2D` | simplex count 3 | [x] |
| 39 | `c2Support` | one vertex | [x] |
| 40 | `c2Support` | 2 through 8 vertices with unique maximum | [x] |
| 41 | `c2Support` | 2 through 8 vertices with tied maximum (first wins) | [x] |
| 42 | `c2Witness` | simplex count 1 | [x] |
| 43 | `c2Witness` | simplex count 2 with barycentric weights | [x] |
| 44 | `c2Witness` | simplex count 3 with barycentric weights | [x] |
| 45 | `c2Div` | nonzero divisor | [x] |
| 46 | `c2Norm` | nonzero vector | [x] |
| 47 | `c2L` | simplex count 1 | [x] |
| 48 | `c2L` | simplex count 2 with barycentric weights | [x] |
| 49 | `c2L` | simplex count 3 | [x] |
| 50 | `c2MulrvT` | identity and arbitrary rotation coefficients | [x] |
| 51 | `c2GJK` | circle-circle, identity transforms, radius disabled, cold/no cache | [x] |
| 52 | `c2GJK` | circle-AABB, identity transforms, radius disabled, cold/no cache | [x] |
| 53 | `c2GJK` | circle-capsule, identity transforms, radius disabled, cold/no cache | [x] |
| 54 | `c2GJK` | AABB-circle, identity transforms, radius disabled, cold/no cache | [x] |
| 55 | `c2GJK` | AABB-AABB, identity transforms, radius disabled, cold/no cache | [x] |
| 56 | `c2GJK` | AABB-capsule, identity transforms, radius disabled, cold/no cache | [x] |
| 57 | `c2GJK` | capsule-circle, identity transforms, radius disabled, cold/no cache | [x] |
| 58 | `c2GJK` | capsule-AABB, identity transforms, radius disabled, cold/no cache | [x] |
| 59 | `c2GJK` | capsule-capsule, identity transforms, radius disabled, cold/no cache | [x] |
| 60 | `c2GJK` | all nine ordered shape pairs, explicit transforms, radius enabled, cold cache, all outputs | [x] |
| 61 | `c2GJK` | all nine ordered shape pairs, explicit transforms, radius enabled, warm cache reused | [x] |
| 62 | `c2GJK` | optional witness, iteration, and cache pointers all null | [x] |
| 63 | `c2GJK` | separated shapes (`dist > rA + rB` and epsilon) | [x] |
| 64 | `c2GJK` | radius overlap/touch branch (`dist <= rA + rB` or epsilon) | [x] |
| 65 | `c2AABBtoAABB` | overlap, edge touch, corner touch, x/y separation | [x] |
| 66 | `c2AABBtoCapsule` | overlap/touch and separation | [x] |
| 67 | `c2CapsuletoCapsule` | overlap/touch and separation | [x] |
| 68 | `c2CircletoCircle` | overlap, exact tangent, and separation | [x] |
| 69 | `c2CircletoAABB` | center inside, edge/corner overlap, exact tangent, separation | [x] |
| 70 | `c2CircletoCapsule` | closest to endpoint A, segment interior, endpoint B; overlap/tangent/separation | [x] |
| 71 | `c2Collided` | all nine ordered valid shape-type pairs, overlap/touch/separation | [x] |
| 72 | `aabb` | normalized AABBs across the three fixed-object collision regions | [x] |
| 73 | `aabb` | degenerate (zero width and/or height) AABBs | [x] |
| 74 | `aabb` | inverted min/max AABBs | [x] |
| 75 | scalar/vector helpers, `aabb` | signed zero, subnormal, infinity, and distinct signed NaN payloads | [x] |

Cargo declares no features. The complete build matrix is therefore the default
configuration and `--no-default-features`; both compile the same source paths
but are verified independently.
