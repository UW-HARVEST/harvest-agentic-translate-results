# Configuration surface

Rows are derived from every dynamic C symbol and the branch-distinct states in
`src/lib.c`. “Random finite” includes positive, negative, zero, and boundary
magnitudes while avoiding undefined float-to-memory behavior.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | arbitrary random finite `x,y` | [x] |
| 2 | `c2Mulvs` | arbitrary vector; scalar negative, zero, or positive | [x] |
| 3 | `c2Maxv` | component comparison selects first operand | [x] |
| 4 | `c2Maxv` | component comparison selects second operand, including ties | [x] |
| 5 | `c2Minv` | component comparison selects first operand | [x] |
| 6 | `c2Minv` | component comparison selects second operand, including ties | [x] |
| 7 | `c2Clampv` | components below, inside, and above inclusive bounds | [x] |
| 8 | `c2Sub`, `c2Add` | arbitrary random finite vector pairs | [x] |
| 9 | `c2Dot`, `c2Det2` | parallel, perpendicular, and arbitrary vector pairs | [x] |
| 10 | `c2Len` | zero and nonzero vectors | [x] |
| 11 | `c2Neg`, `c2Skew`, `c2CCW90` | arbitrary random finite vectors | [x] |
| 12 | `c2Div` | nonzero positive and negative divisors | [x] |
| 13 | `c2Norm` | arbitrary nonzero vectors | [x] |
| 14 | `c2RotIdentity`, `c2xIdentity` | no-input identity constructors | [x] |
| 15 | `c2Mulrv`, `c2MulrvT` | identity and arbitrary rotation coefficients | [x] |
| 16 | `c2Mulxv` | arbitrary rotation coefficients plus translation | [x] |
| 17 | `c2BBVerts` | AABB with ordinary, zero-size, and inverted bounds | [x] |
| 18 | `c2MakeProxy` | circle shape (`type == 0`, one vertex, radius retained) | [x] |
| 19 | `c2MakeProxy` | AABB shape (`type == 1`, four vertices, zero radius) | [x] |
| 20 | `c2MakeProxy` | capsule shape (`type == 2`, two vertices, radius retained) | [x] |
| 21 | `c2GJKSimplexMetric` | count 1/default: metric zero | [x] |
| 22 | `c2GJKSimplexMetric` | count 2: segment length | [x] |
| 23 | `c2GJKSimplexMetric` | count 3: signed triangle determinant | [x] |
| 24 | `c22` | `v <= 0`: simplex reduces to vertex A | [x] |
| 25 | `c22` | `v > 0 && u <= 0`: simplex reduces to vertex B | [x] |
| 26 | `c22` | `v > 0 && u > 0`: simplex retains edge AB | [x] |
| 27 | `c23` | `vAB <= 0 && uCA <= 0`: reduce to A | [x] |
| 28 | `c23` | `uAB <= 0 && vBC <= 0`: reduce to B | [x] |
| 29 | `c23` | `uBC <= 0 && vCA <= 0`: reduce to C | [x] |
| 30 | `c23` | AB edge region (`uAB,vAB > 0`, `wABC <= 0`) | [x] |
| 31 | `c23` | BC edge region (`uBC,vBC > 0`, `uABC <= 0`) | [x] |
| 32 | `c23` | CA edge region (`uCA,vCA > 0`, `vABC <= 0`) | [x] |
| 33 | `c23` | triangle interior: retain all three vertices | [x] |
| 34 | `c2D` | simplex count 1 | [x] |
| 35 | `c2D` | simplex count 2 and determinant positive | [x] |
| 36 | `c2D` | simplex count 2 and determinant non-positive | [x] |
| 37 | `c2D` | simplex count 3/default | [x] |
| 38 | `c2Support` | one vertex | [x] |
| 39 | `c2Support` | many vertices; first vertex is strict maximum | [x] |
| 40 | `c2Support` | many vertices; later vertex is strict maximum | [x] |
| 41 | `c2Support` | many vertices; tied maximum keeps earliest index | [x] |
| 42 | `c2Witness` | simplex count 1 | [x] |
| 43 | `c2Witness` | simplex count 2 weighted by `u/div` | [x] |
| 44 | `c2Witness` | simplex count 3 weighted by `u/div` | [x] |
| 45 | `c2Witness` | simplex count default writes zero vectors | [x] |
| 46 | `c2L` | simplex count 1 | [x] |
| 47 | `c2L` | simplex count 2 weighted by `u/div` | [x] |
| 48 | `c2L` | simplex count default returns zero | [x] |
| 49 | `c2GJK` | circle-circle shape pair | [x] |
| 50 | `c2GJK` | circle-AABB shape pair | [x] |
| 51 | `c2GJK` | circle-capsule shape pair | [x] |
| 52 | `c2GJK` | AABB-circle shape pair | [x] |
| 53 | `c2GJK` | AABB-AABB shape pair | [x] |
| 54 | `c2GJK` | AABB-capsule shape pair | [x] |
| 55 | `c2GJK` | capsule-circle shape pair | [x] |
| 56 | `c2GJK` | capsule-AABB shape pair | [x] |
| 57 | `c2GJK` | capsule-capsule shape pair | [x] |
| 58 | `c2GJK` | null transforms select identity | [x] |
| 59 | `c2GJK` | non-null transforms apply rotation and translation | [x] |
| 60 | `c2GJK` | `use_radius == 0` | [x] |
| 61 | `c2GJK` | `use_radius != 0`, separated farther than radius sum | [x] |
| 62 | `c2GJK` | `use_radius != 0`, overlap/within radius sum | [x] |
| 63 | `c2GJK` | no cache | [x] |
| 64 | `c2GJK` | empty cache (`count == 0`) is initialized and written | [x] |
| 65 | `c2GJK` | populated cache is read, validated, and rewritten | [x] |
| 66 | `c2GJK` | non-null witness and iteration outputs | [x] |
| 67 | `c2GJK` | null witness and iteration outputs | [x] |
| 68 | `c2GJK` | simplex reaches count 3 (intersection hit) | [x] |
| 69 | `c2GJK` | search exits by duplicate support point / no progress | [x] |
| 70 | `gjk` | `reverse == 0`, separated AABB-capsule | [x] |
| 71 | `gjk` | `reverse == 0`, overlapping AABB-capsule | [x] |
| 72 | `gjk` | `reverse != 0`, separated capsule-AABB | [x] |
| 73 | `gjk` | `reverse != 0`, overlapping capsule-AABB | [x] |

Cargo features found in `Cargo.toml`: none. The only feature configuration is
the default/no-feature build.

