# Configuration Surface

Rows are derived from the public exported entry points and each distinct
`if`/`switch` branch, count/shape case, option, and optional-output state in
`c_src/src/lib.c`. Randomized rows use a fixed seed.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `c2V` | arbitrary finite scalar pair | [x] |
| C2 | `c2Mulvs` | arbitrary finite vector and scalar, including zero/negative | [x] |
| C3 | `c2Maxv`, `c2Minv`, `c2Clampv` | independently ordered x/y components | [x] |
| C4 | `c2Maxv`, `c2Minv`, `c2Clampv` | equal boundary components and reversed clamp bounds | [x] |
| C5 | `c2Sub`, `c2Add`, `c2Neg` | arbitrary finite vectors | [x] |
| C6 | `c2Dot`, `c2Det2` | arbitrary finite vectors | [x] |
| C7 | `c2Mulvs`, `c2Add`, `c2Sub`, `c2Dot` | signed zero, infinity, and NaN component propagation | [x] |
| C8 | `c2RotIdentity`, `c2xIdentity` | no-input identity constructors | [x] |
| C9 | `c2Skew`, `c2CCW90` | arbitrary finite vectors | [x] |
| C10 | `c2Len`, `c2Div`, `c2Norm` | nonzero finite vectors and nonzero divisors | [x] |
| C11 | `c2Mulrv`, `c2MulrvT` | arbitrary finite rotation coefficients and vectors | [x] |
| C12 | `c2Mulxv` | arbitrary finite transform and vector | [x] |
| C13 | `c2BBVerts` | arbitrary AABB, including min components greater than max | [x] |
| C14 | `c2MakeProxy` | circle (`type == 0`, one vertex, radius copied) | [x] |
| C15 | `c2MakeProxy` | AABB (`type == 1`, four vertices, zero radius) | [x] |
| C16 | `c2MakeProxy` | capsule (`type == 2`, two vertices, radius copied) | [x] |
| C17 | `c2GJKSimplexMetric` | simplex count 1 | [x] |
| C18 | `c2GJKSimplexMetric` | simplex count 2 | [x] |
| C19 | `c2GJKSimplexMetric` | simplex count 3, both triangle orientations | [x] |
| C20 | `c22` | `v <= 0` selects vertex A | [x] |
| C21 | `c22` | `v > 0 && u <= 0` selects vertex B | [x] |
| C22 | `c22` | `v > 0 && u > 0` retains edge AB | [x] |
| C23 | `c23` | vertex-A region (`vAB <= 0 && uCA <= 0`) | [x] |
| C24 | `c23` | vertex-B region (`uAB <= 0 && vBC <= 0`) | [x] |
| C25 | `c23` | vertex-C region (`uBC <= 0 && vCA <= 0`) | [x] |
| C26 | `c23` | edge-AB region | [x] |
| C27 | `c23` | edge-BC region | [x] |
| C28 | `c23` | edge-CA region | [x] |
| C29 | `c23` | triangle interior region | [x] |
| C30 | `c2D` | simplex count 1 | [x] |
| C31 | `c2D` | simplex count 2 with positive determinant branch | [x] |
| C32 | `c2D` | simplex count 2 with nonpositive determinant branch | [x] |
| C33 | `c2D` | simplex count 3/default zero direction | [x] |
| C34 | `c2Support` | one vertex | [x] |
| C35 | `c2Support` | many vertices with unique later maximum | [x] |
| C36 | `c2Support` | many vertices with tied maximum (first index retained) | [x] |
| C37 | `c2Witness` | simplex count 1 | [x] |
| C38 | `c2Witness` | simplex count 2 with arbitrary positive weights/divisor | [x] |
| C39 | `c2Witness` | simplex count 3 with arbitrary positive weights/divisor | [x] |
| C40 | `c2L` | simplex count 1 | [x] |
| C41 | `c2L` | simplex count 2 with arbitrary positive weights/divisor | [x] |
| C42 | `c2GJK` | circle-circle, `use_radius == 0`, identity transforms, no cache | [x] |
| C43 | `c2GJK` | circle-AABB, `use_radius == 0`, identity transforms, no cache | [x] |
| C44 | `c2GJK` | circle-capsule, `use_radius == 0`, identity transforms, no cache | [x] |
| C45 | `c2GJK` | AABB-circle, `use_radius == 0`, identity transforms, no cache | [x] |
| C46 | `c2GJK` | AABB-AABB, `use_radius == 0`, identity transforms, no cache | [x] |
| C47 | `c2GJK` | AABB-capsule, `use_radius == 0`, identity transforms, no cache | [x] |
| C48 | `c2GJK` | capsule-circle, `use_radius == 0`, identity transforms, no cache | [x] |
| C49 | `c2GJK` | capsule-AABB, `use_radius == 0`, identity transforms, no cache | [x] |
| C50 | `c2GJK` | capsule-capsule, `use_radius == 0`, identity transforms, no cache | [x] |
| C51 | `c2GJK` | circle-circle, `use_radius != 0`, identity transforms, no cache | [x] |
| C52 | `c2GJK` | circle-AABB, `use_radius != 0`, identity transforms, no cache | [x] |
| C53 | `c2GJK` | circle-capsule, `use_radius != 0`, identity transforms, no cache | [x] |
| C54 | `c2GJK` | AABB-circle, `use_radius != 0`, identity transforms, no cache | [x] |
| C55 | `c2GJK` | AABB-AABB, `use_radius != 0`, identity transforms, no cache | [x] |
| C56 | `c2GJK` | AABB-capsule, `use_radius != 0`, identity transforms, no cache | [x] |
| C57 | `c2GJK` | capsule-circle, `use_radius != 0`, identity transforms, no cache | [x] |
| C58 | `c2GJK` | capsule-AABB, `use_radius != 0`, identity transforms, no cache | [x] |
| C59 | `c2GJK` | capsule-capsule, `use_radius != 0`, identity transforms, no cache | [x] |
| C60 | `c2GJK` | explicit translated/rotated transforms for both shapes | [x] |
| C61 | `c2GJK` | mixed NULL/explicit transforms | [x] |
| C62 | `c2GJK` | cold cache (`count == 0`) with cache output | [x] |
| C63 | `c2GJK` | warm cache from immediately preceding identical query | [x] |
| C64 | `c2GJK` | nonzero cache rejected by the source metric condition and rebuilt | [x] |
| C65 | `c2GJK` | separated shapes, radius branch subtracts radii | [x] |
| C66 | `c2GJK` | overlapping/touching shapes, radius branch collapses witnesses | [x] |
| C67 | `c2GJK` | duplicate support pair terminates iteration | [x] |
| C68 | `c2GJK` | all optional outputs present | [x] |
| C69 | `c2GJK` | each optional output omitted independently | [x] |
| C70 | `gjk_cache` | `reverse == 0`, randomized AABB/capsule inputs | [x] |
| C71 | `gjk_cache` | `reverse != 0` (including non-1 values), randomized inputs | [x] |

Cargo feature combinations: **one** (the crate defines no features), so the
default/no-feature build is the complete feature matrix.

