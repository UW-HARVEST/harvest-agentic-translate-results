# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `CMakeLists.txt` has no options,
compile definitions, conditional sources, or preprocessor-controlled
backends. There is exactly one valid feature combination:

| # | Cargo invocation | C configuration | [ ] |
|---|------------------|-----------------|-----|
| B1 | `--no-default-features` (empty feature set) | Default CMake configuration | [x] |

## Runtime Configurations

Rows are derived from exported C functions and their `if`/`switch`/loop
branches. "Random" means many finite randomized values with a fixed seed.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | Random scalar pair | [x] |
| 2 | `c2Mulvs` | Random vector and scalar | [x] |
| 3 | `c2Maxv` | `a.x > b.x`, `a.y > b.y` | [x] |
| 4 | `c2Maxv` | `a.x > b.x`, `a.y <= b.y` | [x] |
| 5 | `c2Maxv` | `a.x <= b.x`, `a.y > b.y` | [x] |
| 6 | `c2Maxv` | `a.x <= b.x`, `a.y <= b.y`, including equality | [x] |
| 7 | `c2Minv` | `a.x < b.x`, `a.y < b.y` | [x] |
| 8 | `c2Minv` | `a.x < b.x`, `a.y >= b.y` | [x] |
| 9 | `c2Minv` | `a.x >= b.x`, `a.y < b.y` | [x] |
| 10 | `c2Minv` | `a.x >= b.x`, `a.y >= b.y`, including equality | [x] |
| 11 | `c2Clampv` | x below, y below | [x] |
| 12 | `c2Clampv` | x below, y inside | [x] |
| 13 | `c2Clampv` | x below, y above | [x] |
| 14 | `c2Clampv` | x inside, y below | [x] |
| 15 | `c2Clampv` | x inside, y inside, including bounds | [x] |
| 16 | `c2Clampv` | x inside, y above | [x] |
| 17 | `c2Clampv` | x above, y below | [x] |
| 18 | `c2Clampv` | x above, y inside | [x] |
| 19 | `c2Clampv` | x above, y above | [x] |
| 20 | `c2Sub` | Random vectors | [x] |
| 21 | `c2Dot` | Random vectors | [x] |
| 22 | `c2RotIdentity` | No input | [x] |
| 23 | `c2xIdentity` | No input | [x] |
| 24 | `c2BBVerts` | Random ordered AABB bounds | [x] |
| 25 | `c2MakeProxy` | Circle shape | [x] |
| 26 | `c2MakeProxy` | AABB shape | [x] |
| 27 | `c2MakeProxy` | Capsule shape | [x] |
| 28 | `c2Len` | Random vector, including zero | [x] |
| 29 | `c2Det2` | Random vectors | [x] |
| 30 | `c2GJKSimplexMetric` | `count == 1` | [x] |
| 31 | `c2GJKSimplexMetric` | `count == 2` | [x] |
| 32 | `c2GJKSimplexMetric` | `count == 3`, both determinant signs | [x] |
| 33 | `c2GJKSimplexMetric` | count outside 1..3 takes default/1 arm | [x] |
| 34 | `c2Mulrv` | Random rotation coefficients and vector | [x] |
| 35 | `c2Add` | Random vectors | [x] |
| 36 | `c2Mulxv` | Random transform and vector | [x] |
| 37 | `c22` | `v <= 0` selects vertex A | [x] |
| 38 | `c22` | `v > 0 && u <= 0` selects vertex B | [x] |
| 39 | `c22` | `v > 0 && u > 0` retains edge AB | [x] |
| 40 | `c23` | vertex-A Voronoi region | [x] |
| 41 | `c23` | vertex-B Voronoi region | [x] |
| 42 | `c23` | vertex-C Voronoi region | [x] |
| 43 | `c23` | edge-AB Voronoi region | [x] |
| 44 | `c23` | edge-BC Voronoi region | [x] |
| 45 | `c23` | edge-CA Voronoi region | [x] |
| 46 | `c23` | triangle interior/fallback | [x] |
| 47 | `c2Neg` | Random vector | [x] |
| 48 | `c2Skew` | Random vector | [x] |
| 49 | `c2CCW90` | Random vector | [x] |
| 50 | `c2D` | `count == 1` | [x] |
| 51 | `c2D` | `count == 2`, determinant positive | [x] |
| 52 | `c2D` | `count == 2`, determinant nonpositive | [x] |
| 53 | `c2D` | `count == 3` | [x] |
| 54 | `c2D` | count outside 1..3 | [x] |
| 55 | `c2Support` | `count == 1` | [x] |
| 56 | `c2Support` | many vertices, first is strict maximum | [x] |
| 57 | `c2Support` | many vertices, later vertex is strict maximum | [x] |
| 58 | `c2Support` | tied maximum retains earliest index | [x] |
| 59 | `c2Witness` | `count == 1` | [x] |
| 60 | `c2Witness` | `count == 2` | [x] |
| 61 | `c2Witness` | `count == 3` | [x] |
| 62 | `c2Witness` | count outside 1..3 writes zero vectors | [x] |
| 63 | `c2Div` | Random vector and nonzero divisor | [x] |
| 64 | `c2Norm` | Random nonzero vector | [x] |
| 65 | `c2Norm` | Zero vector follows IEEE division behavior | [x] |
| 66 | `c2L` | `count == 1` | [x] |
| 67 | `c2L` | `count == 2` | [x] |
| 68 | `c2L` | other count returns zero vector | [x] |
| 69 | `c2MulrvT` | Random rotation coefficients and vector | [x] |
| 70 | `c2GJK` | circle-circle, `use_radius == 0`, identity transforms | [x] |
| 71 | `c2GJK` | circle-AABB, `use_radius == 0`, identity transforms | [x] |
| 72 | `c2GJK` | circle-capsule, `use_radius == 0`, identity transforms | [x] |
| 73 | `c2GJK` | AABB-circle, `use_radius == 0`, identity transforms | [x] |
| 74 | `c2GJK` | AABB-AABB, `use_radius == 0`, identity transforms | [x] |
| 75 | `c2GJK` | AABB-capsule, `use_radius == 0`, identity transforms | [x] |
| 76 | `c2GJK` | capsule-circle, `use_radius == 0`, identity transforms | [x] |
| 77 | `c2GJK` | capsule-AABB, `use_radius == 0`, identity transforms | [x] |
| 78 | `c2GJK` | capsule-capsule, `use_radius == 0`, identity transforms | [x] |
| 79 | `c2GJK` | circle-circle, `use_radius != 0`, identity transforms | [x] |
| 80 | `c2GJK` | circle-AABB, `use_radius != 0`, identity transforms | [x] |
| 81 | `c2GJK` | circle-capsule, `use_radius != 0`, identity transforms | [x] |
| 82 | `c2GJK` | AABB-circle, `use_radius != 0`, identity transforms | [x] |
| 83 | `c2GJK` | AABB-AABB, `use_radius != 0`, identity transforms | [x] |
| 84 | `c2GJK` | AABB-capsule, `use_radius != 0`, identity transforms | [x] |
| 85 | `c2GJK` | capsule-circle, `use_radius != 0`, identity transforms | [x] |
| 86 | `c2GJK` | capsule-AABB, `use_radius != 0`, identity transforms | [x] |
| 87 | `c2GJK` | capsule-capsule, `use_radius != 0`, identity transforms | [x] |
| 88 | `c2GJK` | nonnull A transform, null B transform | [x] |
| 89 | `c2GJK` | null A transform, nonnull B transform | [x] |
| 90 | `c2GJK` | both transforms nonnull | [x] |
| 91 | `c2GJK` | null `outA`, other outputs present | [x] |
| 92 | `c2GJK` | null `outB`, other outputs present | [x] |
| 93 | `c2GJK` | null `iterations`, point outputs present | [x] |
| 94 | `c2GJK` | all optional output pointers null | [x] |
| 95 | `c2GJK` | nonnull cache with `count == 0` | [x] |
| 96 | `c2GJK` | warm cache with `count == 1` | [x] |
| 97 | `c2GJK` | warm cache with `count == 2` | [x] |
| 98 | `c2GJK` | warm cache with `count == 3` | [x] |
| 99 | `c2GJK` | cache metric rejection condition triggers cold start | [x] |
| 100 | `c2GJK` | separated shapes | [x] |
| 101 | `c2GJK` | touching shapes | [x] |
| 102 | `c2GJK` | overlapping or simplex-hit shapes | [x] |
| 103 | `c2GJK` | degenerate zero-length capsule/zero-size AABB | [x] |
| 104 | `c2AABBtoAABB` | separated on x | [x] |
| 105 | `c2AABBtoAABB` | separated on y | [x] |
| 106 | `c2AABBtoAABB` | touching boundary | [x] |
| 107 | `c2AABBtoAABB` | positive-area overlap | [x] |
| 108 | `c2AABBtoCapsule` | separated | [x] |
| 109 | `c2AABBtoCapsule` | colliding | [x] |
| 110 | `c2AABBtoCapsule` | tangent | [x] |
| 111 | `c2CapsuletoCapsule` | separated | [x] |
| 112 | `c2CapsuletoCapsule` | colliding | [x] |
| 113 | `c2CapsuletoCapsule` | tangent | [x] |
| 114 | `c2CircletoCircle` | separated | [x] |
| 115 | `c2CircletoCircle` | tangent (strict comparison returns false) | [x] |
| 116 | `c2CircletoCircle` | overlapping | [x] |
| 117 | `c2CircletoAABB` | center inside AABB | [x] |
| 118 | `c2CircletoAABB` | nearest point on edge, separated | [x] |
| 119 | `c2CircletoAABB` | nearest point on edge, tangent | [x] |
| 120 | `c2CircletoAABB` | nearest point on edge, overlapping | [x] |
| 121 | `c2CircletoAABB` | nearest point is corner, separated | [x] |
| 122 | `c2CircletoAABB` | nearest point is corner, tangent | [x] |
| 123 | `c2CircletoAABB` | nearest point is corner, overlapping | [x] |
| 124 | `c2CircletoCapsule` | projection before endpoint A, separated/colliding | [x] |
| 125 | `c2CircletoCapsule` | projection on segment interior, separated/colliding | [x] |
| 126 | `c2CircletoCapsule` | projection after endpoint B, separated/colliding | [x] |
| 127 | `c2CircletoCapsule` | tangent boundary | [x] |
| 128 | `c2CircletoCapsule` | zero-length capsule | [x] |
| 129 | `c2Collided` | circle-circle dispatch | [x] |
| 130 | `c2Collided` | circle-AABB dispatch | [x] |
| 131 | `c2Collided` | circle-capsule dispatch | [x] |
| 132 | `c2Collided` | AABB-circle reverse dispatch | [x] |
| 133 | `c2Collided` | AABB-AABB dispatch | [x] |
| 134 | `c2Collided` | AABB-capsule dispatch | [x] |
| 135 | `c2Collided` | capsule-circle reverse dispatch | [x] |
| 136 | `c2Collided` | capsule-AABB reverse dispatch | [x] |
| 137 | `c2Collided` | capsule-capsule dispatch | [x] |
| 138 | `reverse_collide` | Random `(x, y, r)`, including all three collision-bit boundaries | [x] |
