# Configuration surface

Rows are derived from every exported entry point and each branch axis in
`src/lib.c`. "Random finite" includes positive, negative, zero, boundary
equality, and varied magnitudes. Each checked row is exercised repeatedly with
a fixed-seed generator through both shared-library FFI boundaries.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random finite scalar pair | [x] |
| 2 | `c2Mulvs`, `c2Add`, `c2Sub` | random finite vectors/scalar | [x] |
| 3 | `c2Dot`, `c2Det2` | random finite vector pairs | [x] |
| 4 | `c2Maxv`, `c2Minv`, `c2Clampv` | each comparison arm, equality, ordered bounds | [x] |
| 5 | `c2RotIdentity`, `c2xIdentity` | no-input identity constructors | [x] |
| 6 | `c2Len`, `c2Div`, `c2Norm` | nonzero finite vectors and nonzero divisor | [x] |
| 7 | `c2Neg`, `c2Skew`, `c2CCW90` | random finite vectors | [x] |
| 8 | `c2Mulrv`, `c2MulrvT`, `c2Mulxv` | random finite rotations/transforms/vectors | [x] |
| 9 | `c2BBVerts` | random AABB, four output vertices | [x] |
| 10 | `c2MakeProxy` | circle: one vertex plus radius | [x] |
| 11 | `c2MakeProxy` | AABB: four vertices and zero radius | [x] |
| 12 | `c2MakeProxy` | capsule: two vertices plus radius | [x] |
| 13 | `c2GJKSimplexMetric` | `count` default/0/1 | [x] |
| 14 | `c2GJKSimplexMetric` | `count == 2` segment length | [x] |
| 15 | `c2GJKSimplexMetric` | `count == 3` signed triangle determinant | [x] |
| 16 | `c22` | Voronoi region A (`v <= 0`) | [x] |
| 17 | `c22` | Voronoi region B (`v > 0 && u <= 0`) | [x] |
| 18 | `c22` | segment interior (`v > 0 && u > 0`) | [x] |
| 19 | `c23` | vertex region A | [x] |
| 20 | `c23` | vertex region B | [x] |
| 21 | `c23` | vertex region C | [x] |
| 22 | `c23` | edge region AB | [x] |
| 23 | `c23` | edge region BC | [x] |
| 24 | `c23` | edge region CA | [x] |
| 25 | `c23` | triangle interior/default | [x] |
| 26 | `c2D` | simplex `count == 1` | [x] |
| 27 | `c2D` | `count == 2`, positive determinant | [x] |
| 28 | `c2D` | `count == 2`, nonpositive determinant | [x] |
| 29 | `c2D` | `count == 3` and unsupported/default count | [x] |
| 30 | `c2Support` | `count <= 1` | [x] |
| 31 | `c2Support` | many vertices, first remains maximum or ties | [x] |
| 32 | `c2Support` | many vertices, later strict maximum | [x] |
| 33 | `c2Witness` | simplex `count == 1` | [x] |
| 34 | `c2Witness` | simplex `count == 2` | [x] |
| 35 | `c2Witness` | simplex `count == 3` | [x] |
| 36 | `c2Witness` | unsupported/default simplex count | [x] |
| 37 | `c2L` | simplex `count == 1` | [x] |
| 38 | `c2L` | simplex `count == 2` | [x] |
| 39 | `c2L` | unsupported/default simplex count | [x] |
| 40 | `c2GJK` | circle-circle; radius off and on | [x] |
| 41 | `c2GJK` | circle-AABB; radius off and on | [x] |
| 42 | `c2GJK` | circle-capsule; radius off and on | [x] |
| 43 | `c2GJK` | AABB-circle; radius off and on | [x] |
| 44 | `c2GJK` | AABB-AABB; radius off and on | [x] |
| 45 | `c2GJK` | AABB-capsule; radius off and on | [x] |
| 46 | `c2GJK` | capsule-circle; radius off and on | [x] |
| 47 | `c2GJK` | capsule-AABB; radius off and on | [x] |
| 48 | `c2GJK` | capsule-capsule; radius off and on | [x] |
| 49 | `c2GJK` | each shape pair with null versus explicit transforms | [x] |
| 50 | `c2GJK` | each shape pair with all output pointers present versus null | [x] |
| 51 | `c2GJK` | null cache, zero-count cache, and warm cache reused on changed shapes | [x] |
| 52 | `c2GJK` | separated, touching/radius-overlap, and simplex-hit geometry | [x] |
| 53 | `c2AABBtoAABB` | separated on each axis, touching, and overlapping | [x] |
| 54 | `c2AABBtoCapsule` | separated, touching, and overlapping | [x] |
| 55 | `c2CapsuletoCapsule` | separated, touching, and overlapping | [x] |
| 56 | `c2CircletoCircle` | separated, exactly tangent, and overlapping | [x] |
| 57 | `c2CircletoAABB` | center by side/corner/inside; tangent and overlap | [x] |
| 58 | `c2CircletoCapsule` | nearest region A endpoint (`da < 0`) | [x] |
| 59 | `c2CircletoCapsule` | nearest segment interior (`da >= 0 && db < 0`) | [x] |
| 60 | `c2CircletoCapsule` | nearest region B endpoint (`db >= 0`) | [x] |
| 61 | `c2Collided` | all 9 ordered valid type pairs | [x] |
| 62 | `reverse_collide` | random finite `(x, y, r)`, including each output bit | [x] |
| 63 | scalar/vector/transform entry points | signed zero, infinities, subnormals, maxima, and distinct quiet-NaN payloads | [x] |
| 64 | `c2GJK`, collision predicates, `reverse_collide` | signed zero, infinities, subnormals, maxima, and distinct quiet-NaN payloads | [x] |

Cargo features: none are declared, so the only feature combination is
`--no-default-features` (identical to the default build).
