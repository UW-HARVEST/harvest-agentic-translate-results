# Configuration surface

There are no Cargo features, C preprocessor feature flags, or binary targets.
The only build configuration is the default library build. Runtime axes come
from enum modes, optional pointers, count fields, strict boundary comparisons,
and geometric input shape.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | arbitrary finite/IEEE x and y scalars | [x] |
| 2 | `c2Mulvs` | vector times positive, negative, zero, and fractional scalar | [x] |
| 3 | `c2Add`, `c2Sub` | arbitrary vectors | [x] |
| 4 | `c2Dot`, `c2Det2` | parallel, perpendicular, and arbitrary vectors | [x] |
| 5 | `c2Maxv`, `c2Minv` | each component selected from either operand, including ties | [x] |
| 6 | `c2Clampv` | below, inside, and above bounds independently on each axis | [x] |
| 7 | `c2Len` | zero and nonzero vectors | [x] |
| 8 | `c2Div` | positive, negative, fractional, and zero divisor | [x] |
| 9 | `c2Norm` | nonzero and zero-length vectors | [x] |
| 10 | `c2Neg`, `c2Absv`, `c2CCW90`, `c2Skew` | positive, negative, zero, and signed-zero components | [x] |
| 11 | `c2Dist` | points on, behind, and in front of plane | [x] |
| 12 | `c2PlaneAt` | valid polygon indices for polygons with 1, 2, 4, and 8 vertices | [x] |
| 13 | `c2RotIdentity`, `c2xIdentity` | identity constructors | [x] |
| 14 | `c2Mulrv`, `c2MulrvT` | identity and arbitrary cosine/sine pair | [x] |
| 15 | `c2Mulxv`, `c2MulxvT` | identity and translated/rotated transforms | [x] |
| 16 | `c2BBVerts` | ordinary, zero-area, and inverted AABBs | [x] |
| 17 | `c2MakeProxy` | circle: one vertex plus radius | [x] |
| 18 | `c2MakeProxy` | AABB: four corner vertices and zero radius | [x] |
| 19 | `c2MakeProxy` | capsule: two vertices plus radius | [x] |
| 20 | `c2MakeProxy` | poly/out-of-range type: no switch arm | [x] |
| 21 | `c2GJKSimplexMetric` | simplex count 0/1/default | [x] |
| 22 | `c2GJKSimplexMetric` | simplex count 2 (segment length) | [x] |
| 23 | `c2GJKSimplexMetric` | simplex count 3 (signed determinant) | [x] |
| 24 | `c2Intersect` | crossing distances, endpoint-on-plane, and equal distances | [x] |
| 25 | `c22` | Voronoi region A (`v <= 0`) | [x] |
| 26 | `c22` | Voronoi region B (`v > 0 && u <= 0`) | [x] |
| 27 | `c22` | segment interior (`u > 0 && v > 0`) | [x] |
| 28 | `c23` | vertex-A region | [x] |
| 29 | `c23` | vertex-B region | [x] |
| 30 | `c23` | vertex-C region | [x] |
| 31 | `c23` | edge-AB region | [x] |
| 32 | `c23` | edge-BC region | [x] |
| 33 | `c23` | edge-CA region | [x] |
| 34 | `c23` | triangle interior/default region | [x] |
| 35 | `c2D` | simplex count 1 | [x] |
| 36 | `c2D` | simplex count 2 with positive determinant branch | [x] |
| 37 | `c2D` | simplex count 2 with non-positive determinant branch | [x] |
| 38 | `c2D` | count 3/default | [x] |
| 39 | `c2Support` | one vertex | [x] |
| 40 | `c2Support` | many vertices with unique maximum | [x] |
| 41 | `c2Support` | tied maximum (strict `>` preserves first index) | [x] |
| 42 | `c2Witness` | simplex count 1 | [x] |
| 43 | `c2Witness` | simplex count 2 | [x] |
| 44 | `c2Witness` | simplex count 3 | [x] |
| 45 | `c2Witness` | default count | [x] |
| 46 | `c2L` | simplex count 1 | [x] |
| 47 | `c2L` | simplex count 2 | [x] |
| 48 | `c2L` | default count | [x] |
| 49 | `c2GJK` | circle-circle, no transforms, `use_radius = 0`, no cache, all optional outputs present | [x] |
| 50 | `c2GJK` | circle-circle, `use_radius != 0`, separated/tangent/overlapping centers | [x] |
| 51 | `c2GJK` | circle-AABB and AABB-circle shape order | [x] |
| 52 | `c2GJK` | circle-capsule and capsule-circle shape order | [x] |
| 53 | `c2GJK` | AABB-AABB | [x] |
| 54 | `c2GJK` | AABB-capsule and capsule-AABB shape order | [x] |
| 55 | `c2GJK` | capsule-capsule | [x] |
| 56 | `c2GJK` | explicit identity vs non-identity A/B transforms | [x] |
| 57 | `c2GJK` | optional output pointers independently NULL | [x] |
| 58 | `c2GJK` | empty cache, reusable populated cache, and rejected cache predicate | [x] |
| 59 | `c2GJK` | each termination family: triangle hit, increased distance, tiny direction, duplicate support, iteration cap where reachable | [x] |
| 60 | `c2CircletoCircleManifold` | separated, tangent, overlapping, and coincident centers | [x] |
| 61 | `c2CircletoAABBManifold` | separated/tangent; outside overlap (`d2 != 0`) | [x] |
| 62 | `c2CircletoAABBManifold` | center inside/on box (`d2 == 0`), x overlap selected | [x] |
| 63 | `c2CircletoAABBManifold` | center inside/on box (`d2 == 0`), y overlap selected | [x] |
| 64 | `c2CircletoCapsuleManifold` | separated, tangent, overlap with nonzero GJK distance, and zero-distance fallback normal | [x] |
| 65 | `c2AABBtoAABBManifold` | separated on x, separated on y, tangent, overlap with x minimum, overlap with y minimum | [x] |
| 66 | `c2AABBtoAABBManifold` | relative center on negative/positive side for each chosen axis | [x] |
| 67 | `c2CapsuletoPolyManifold` | `bx_ptr == NULL` and explicit identity transform | [x] |
| 68 | `c2CapsuletoPolyManifold` | transformed polygon | [x] |
| 69 | `c2CapsuletoPolyManifold` | GJK `d < 1e-6`, reference face code 0 | [x] |
| 70 | `c2CapsuletoPolyManifold` | GJK `d < 1e-6`, capsule side code 1 | [x] |
| 71 | `c2CapsuletoPolyManifold` | GJK `d < 1e-6`, capsule side code 2 | [x] |
| 72 | `c2CapsuletoPolyManifold` | `1e-6 <= d < A.r` single-contact branch | [x] |
| 73 | `c2CapsuletoPolyManifold` | `d >= A.r` no-contact branch | [x] |
| 74 | `c2Norms` | counts 0, 1, 2, 4, and 8 with wraparound edge | [x] |
| 75 | `c2AABBtoCapsuleManifold` | separated, tangent, edge overlap, corner overlap, and capsule inside box | [x] |
| 76 | `c2CapsuletoCapsuleManifold` | separated, tangent, overlap with nonzero distance, and zero-distance fallback normal | [x] |
| 77 | `c2Collide` | all nine ordered pairs of circle/AABB/capsule | [x] |
| 78 | `c2Collide` | unsupported type on A or B | [x] |
| 79 | `ptr_from_parts` | circle field mapping | [x] |
| 80 | `ptr_from_parts` | AABB field mapping | [x] |
| 81 | `ptr_from_parts` | capsule field mapping | [x] |
| 82 | `omni_manifold` | all nine ordered pairs of circle/AABB/capsule, randomized parameters | [x] |
| 83 | `omni_manifold` | unsupported type on A or B | [x] |

