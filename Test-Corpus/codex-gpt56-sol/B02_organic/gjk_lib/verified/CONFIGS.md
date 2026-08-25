# Configuration Surface

## Build-Time Matrix

`Cargo.toml` has no `[features]` table, implicit optional-dependency features,
or default features. `c_src/CMakeLists.txt` has no `option`, conditional,
compile-definition, platform, or backend branch. There is exactly one valid
build-time configuration:

| # | Cargo invocation | CMake configuration | Status |
|---|------------------|---------------------|--------|
| F1 | `--no-default-features` with no named features | default, position-independent shared library linked with `libm` | [x] |

## Runtime Configuration Matrix

Each row comes from an exported C symbol and the `if`, ternary, `switch`, loop,
or fixed input shape that its implementation distinguishes. "Random finite"
includes positive, negative, zero, sub-unit, and boundary-near values. Rows
whose configuration is a cross-product require every listed member, not one
representative.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|--------------------------------------------|--------|
| 1 | `c2V` | random finite scalar pair | [x] |
| 2 | `c2Mulvs` | random vector and scalar, including zero and negative scalar | [x] |
| 3 | `c2Maxv` | random vectors covering all four independent x/y greater-than outcomes and equality | [x] |
| 4 | `c2Minv` | random vectors covering all four independent x/y less-than outcomes and equality | [x] |
| 5 | `c2Clampv` | values below, within, and above each independent component interval | [x] |
| 6 | `c2Sub` | random finite vector pair | [x] |
| 7 | `c2Dot` | random finite vector pair, including orthogonal and zero vectors | [x] |
| 8 | `c2RotIdentity` | no-input identity result | [x] |
| 9 | `c2xIdentity` | no-input identity transform | [x] |
| 10 | `c2BBVerts` | random AABB minima/maxima; four output vertices and order | [x] |
| 11 | `c2MakeProxy` | type 0 circle: one vertex and radius | [x] |
| 12 | `c2MakeProxy` | type 1 AABB: four ordered vertices and zero radius | [x] |
| 13 | `c2MakeProxy` | type 2 capsule: two vertices and radius | [x] |
| 14 | `c2Len` | random finite vectors, zero, axis-aligned, and diagonal | [x] |
| 15 | `c2Det2` | random finite vector pair, parallel and perpendicular | [x] |
| 16 | `c2GJKSimplexMetric` | simplex count 1: zero metric | [x] |
| 17 | `c2GJKSimplexMetric` | simplex count 2: segment length | [x] |
| 18 | `c2GJKSimplexMetric` | simplex count 3: signed triangle determinant | [x] |
| 19 | `c2Mulrv` | random rotation coefficients and vector | [x] |
| 20 | `c2Add` | random finite vector pair | [x] |
| 21 | `c2Mulxv` | random translation, rotation coefficients, and vector | [x] |
| 22 | `c22` | `v <= 0`: simplex reduces to vertex A | [x] |
| 23 | `c22` | `v > 0 && u <= 0`: simplex reduces to vertex B | [x] |
| 24 | `c22` | `v > 0 && u > 0`: two-vertex simplex with barycentric weights | [x] |
| 25 | `c23` | `vAB <= 0 && uCA <= 0`: vertex-A Voronoi region | [x] |
| 26 | `c23` | `uAB <= 0 && vBC <= 0`: vertex-B Voronoi region | [x] |
| 27 | `c23` | `uBC <= 0 && vCA <= 0`: vertex-C Voronoi region | [x] |
| 28 | `c23` | `uAB > 0 && vAB > 0 && wABC <= 0`: edge-AB region | [x] |
| 29 | `c23` | `uBC > 0 && vBC > 0 && uABC <= 0`: edge-BC region | [x] |
| 30 | `c23` | `uCA > 0 && vCA > 0 && vABC <= 0`: edge-CA region | [x] |
| 31 | `c23` | all prior conditions false: triangle interior | [x] |
| 32 | `c2Neg` | random finite vector | [x] |
| 33 | `c2Skew` | random finite vector | [x] |
| 34 | `c2CCW90` | random finite vector | [x] |
| 35 | `c2D` | count 1 | [x] |
| 36 | `c2D` | count 2 and determinant greater than zero | [x] |
| 37 | `c2D` | count 2 and determinant less than or equal to zero | [x] |
| 38 | `c2D` | count 3/default zero direction | [x] |
| 39 | `c2Support` | one vertex | [x] |
| 40 | `c2Support` | many vertices: first maximum, later strict maximum, and equal-dot tie retaining first | [x] |
| 41 | `c2Witness` | count 1 | [x] |
| 42 | `c2Witness` | count 2 with random nonzero divisor and weights | [x] |
| 43 | `c2Witness` | count 3 with random nonzero divisor and weights | [x] |
| 44 | `c2Div` | random vector and finite nonzero divisor | [x] |
| 45 | `c2Norm` | random nonzero vector | [x] |
| 46 | `c2L` | count 1 | [x] |
| 47 | `c2L` | count 2 with random nonzero divisor and weights | [x] |
| 48 | `c2MulrvT` | random rotation coefficients and vector | [x] |
| 49 | `c2GJK` | circle/circle. Cross product: identity/explicit transform independently for A/B; radius off/nonzero; each optional output independently null/non-null; cache null/empty/warm; separated/touching/overlapping geometry | [x] |
| 50 | `c2GJK` | circle/AABB with the full option, cache, output, transform, and geometry cross-product from row 49 | [x] |
| 51 | `c2GJK` | circle/capsule with the full option, cache, output, transform, and geometry cross-product from row 49 | [x] |
| 52 | `c2GJK` | AABB/circle with the full option, cache, output, transform, and geometry cross-product from row 49 | [x] |
| 53 | `c2GJK` | AABB/AABB with the full option, cache, output, transform, and geometry cross-product from row 49 | [x] |
| 54 | `c2GJK` | AABB/capsule with the full option, cache, output, transform, and geometry cross-product from row 49 | [x] |
| 55 | `c2GJK` | capsule/circle with the full option, cache, output, transform, and geometry cross-product from row 49 | [x] |
| 56 | `c2GJK` | capsule/AABB with the full option, cache, output, transform, and geometry cross-product from row 49 | [x] |
| 57 | `c2GJK` | capsule/capsule with the full option, cache, output, transform, and geometry cross-product from row 49 | [x] |
| 58 | `c2GJK` | cache reuse with count 1, 2, and 3; cache accepted and source rejection predicate `!(min_metric < max_metric * 2 && metric < -1e8)` exercised | [x] |
| 59 | `c2GJK` | loop exits: simplex hit, increasing squared distance, tiny direction, duplicate support, and 20-iteration cap when reachable | [x] |
| 60 | `c2GJK` | radius branch: separated beyond radius sum, touching/overlapping radius sum, and post-adjustment equal witnesses | [x] |
| 61 | `gjk` | `reverse == 0`: AABB is shape A, capsule is shape B, radius enabled | [x] |
| 62 | `gjk` | `reverse != 0`: capsule is shape A, AABB is shape B, radius enabled | [x] |

The C ABI has no byte-order, serialized format, string, variable-width element,
or allocation mode. All scalar floating inputs are native IEEE-754 `float`,
all counts/enums are native C `int`, and the wrapper flag is native C `char`.
