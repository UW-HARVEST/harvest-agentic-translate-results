# Configuration Surface

`Cargo.toml` has no `[features]` table and CMake has no options or conditional
definitions. The complete build-time matrix is one combination:
`--no-default-features` (empty feature set).

The rows below come from the public exported functions and their C
`if`/`switch` branches. Randomized finite values are used unless a boundary or
special shape is named explicitly.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| C001 | `c2V`, `c2Mulvs`, `c2Add`, `c2Sub` | arbitrary finite vectors/scalars | [x] |
| C002 | `c2Maxv`, `c2Minv`, `c2Clampv` | each component below its comparator/bounds | [x] |
| C003 | `c2Maxv`, `c2Minv`, `c2Clampv` | each component equal to its comparator/bounds | [x] |
| C004 | `c2Maxv`, `c2Minv`, `c2Clampv` | each component above its comparator/bounds | [x] |
| C005 | `c2Dot`, `c2Len`, `c2Det2`, `c2Absv`, `c2Neg`, `c2CCW90`, `c2Skew` | arbitrary finite vectors, including mixed signs | [x] |
| C006 | `c2Dist`, `c2PlaneAt` | polygon edge indices from zero through `count-1` | [x] |
| C007 | `c2RotIdentity`, `c2xIdentity` | no input | [x] |
| C008 | `c2BBVerts` | arbitrary ordered AABB bounds | [x] |
| C009 | `c2MakeProxy` | circle: one vertex and nonzero radius | [x] |
| C010 | `c2MakeProxy` | AABB: four vertices and zero radius | [x] |
| C011 | `c2MakeProxy` | capsule: two vertices and nonzero radius | [x] |
| C012 | `c2GJKSimplexMetric` | simplex count 1 or other/default | [x] |
| C013 | `c2GJKSimplexMetric` | simplex count 2 (segment length) | [x] |
| C014 | `c2GJKSimplexMetric` | simplex count 3 (signed determinant) | [x] |
| C015 | `c2Mulrv`, `c2MulrvT` | identity rotation | [x] |
| C016 | `c2Mulrv`, `c2MulrvT` | arbitrary sine/cosine pair | [x] |
| C017 | `c2Mulxv`, `c2MulxvT` | arbitrary rotation and translation | [x] |
| C018 | `c2Intersect` | segment endpoints on opposite sides (`da*db < 0`) | [x] |
| C019 | `c2Intersect` | one endpoint on plane (`da == 0` or `db == 0`) | [x] |
| C020 | `c2Div` | arbitrary finite nonzero divisor | [x] |
| C021 | `c2Norm` | arbitrary nonzero vector | [x] |
| C022 | `c2Norm` | zero vector, producing the C floating-point special values | [x] |
| C023 | `c22` | `v <= 0`, simplex reduces to vertex A | [x] |
| C024 | `c22` | `v > 0 && u <= 0`, simplex reduces to vertex B | [x] |
| C025 | `c22` | `u > 0 && v > 0`, simplex remains an edge | [x] |
| C026 | `c23` | A Voronoi region (`vAB <= 0 && uCA <= 0`) | [x] |
| C027 | `c23` | B Voronoi region (`uAB <= 0 && vBC <= 0`) | [x] |
| C028 | `c23` | C Voronoi region (`uBC <= 0 && vCA <= 0`) | [x] |
| C029 | `c23` | AB edge region | [x] |
| C030 | `c23` | BC edge region | [x] |
| C031 | `c23` | CA edge region | [x] |
| C032 | `c23` | triangle interior | [x] |
| C033 | `c2D` | simplex count 1 | [x] |
| C034 | `c2D` | simplex count 2 and positive determinant branch | [x] |
| C035 | `c2D` | simplex count 2 and nonpositive determinant branch | [x] |
| C036 | `c2D` | simplex count 3/default | [x] |
| C037 | `c2Support` | one vertex | [x] |
| C038 | `c2Support` | two through eight vertices with a unique maximum | [x] |
| C039 | `c2Support` | tied maxima; strict `>` preserves first index | [x] |
| C040 | `c2Witness` | simplex count 1 | [x] |
| C041 | `c2Witness` | simplex count 2 | [x] |
| C042 | `c2Witness` | simplex count 3 | [x] |
| C043 | `c2Witness` | default/other simplex count | [x] |
| C044 | `c2L` | simplex count 1 | [x] |
| C045 | `c2L` | simplex count 2 | [x] |
| C046 | `c2L` | simplex count 3/default | [x] |
| C047 | `c2GJK` | circle-circle, identity transforms, `use_radius=0` | [x] |
| C048 | `c2GJK` | circle-AABB, identity transforms, `use_radius=0` | [x] |
| C049 | `c2GJK` | circle-capsule, identity transforms, `use_radius=0` | [x] |
| C050 | `c2GJK` | AABB-circle, identity transforms, `use_radius=0` | [x] |
| C051 | `c2GJK` | AABB-AABB, identity transforms, `use_radius=0` | [x] |
| C052 | `c2GJK` | AABB-capsule, identity transforms, `use_radius=0` | [x] |
| C053 | `c2GJK` | capsule-circle, identity transforms, `use_radius=0` | [x] |
| C054 | `c2GJK` | capsule-AABB, identity transforms, `use_radius=0` | [x] |
| C055 | `c2GJK` | capsule-capsule, identity transforms, `use_radius=0` | [x] |
| C056 | `c2GJK` | all supported shape pairs, non-null arbitrary transforms | [x] |
| C057 | `c2GJK` | all supported shape pairs, `use_radius=1`, separated beyond radii | [x] |
| C058 | `c2GJK` | all supported shape pairs, `use_radius=1`, touching/overlapping radii | [x] |
| C059 | `c2GJK` | null cache versus non-null zero-count cache | [x] |
| C060 | `c2GJK` | warm cache from a prior call, including metric validation/read path | [x] |
| C061 | `c2GJK` | each optional output pointer (`outA`, `outB`, `iterations`) null and non-null | [x] |
| C062 | `c2CircletoCircleManifold` | separated, exactly tangent, overlapping distinct centers, coincident centers | [x] |
| C063 | `c2CircletoAABBManifold` | separated/tangent; overlapping outside; center inside with x/y minimum overlap | [x] |
| C064 | `c2CircletoCapsuleManifold` | separated/tangent; overlap at side/end; zero-distance branch | [x] |
| C065 | `c2AABBtoAABBManifold` | x/y separated; x/y minimum penetration; signed normal branches; touching | [x] |
| C066 | `c2Norms` | counts 1, 2, and 3 through 8 | [x] |
| C067 | `c2Norms` | count zero (loop performs no writes) | [x] |
| C068 | `c2CapsuletoPolyManifold` | polygon counts 3 through 8, null identity transform, reference-face path | N/A: C UB |
| C069 | `c2CapsuletoPolyManifold` | polygon counts 3 through 8, non-null transform, capsule-face paths | N/A: C UB |
| C070 | `c2CapsuletoPolyManifold` | shallow contact (`1e-6 <= d < radius`) | N/A: C UB |
| C071 | `c2CapsuletoPolyManifold` | separated/no-contact path | N/A: C UB |
| C072 | `c2AABBtoCapsuleManifold` | separated/tangent/overlapping, horizontal and vertical capsules | N/A: C UB |
| C073 | `c2CapsuletoCapsuleManifold` | separated/tangent/overlap, parallel/crossing/degenerate segments | [x] |
| C074 | `c2Collide` | circle-circle | [x] |
| C075 | `c2Collide` | circle-AABB and reversed AABB-circle normal | [x] |
| C076 | `c2Collide` | circle-capsule and reversed capsule-circle normal | [x] |
| C077 | `c2Collide` | AABB-AABB | [x] |
| C078 | `c2Collide` | AABB-capsule and reversed capsule-AABB normal | N/A: C UB |
| C079 | `c2Collide` | capsule-capsule | [x] |
| C080 | `ptr_from_parts` | circle allocation/layout | [x] |
| C081 | `ptr_from_parts` | AABB allocation/layout | [x] |
| C082 | `ptr_from_parts` | capsule allocation/layout | [x] |
| C083 | `omni_manifold` | circle-circle | [x] |
| C084 | `omni_manifold` | circle-AABB and AABB-circle | [x] |
| C085 | `omni_manifold` | circle-capsule and capsule-circle | [x] |
| C086 | `omni_manifold` | AABB-AABB | [x] |
| C087 | `omni_manifold` | AABB-capsule and capsule-AABB | N/A: C UB |
| C088 | `omni_manifold` | capsule-capsule | [x] |

Rows C068-C072, C078, and C087 all enter the same C-undefined path:
`c2GJK` requests a `C2_TYPE_POLY` proxy, while `c2MakeProxy` leaves that proxy
uninitialized. MemorySanitizer reports the first uninitialized use at
`c_src/src/lib.c:506`; the default C build also segfaulted during randomized
calls. They cannot have byte-identical differential assertions without
modifying the ground-truth C source.
