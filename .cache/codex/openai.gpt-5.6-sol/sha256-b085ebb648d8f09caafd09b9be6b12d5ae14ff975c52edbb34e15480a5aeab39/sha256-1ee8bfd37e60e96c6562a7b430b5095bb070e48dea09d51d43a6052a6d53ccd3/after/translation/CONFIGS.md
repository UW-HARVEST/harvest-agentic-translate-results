# Configuration Surface

Rows are derived from every `switch`, option, shape dispatch, and
value-dependent branch in `src/lib.c`. "Randomized" means deterministic
fixed-seed samples including zeros, signs, ties, degeneracies, and finite
boundary values.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---:|----------------|--------------------------------------------|:---:|
| 1 | `c2V` | Randomized scalar pairs | [x] |
| 2 | `c2Mulvs` | Randomized vectors and zero/positive/negative scalars | [x] |
| 3 | `c2Maxv` | Per-axis comparison cross-product: `<`/`>=` including ties | [x] |
| 4 | `c2Minv` | Per-axis comparison cross-product: `<`/`>=` including ties | [x] |
| 5 | `c2Clampv` | Per-axis below/inside/above cross-product (9 regions), including equal and inverted bounds | [x] |
| 6 | `c2Sub` | Randomized vectors | [x] |
| 7 | `c2Dot` | Randomized vectors including orthogonal and zero vectors | [x] |
| 8 | `c2RotIdentity` | No-input identity constructor | [x] |
| 9 | `c2xIdentity` | No-input identity constructor | [x] |
| 10 | `c2BBVerts` | Normal, degenerate, and inverted AABBs | [x] |
| 11 | `c2MakeProxy` | Circle: one vertex and shape radius | [x] |
| 12 | `c2MakeProxy` | AABB: four generated vertices and zero radius | [x] |
| 13 | `c2MakeProxy` | Capsule: two vertices and shape radius | [x] |
| 14 | `c2Len` | Zero and randomized vectors | [x] |
| 15 | `c2Det2` | Negative, zero, and positive determinants | [x] |
| 16 | `c2GJKSimplexMetric` | `count == 1` | [x] |
| 17 | `c2GJKSimplexMetric` | `count == 2` | [x] |
| 18 | `c2GJKSimplexMetric` | `count == 3`, both winding signs and degenerate triangles | [x] |
| 19 | `c2Mulrv` | Identity and randomized rotation records/vectors | [x] |
| 20 | `c2Add` | Randomized vectors | [x] |
| 21 | `c2Mulxv` | Identity and randomized transform records/vectors | [x] |
| 22 | `c22` | `v <= 0`: origin lies in vertex-A region | [x] |
| 23 | `c22` | `v > 0 && u <= 0`: origin lies in vertex-B region | [x] |
| 24 | `c22` | `v > 0 && u > 0`: origin lies on edge-AB region | [x] |
| 25 | `c23` | Vertex-A region: `vAB <= 0 && uCA <= 0` | [x] |
| 26 | `c23` | Vertex-B region: `uAB <= 0 && vBC <= 0` | [x] |
| 27 | `c23` | Vertex-C region: `uBC <= 0 && vCA <= 0` | [x] |
| 28 | `c23` | Edge-AB region | [x] |
| 29 | `c23` | Edge-BC region | [x] |
| 30 | `c23` | Edge-CA region | [x] |
| 31 | `c23` | Triangle interior/fallback region | [x] |
| 32 | `c2Neg` | Zero and randomized vectors | [x] |
| 33 | `c2Skew` | Zero and randomized vectors | [x] |
| 34 | `c2CCW90` | Zero and randomized vectors | [x] |
| 35 | `c2D` | `count == 1` | [x] |
| 36 | `c2D` | `count == 2`, determinant positive | [x] |
| 37 | `c2D` | `count == 2`, determinant non-positive | [x] |
| 38 | `c2D` | `count == 3` | [x] |
| 39 | `c2Support` | One vertex | [x] |
| 40 | `c2Support` | 2 through 8 vertices; maximum first/middle/last and tied maxima (first wins) | [x] |
| 41 | `c2Witness` | `count == 1` | [x] |
| 42 | `c2Witness` | `count == 2`, randomized positive barycentric weights/divisor | [x] |
| 43 | `c2Witness` | `count == 3`, randomized positive barycentric weights/divisor | [x] |
| 44 | `c2Div` | Positive/negative/zero divisors and randomized vectors | [x] |
| 45 | `c2Norm` | Nonzero, axis-aligned, and zero vectors | [x] |
| 46 | `c2L` | `count == 1` | [x] |
| 47 | `c2L` | `count == 2`, randomized positive barycentric weights/divisor | [x] |
| 48 | `c2MulrvT` | Identity and randomized rotation records/vectors | [x] |
| 49 | `c2GJK` | Capsule/Capsule; cross product of identity/custom transforms for A/B, `use_radius` zero/nonzero, null/empty/warm cache, and all null/present output-pointer combinations | [x] |
| 50 | `c2GJK` | Capsule/Circle; same complete option cross-product | [x] |
| 51 | `c2GJK` | Capsule/AABB; same complete option cross-product | [x] |
| 52 | `c2GJK` | Circle/Capsule; same complete option cross-product | [x] |
| 53 | `c2GJK` | Circle/Circle; same complete option cross-product | [x] |
| 54 | `c2GJK` | Circle/AABB; same complete option cross-product | [x] |
| 55 | `c2GJK` | AABB/Capsule; same complete option cross-product | [x] |
| 56 | `c2GJK` | AABB/Circle; same complete option cross-product | [x] |
| 57 | `c2GJK` | AABB/AABB; same complete option cross-product | [x] |
| 58 | `c2GJK` | Valid cache rejected by the `metric < -1.0e8f` freshness branch, then cold-started | [x] |
| 59 | `c2AABBtoAABB` | Overlap, edge/corner touch, and separation in each of four directional tests | [x] |
| 60 | `c2AABBtoCapsule` | GJK radius-adjusted hit and miss, including tangent/degenerate capsule | [x] |
| 61 | `c2CapsuletoCapsule` | GJK radius-adjusted hit and miss, including tangent/degenerate capsules | [x] |
| 62 | `c2CircletoCircle` | Overlap, exact tangent, separate, and concentric circles | [x] |
| 63 | `c2CircletoAABB` | Center below/inside/above each axis (9 regions), overlap/tangent/separate | [x] |
| 64 | `c2CircletoCapsule` | `da < 0` endpoint-A region, overlap/tangent/separate | [x] |
| 65 | `c2CircletoCapsule` | `da >= 0 && db < 0` segment-interior region, overlap/tangent/separate | [x] |
| 66 | `c2CircletoCapsule` | `da >= 0 && db >= 0` endpoint-B region, overlap/tangent/separate | [x] |
| 67 | `c2Collided` | All 9 ordered valid shape-type pairs, exercising direct and argument-swapped dispatch | [x] |
| 68 | `ptr_from_parts` | Capsule layout: `(a,b)`, `(c,d)`, radius `e` | [x] |
| 69 | `ptr_from_parts` | Circle layout: center `(a,b)`, radius `c`; ignored `d,e` | [x] |
| 70 | `ptr_from_parts` | AABB layout: min `(a,b)`, max `(c,d)`; ignored `e` | [x] |
| 71 | `omni_collide` | All 9 ordered valid type pairs with randomized packed arguments | [x] |

There are no Cargo features in `Cargo.toml`; the sole build configuration is
the no-feature/default configuration.
