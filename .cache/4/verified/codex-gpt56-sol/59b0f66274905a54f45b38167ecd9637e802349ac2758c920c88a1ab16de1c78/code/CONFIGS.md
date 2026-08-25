# Configuration Surface

## Build-Time Configurations

Neither `Cargo.toml` nor `c_src/CMakeLists.txt` declares a feature or build
option. There is exactly one valid feature combination:

| # | Cargo features | CMake options | |
|---|----------------|---------------|-|
| 1 | empty (`--no-default-features`) | defaults, plus requested PIC setting | [x] |

## Runtime Configurations

Rows are derived from every exported C function and its `if`/`switch` branches.
Randomized rows use a fixed seed and multiple inputs for each listed branch.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| 1 | `c2V` | arbitrary two-component vector construction | [x] |
| 2 | `c2Mulvs` | arbitrary vector and scalar | [x] |
| 3 | `c2Maxv` | `a.x > b.x`, `a.y > b.y` | [x] |
| 4 | `c2Maxv` | `a.x > b.x`, `a.y <= b.y` | [x] |
| 5 | `c2Maxv` | `a.x <= b.x`, `a.y > b.y` | [x] |
| 6 | `c2Maxv` | `a.x <= b.x`, `a.y <= b.y`, including equality | [x] |
| 7 | `c2Minv` | `a.x < b.x`, `a.y < b.y` | [x] |
| 8 | `c2Minv` | `a.x < b.x`, `a.y >= b.y` | [x] |
| 9 | `c2Minv` | `a.x >= b.x`, `a.y < b.y` | [x] |
| 10 | `c2Minv` | `a.x >= b.x`, `a.y >= b.y`, including equality | [x] |
| 11 | `c2Clampv` | x below `lo`; y below `lo` | [x] |
| 12 | `c2Clampv` | x below `lo`; y within bounds | [x] |
| 13 | `c2Clampv` | x below `lo`; y above `hi` | [x] |
| 14 | `c2Clampv` | x within bounds; y below `lo` | [x] |
| 15 | `c2Clampv` | x within bounds; y within bounds, including boundaries | [x] |
| 16 | `c2Clampv` | x within bounds; y above `hi` | [x] |
| 17 | `c2Clampv` | x above `hi`; y below `lo` | [x] |
| 18 | `c2Clampv` | x above `hi`; y within bounds | [x] |
| 19 | `c2Clampv` | x above `hi`; y above `hi` | [x] |
| 20 | `c2Sub` | arbitrary vectors | [x] |
| 21 | `c2Dot` | arbitrary vectors | [x] |
| 22 | `c2RotIdentity` | no input | [x] |
| 23 | `c2xIdentity` | no input | [x] |
| 24 | `c2BBVerts` | arbitrary AABB; four output vertices | [x] |
| 25 | `c2MakeProxy` | circle (`type == 0`): one vertex and radius | [x] |
| 26 | `c2MakeProxy` | AABB (`type == 1`): four vertices and zero radius | [x] |
| 27 | `c2MakeProxy` | capsule (`type == 2`): two vertices and radius | [x] |
| 28 | `c2MakeProxy` | invalid enum with null shape/output pointers: switch performs no access | [x] |
| 29 | `c2Len` | arbitrary vector | [x] |
| 30 | `c2Det2` | arbitrary vectors | [x] |
| 31 | `c2GJKSimplexMetric` | `count == 1` | [x] |
| 32 | `c2GJKSimplexMetric` | `count == 2` | [x] |
| 33 | `c2GJKSimplexMetric` | `count == 3` | [x] |
| 34 | `c2GJKSimplexMetric` | count outside 1..3, including zero | [x] |
| 35 | `c2Mulrv` | arbitrary rotation/vector | [x] |
| 36 | `c2Add` | arbitrary vectors | [x] |
| 37 | `c2Mulxv` | arbitrary transform/vector | [x] |
| 38 | `c22` | `v <= 0`: simplex reduces to original A | [x] |
| 39 | `c22` | `v > 0 && u <= 0`: simplex reduces to B | [x] |
| 40 | `c22` | `v > 0 && u > 0`: two-vertex simplex | [x] |
| 41 | `c23` | A Voronoi region (`vAB <= 0 && uCA <= 0`) | [x] |
| 42 | `c23` | B Voronoi region (`uAB <= 0 && vBC <= 0`) | [x] |
| 43 | `c23` | C Voronoi region (`uBC <= 0 && vCA <= 0`) | [x] |
| 44 | `c23` | AB edge region | [x] |
| 45 | `c23` | BC edge region | [x] |
| 46 | `c23` | CA edge region | [x] |
| 47 | `c23` | triangle interior/final `else` | [x] |
| 48 | `c2Neg` | arbitrary vector | [x] |
| 49 | `c2Skew` | arbitrary vector | [x] |
| 50 | `c2CCW90` | arbitrary vector | [x] |
| 51 | `c2D` | `count == 1` | [x] |
| 52 | `c2D` | `count == 2`, determinant positive | [x] |
| 53 | `c2D` | `count == 2`, determinant non-positive | [x] |
| 54 | `c2D` | `count == 3` | [x] |
| 55 | `c2D` | count outside 1..3 | [x] |
| 56 | `c2Support` | zero count with a valid first vertex | [x] |
| 57 | `c2Support` | one vertex | [x] |
| 58 | `c2Support` | many vertices; first remains maximum, including ties | [x] |
| 59 | `c2Support` | many vertices; later strict maximum replaces first | [x] |
| 60 | `c2Witness` | `count == 1` | [x] |
| 61 | `c2Witness` | `count == 2` weighted result | [x] |
| 62 | `c2Witness` | `count == 3` weighted result | [x] |
| 63 | `c2Witness` | count outside 1..3 returns two zero vectors | [x] |
| 64 | `c2Div` | nonzero divisor | [x] |
| 65 | `c2Div` | zero divisor; compare C IEEE-754 result | [x] |
| 66 | `c2Norm` | nonzero vector | [x] |
| 67 | `c2Norm` | zero vector; compare C IEEE-754 result | [x] |
| 68 | `c2L` | `count == 1` | [x] |
| 69 | `c2L` | `count == 2` weighted result | [x] |
| 70 | `c2L` | other count | [x] |
| 71 | `c2MulrvT` | arbitrary rotation/vector | [x] |
| 72 | `c2GJK` | circle-circle, identity transforms, `use_radius == 0` | [x] |
| 73 | `c2GJK` | circle-AABB, identity transforms, `use_radius == 0` | [x] |
| 74 | `c2GJK` | circle-capsule, identity transforms, `use_radius == 0` | [x] |
| 75 | `c2GJK` | AABB-circle, identity transforms, `use_radius == 0` | [x] |
| 76 | `c2GJK` | AABB-AABB, identity transforms, `use_radius == 0` | [x] |
| 77 | `c2GJK` | AABB-capsule, identity transforms, `use_radius == 0` | [x] |
| 78 | `c2GJK` | capsule-circle, identity transforms, `use_radius == 0` | [x] |
| 79 | `c2GJK` | capsule-AABB, identity transforms, `use_radius == 0` | [x] |
| 80 | `c2GJK` | capsule-capsule, identity transforms, `use_radius == 0` | [x] |
| 81 | `c2GJK` | circle-circle, identity transforms, `use_radius != 0` | [x] |
| 82 | `c2GJK` | circle-AABB, identity transforms, `use_radius != 0` | [x] |
| 83 | `c2GJK` | circle-capsule, identity transforms, `use_radius != 0` | [x] |
| 84 | `c2GJK` | AABB-circle, identity transforms, `use_radius != 0` | [x] |
| 85 | `c2GJK` | AABB-AABB, identity transforms, `use_radius != 0` | [x] |
| 86 | `c2GJK` | AABB-capsule, identity transforms, `use_radius != 0` | [x] |
| 87 | `c2GJK` | capsule-circle, identity transforms, `use_radius != 0` | [x] |
| 88 | `c2GJK` | capsule-AABB, identity transforms, `use_radius != 0` | [x] |
| 89 | `c2GJK` | capsule-capsule, identity transforms, `use_radius != 0` | [x] |
| 90 | `c2GJK` | only A transform provided; all nine ordered shape pairs | [x] |
| 91 | `c2GJK` | only B transform provided; all nine ordered shape pairs | [x] |
| 92 | `c2GJK` | both transforms provided; all nine ordered shape pairs | [x] |
| 93 | `c2GJK` | all optional output pointers null | [x] |
| 94 | `c2GJK` | only `outA` nonnull | [x] |
| 95 | `c2GJK` | only `outB` nonnull | [x] |
| 96 | `c2GJK` | only `iterations` nonnull | [x] |
| 97 | `c2GJK` | `outA` and `outB` nonnull; `iterations` null | [x] |
| 98 | `c2GJK` | `outA` and `iterations` nonnull; `outB` null | [x] |
| 99 | `c2GJK` | `outB` and `iterations` nonnull; `outA` null | [x] |
| 100 | `c2GJK` | nonnull empty cache (`count == 0`), then cache write | [x] |
| 101 | `c2GJK` | warm cache produced by a prior call; all ordered shape pairs | [x] |
| 102 | `c2GJK` | cached simplex `count == 1` | [x] |
| 103 | `c2GJK` | cached simplex `count == 2` | [x] |
| 104 | `c2GJK` | cached simplex `count == 3` | [x] |
| 105 | `c2GJK` | cached metric invalidation condition true, reinitialize simplex | [x] |
| 106 | `c2AABBtoAABB` | B left of A | [x] |
| 107 | `c2AABBtoAABB` | B right of A | [x] |
| 108 | `c2AABBtoAABB` | B below A | [x] |
| 109 | `c2AABBtoAABB` | B above A | [x] |
| 110 | `c2AABBtoAABB` | overlap or edge touch | [x] |
| 111 | `c2AABBtoCapsule` | separated | [x] |
| 112 | `c2AABBtoCapsule` | colliding/touching | [x] |
| 113 | `c2CapsuletoCapsule` | separated | [x] |
| 114 | `c2CapsuletoCapsule` | colliding/touching | [x] |
| 115 | `c2CircletoCircle` | strict overlap | [x] |
| 116 | `c2CircletoCircle` | exact tangent (`d2 == r2`) | [x] |
| 117 | `c2CircletoCircle` | separated | [x] |
| 118 | `c2CircletoAABB` | center inside box | [x] |
| 119 | `c2CircletoAABB` | nearest point on edge | [x] |
| 120 | `c2CircletoAABB` | nearest point at corner | [x] |
| 121 | `c2CircletoAABB` | exact tangent (`d2 == r2`) | [x] |
| 122 | `c2CircletoAABB` | separated | [x] |
| 123 | `c2CircletoCapsule` | before A endpoint (`da < 0`), overlap | [x] |
| 124 | `c2CircletoCapsule` | before A endpoint (`da < 0`), separated | [x] |
| 125 | `c2CircletoCapsule` | segment interior (`da >= 0 && db < 0`), overlap | [x] |
| 126 | `c2CircletoCapsule` | segment interior (`da >= 0 && db < 0`), separated | [x] |
| 127 | `c2CircletoCapsule` | beyond B endpoint (`db >= 0`), overlap | [x] |
| 128 | `c2CircletoCapsule` | beyond B endpoint (`db >= 0`), separated | [x] |
| 129 | `c2Collided` | circle-circle dispatch | [x] |
| 130 | `c2Collided` | circle-AABB dispatch | [x] |
| 131 | `c2Collided` | circle-capsule dispatch | [x] |
| 132 | `c2Collided` | AABB-circle reversed dispatch | [x] |
| 133 | `c2Collided` | AABB-AABB dispatch | [x] |
| 134 | `c2Collided` | AABB-capsule dispatch | [x] |
| 135 | `c2Collided` | capsule-circle reversed dispatch | [x] |
| 136 | `c2Collided` | capsule-AABB reversed dispatch | [x] |
| 137 | `c2Collided` | capsule-capsule dispatch | [x] |
| 138 | `aabb` | randomized input AABBs, including no collision | [x] |
| 139 | `aabb` | input collides with fixed circle (bit 0) | [x] |
| 140 | `aabb` | input collides with fixed AABB (bit 1) | [x] |
| 141 | `aabb` | input collides with fixed capsule (bit 2) | [x] |
| 142 | `c2GJK` | circle-circle, `use_radius != 0`, guaranteed overlap | [x] |
| 143 | `c2GJK` | circle-circle, `use_radius != 0`, guaranteed separation | [x] |
| 144 | `c2GJK` | circle-AABB, `use_radius != 0`, guaranteed overlap | [x] |
| 145 | `c2GJK` | circle-AABB, `use_radius != 0`, guaranteed separation | [x] |
| 146 | `c2GJK` | circle-capsule, `use_radius != 0`, guaranteed overlap | [x] |
| 147 | `c2GJK` | circle-capsule, `use_radius != 0`, guaranteed separation | [x] |
| 148 | `c2GJK` | AABB-circle, `use_radius != 0`, guaranteed overlap | [x] |
| 149 | `c2GJK` | AABB-circle, `use_radius != 0`, guaranteed separation | [x] |
| 150 | `c2GJK` | AABB-AABB, `use_radius != 0`, guaranteed overlap | [x] |
| 151 | `c2GJK` | AABB-AABB, `use_radius != 0`, guaranteed separation | [x] |
| 152 | `c2GJK` | AABB-capsule, `use_radius != 0`, guaranteed overlap | [x] |
| 153 | `c2GJK` | AABB-capsule, `use_radius != 0`, guaranteed separation | [x] |
| 154 | `c2GJK` | capsule-circle, `use_radius != 0`, guaranteed overlap | [x] |
| 155 | `c2GJK` | capsule-circle, `use_radius != 0`, guaranteed separation | [x] |
| 156 | `c2GJK` | capsule-AABB, `use_radius != 0`, guaranteed overlap | [x] |
| 157 | `c2GJK` | capsule-AABB, `use_radius != 0`, guaranteed separation | [x] |
| 158 | `c2GJK` | capsule-capsule, `use_radius != 0`, guaranteed overlap | [x] |
| 159 | `c2GJK` | capsule-capsule, `use_radius != 0`, guaranteed separation | [x] |
| 160 | vector/scalar low-level entry points | signed zero, infinities, subnormals, maximum finite values, and NaN payloads | [x] |
| 161 | circle collision entry points | zero and negative radii | [x] |
| 162 | AABB entry points | zero-area and inverted min/max boxes | [x] |
| 163 | capsule entry points | coincident endpoints (zero-length segment) | [x] |
| 164 | capsule entry points | zero and negative radii | [x] |
| 165 | `c2GJK` | degenerate circle/AABB/capsule shapes, with and without radius | [x] |
| 166 | collision entry points, `aabb` | NaN and infinity components; compare C comparison semantics | [x] |
