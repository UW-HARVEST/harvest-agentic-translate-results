# Configuration-surface table

Rows are derived from the public dynamic symbols and the `if`/`switch` branches
in `src/lib.c`. Randomized tests may cover multiple rows in one test function,
but each row must be exercised and checked independently before completion.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `c2V` | arbitrary finite scalar pair | [x] |
| 2 | `c2Mulvs` | arbitrary vector and nonzero finite scalar | [x] |
| 3 | `c2Maxv` | each component chooses `a` (`a > b`) | [x] |
| 4 | `c2Maxv` | each component chooses `b` (`a <= b`, including equality) | [x] |
| 5 | `c2Minv` | each component chooses `a` (`a < b`) | [x] |
| 6 | `c2Minv` | each component chooses `b` (`a >= b`, including equality) | [x] |
| 7 | `c2Clampv` | component below, inside, and above `[lo, hi]` | [x] |
| 8 | `c2Sub`, `c2Add`, `c2Dot`, `c2Det2` | arbitrary finite vectors | [x] |
| 9 | `c2RotIdentity`, `c2xIdentity` | no-input identity constructors | [x] |
| 10 | `c2Mulrv`, `c2MulrvT` | arbitrary finite rotation coefficients/vector | [x] |
| 11 | `c2Mulxv` | arbitrary finite transform/vector | [x] |
| 12 | `c2Neg`, `c2Skew`, `c2CCW90` | arbitrary finite vector | [x] |
| 13 | `c2Len` | zero vector | [x] |
| 14 | `c2Len` | nonzero finite vector | [x] |
| 15 | `c2Div` | arbitrary vector and nonzero finite divisor | [x] |
| 16 | `c2Norm` | nonzero finite vector | [x] |
| 17 | `c2BBVerts` | arbitrary AABB endpoints | [x] |
| 18 | `c2MakeProxy` | circle: one vertex plus radius | [x] |
| 19 | `c2MakeProxy` | AABB: four vertices, zero radius | [x] |
| 20 | `c2MakeProxy` | capsule: two vertices plus radius | [x] |
| 21 | `c2GJKSimplexMetric` | simplex count 1 or any default count: zero | [x] |
| 22 | `c2GJKSimplexMetric` | simplex count 2: segment length | [x] |
| 23 | `c2GJKSimplexMetric` | simplex count 3: signed determinant | [x] |
| 24 | `c22` | `v <= 0`: reduce to vertex A | [x] |
| 25 | `c22` | `v > 0 && u <= 0`: reduce to vertex B | [x] |
| 26 | `c22` | `v > 0 && u > 0`: retain edge AB | [x] |
| 27 | `c23` | vertex-A Voronoi branch | [x] |
| 28 | `c23` | vertex-B Voronoi branch | [x] |
| 29 | `c23` | vertex-C Voronoi branch | [x] |
| 30 | `c23` | edge-AB Voronoi branch | [x] |
| 31 | `c23` | edge-BC Voronoi branch | [x] |
| 32 | `c23` | edge-CA Voronoi branch | [x] |
| 33 | `c23` | interior/default triangle branch | [x] |
| 34 | `c2D` | simplex count 1 | [x] |
| 35 | `c2D` | count 2 and positive determinant: skew branch | [x] |
| 36 | `c2D` | count 2 and non-positive determinant: clockwise branch | [x] |
| 37 | `c2D` | count 3 or default count: zero vector | [x] |
| 38 | `c2Support` | one vertex | [x] |
| 39 | `c2Support` | many vertices with strict greater-than winner | [x] |
| 40 | `c2Support` | tied support values retain earliest index | [x] |
| 41 | `c2Witness` | simplex count 1 | [x] |
| 42 | `c2Witness` | simplex count 2 | [x] |
| 43 | `c2Witness` | simplex count 3 | [x] |
| 44 | `c2Witness` | default count | [x] |
| 45 | `c2L` | simplex count 1 | [x] |
| 46 | `c2L` | simplex count 2 | [x] |
| 47 | `c2L` | default count | [x] |
| 48 | `c2GJK` | all 9 ordered shape-type pairs, identity transforms, `use_radius == 0`, no cache, all optional outputs present | [x] |
| 49 | `c2GJK` | all 9 ordered shape-type pairs, identity transforms, `use_radius != 0`, no cache, all optional outputs present | [x] |
| 50 | `c2GJK` | explicit non-null transforms for both shapes | [x] |
| 51 | `c2GJK` | null `ax_ptr` and/or null `bx_ptr` selects identity | [x] |
| 52 | `c2GJK` | empty non-null cache (`count == 0`) is ignored then populated | [x] |
| 53 | `c2GJK` | populated non-null cache is read, then updated | [x] |
| 54 | `c2GJK` | cache metric guard rejects cached simplex and restarts | [x] |
| 55 | `c2GJK` | null cache | [x] |
| 56 | `c2GJK` | each optional output independently null: `outA`, `outB`, `iterations` | [x] |
| 57 | `c2GJK` | all optional outputs null | [x] |
| 58 | `c2GJK` | overlap reaches triangle/hit result | [x] |
| 59 | `c2GJK` | radius mode, separated beyond summed radii | [x] |
| 60 | `c2GJK` | radius mode, within/touching summed radii collapses witnesses | [x] |
| 61 | `c2GJK` | duplicate support pair terminates iteration | [x] |
| 62 | `c2GJK` | near-zero search direction terminates iteration | [x] |
| 63 | `c2AABBtoAABB` | separated left/right/up/down | [x] |
| 64 | `c2AABBtoAABB` | overlapping or edge-touching | [x] |
| 65 | `c2AABBtoCapsule` | colliding | [x] |
| 66 | `c2AABBtoCapsule` | separated | [x] |
| 67 | `c2CapsuletoCapsule` | colliding | [x] |
| 68 | `c2CapsuletoCapsule` | separated | [x] |
| 69 | `c2CircletoCircle` | strict overlap | [x] |
| 70 | `c2CircletoCircle` | tangent or separated | [x] |
| 71 | `c2CircletoAABB` | strict overlap | [x] |
| 72 | `c2CircletoAABB` | tangent or separated | [x] |
| 73 | `c2CircletoCapsule` | circle projects before capsule A endpoint | [x] |
| 74 | `c2CircletoCapsule` | circle projects onto capsule segment | [x] |
| 75 | `c2CircletoCapsule` | circle projects after capsule B endpoint | [x] |
| 76 | `c2CircletoCapsule` | strict overlap versus tangent/separated boundary | [x] |
| 77 | `c2Collided` | circle-circle | [x] |
| 78 | `c2Collided` | circle-AABB | [x] |
| 79 | `c2Collided` | circle-capsule | [x] |
| 80 | `c2Collided` | AABB-circle (reversed dispatch) | [x] |
| 81 | `c2Collided` | AABB-AABB | [x] |
| 82 | `c2Collided` | AABB-capsule | [x] |
| 83 | `c2Collided` | capsule-circle (reversed dispatch) | [x] |
| 84 | `c2Collided` | capsule-AABB (reversed dispatch) | [x] |
| 85 | `c2Collided` | capsule-capsule | [x] |
| 86 | `ptr_from_parts` | circle field mapping `(a,b,c)` | [x] |
| 87 | `ptr_from_parts` | AABB field mapping `(a,b,c,d)` | [x] |
| 88 | `ptr_from_parts` | capsule field mapping `(a,b,c,d,e)` | [x] |
| 89 | `omni_collide` | all 9 ordered valid type pairs with randomized fields | [x] |

## Cargo feature combinations

`Cargo.toml` declares no `[features]` table. Therefore the complete feature
matrix has one member: the default/no-feature build.

Verified with both:

- `cargo test --release --test differential`
- `cargo test --release --no-default-features --test differential`
