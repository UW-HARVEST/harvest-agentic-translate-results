# Configuration Surface

Rows are derived from every `if`, `switch`, loop-count shape, public option,
and optional pointer in `src/lib.c`. Independent scalar helpers are grouped
only where they have the same input-shape distinction. Every dynamically
exported entry point appears below.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | finite scalar components, including signed zero and finite extrema | [x] |
| 2 | `c2Mulvs` | vectors and positive, negative, zero, and fractional scalars | [x] |
| 3 | `c2Add`, `c2Sub` | finite vectors with mixed component signs | [x] |
| 4 | `c2Dot`, `c2Det2` | finite vectors including parallel, perpendicular, and general pairs | [x] |
| 5 | `c2Maxv`, `c2Minv` | `a.x/y` greater than `b.x/y` | [x] |
| 6 | `c2Maxv`, `c2Minv` | `a.x/y` less than `b.x/y` | [x] |
| 7 | `c2Maxv`, `c2Minv` | equal and mixed-order components | [x] |
| 8 | `c2Clampv` | both components below `lo` | [x] |
| 9 | `c2Clampv` | both components inside `[lo, hi]` | [x] |
| 10 | `c2Clampv` | both components above `hi` | [x] |
| 11 | `c2Clampv` | components in different below/inside/above regions | [x] |
| 12 | `c2Neg`, `c2Skew`, `c2CCW90` | positive, negative, mixed, and zero vectors | [x] |
| 13 | `c2Len` | zero vector | [x] |
| 14 | `c2Len` | nonzero finite vector | [x] |
| 15 | `c2Div` | nonzero positive and negative divisor | [x] |
| 16 | `c2Div` | positive and negative zero divisor | [x] |
| 17 | `c2Norm` | nonzero finite vector | [x] |
| 18 | `c2Norm` | zero vector | [x] |
| 19 | `c2RotIdentity`, `c2xIdentity` | no-input identity construction | [x] |
| 20 | `c2Mulrv`, `c2MulrvT` | identity and general sine/cosine pairs | [x] |
| 21 | `c2Mulxv` | identity and general rotation plus translation | [x] |
| 22 | `c2BBVerts` | nondegenerate, point, line, and inverted AABBs | [x] |
| 23 | `c2MakeProxy` | circle: one vertex and shape radius | [x] |
| 24 | `c2MakeProxy` | AABB: four vertices and zero radius | [x] |
| 25 | `c2MakeProxy` | capsule: two vertices and shape radius | [x] |
| 26 | `c2GJKSimplexMetric` | simplex count `1` or any unsupported count: zero | [x] |
| 27 | `c2GJKSimplexMetric` | simplex count `2`: segment length | [x] |
| 28 | `c2GJKSimplexMetric` | simplex count `3`: signed determinant | [x] |
| 29 | `c22` | `v <= 0`: reduce to vertex A | [x] |
| 30 | `c22` | `v > 0 && u <= 0`: reduce to vertex B | [x] |
| 31 | `c22` | `u > 0 && v > 0`: retain edge AB | [x] |
| 32 | `c23` | `vAB <= 0 && uCA <= 0`: reduce to vertex A | [x] |
| 33 | `c23` | `uAB <= 0 && vBC <= 0`: reduce to vertex B | [x] |
| 34 | `c23` | `uBC <= 0 && vCA <= 0`: reduce to vertex C | [x] |
| 35 | `c23` | positive AB weights and `wABC <= 0`: retain edge AB | [x] |
| 36 | `c23` | positive BC weights and `uABC <= 0`: retain edge BC | [x] |
| 37 | `c23` | positive CA weights and `vABC <= 0`: retain edge CA | [x] |
| 38 | `c23` | interior/default region: retain triangle ABC | [x] |
| 39 | `c2D` | simplex count `1` | [x] |
| 40 | `c2D` | simplex count `2`, determinant positive | [x] |
| 41 | `c2D` | simplex count `2`, determinant zero or negative | [x] |
| 42 | `c2D` | simplex count `3` or unsupported count | [x] |
| 43 | `c2Support` | one vertex | [x] |
| 44 | `c2Support` | many vertices with later strict maximum | [x] |
| 45 | `c2Support` | many vertices with tied maximum (first index retained) | [x] |
| 46 | `c2Witness` | simplex count `1` | [x] |
| 47 | `c2Witness` | simplex count `2` | [x] |
| 48 | `c2Witness` | simplex count `3` | [x] |
| 49 | `c2Witness` | unsupported count: both outputs zero | [x] |
| 50 | `c2L` | simplex count `1` | [x] |
| 51 | `c2L` | simplex count `2` | [x] |
| 52 | `c2L` | simplex count `3` or unsupported count: zero | [x] |
| 53 | `c2GJK` | circle-circle, identity transforms, `use_radius = 0` | [x] |
| 54 | `c2GJK` | circle-AABB, identity transforms, `use_radius = 0` | [x] |
| 55 | `c2GJK` | circle-capsule, identity transforms, `use_radius = 0` | [x] |
| 56 | `c2GJK` | AABB-circle, identity transforms, `use_radius = 0` | [x] |
| 57 | `c2GJK` | AABB-AABB, identity transforms, `use_radius = 0` | [x] |
| 58 | `c2GJK` | AABB-capsule, identity transforms, `use_radius = 0` | [x] |
| 59 | `c2GJK` | capsule-circle, identity transforms, `use_radius = 0` | [x] |
| 60 | `c2GJK` | capsule-AABB, identity transforms, `use_radius = 0` | [x] |
| 61 | `c2GJK` | capsule-capsule, identity transforms, `use_radius = 0` | [x] |
| 62 | `c2GJK` | circle-circle, non-null A transform and null B transform | [x] |
| 63 | `c2GJK` | AABB-capsule, null A transform and non-null B transform | [x] |
| 64 | `c2GJK` | capsule-AABB, both transforms non-null | [x] |
| 65 | `c2GJK` | circle-circle, `use_radius != 0`, separated beyond radii | [x] |
| 66 | `c2GJK` | circle-AABB, `use_radius != 0`, separated beyond radii | [x] |
| 67 | `c2GJK` | circle-capsule, `use_radius != 0`, overlap/touch radii | [x] |
| 68 | `c2GJK` | AABB-circle, `use_radius != 0`, overlap/touch radii | [x] |
| 69 | `c2GJK` | AABB-AABB, `use_radius != 0` (both proxy radii zero) | [x] |
| 70 | `c2GJK` | AABB-capsule, `use_radius != 0`, separated beyond radius | [x] |
| 71 | `c2GJK` | capsule-circle, `use_radius != 0`, separated beyond radii | [x] |
| 72 | `c2GJK` | capsule-AABB, `use_radius != 0`, overlap/touch radius | [x] |
| 73 | `c2GJK` | capsule-capsule, `use_radius != 0`, overlap/touch radii | [x] |
| 74 | `c2GJK` | all `outA`, `outB`, and `iterations` non-null | [x] |
| 75 | `c2GJK` | only `outA` non-null | [x] |
| 76 | `c2GJK` | only `outB` non-null | [x] |
| 77 | `c2GJK` | only `iterations` non-null | [x] |
| 78 | `c2GJK` | all optional output pointers null | [x] |
| 79 | `c2GJK` | cache pointer null | [x] |
| 80 | `c2GJK` | cache count zero: initialize simplex and write cache | [x] |
| 81 | `c2GJK` | warm cache count one | [x] |
| 82 | `c2GJK` | warm cache count two | [x] |
| 83 | `c2GJK` | warm cache count three | [x] |
| 84 | `c2GJK` | warm cache rejected by metric predicate, then reinitialized | [x] |
| 85 | `c2AABBtoAABB` | separated on each of left/right/above/below axes | [x] |
| 86 | `c2AABBtoAABB` | overlap and exact boundary touch | [x] |
| 87 | `c2CircletoCircle` | overlap, exact tangency, and separation | [x] |
| 88 | `c2CircletoAABB` | center inside, edge/corner overlap, exact tangency, separation | [x] |
| 89 | `c2CircletoCapsule` | projection before endpoint A (`da < 0`) | [x] |
| 90 | `c2CircletoCapsule` | projection on segment (`da >= 0 && db < 0`) | [x] |
| 91 | `c2CircletoCapsule` | projection beyond endpoint B (`db >= 0`) | [x] |
| 92 | `c2CircletoCapsule` | degenerate capsule segment | [x] |
| 93 | `c2AABBtoCapsule` | overlap/touch and separation | [x] |
| 94 | `c2CapsuletoCapsule` | overlap/touch and separation | [x] |
| 95 | `c2Collided` | all nine valid ordered type pairs | [x] |
| 96 | `capsule` | randomized finite endpoints/radius producing every reachable result bit combination | [x] |
