# Valid configuration surface

Mechanically derived from every exported function and each `if`/`switch` branch
in `c_src/src/lib.c`. There are no Cargo features and no executable target.
Randomized rows use a fixed seed and include finite values plus signed zero,
infinities, NaNs, and boundary/tie values where the C operation defines them.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `c2V` | arbitrary `x`, `y` bit patterns | [x] |
| 2 | `c2Mulvs`, `c2Sub`, `c2Add`, `c2Dot`, `c2Det2`, `c2Neg`, `c2Skew`, `c2CCW90` | arbitrary vectors/scalar; includes signed zero and non-finite values | [x] |
| 3 | `c2Maxv` | independently `a.x > b.x` / false and `a.y > b.y` / false, including equality and NaN comparisons | [x] |
| 4 | `c2Minv` | independently `a.x < b.x` / false and `a.y < b.y` / false, including equality and NaN comparisons | [x] |
| 5 | `c2Clampv` | each component below, within, or above `[lo, hi]`, including boundary equality | [x] |
| 6 | `c2RotIdentity`, `c2xIdentity` | no-input identity constructors | [x] |
| 7 | `c2BBVerts` | arbitrary AABB, four output vertices | [x] |
| 8 | `c2MakeProxy` | circle shape (`type=0`) | [x] |
| 9 | `c2MakeProxy` | AABB shape (`type=1`) | [x] |
| 10 | `c2MakeProxy` | capsule shape (`type=2`) | [x] |
| 11 | `c2Len`, `c2Norm` | nonzero finite vector | [x] |
| 12 | `c2Len`, `c2Norm` | zero/signed-zero vector and non-finite components | [x] |
| 13 | `c2Div` | finite nonzero divisor | [x] |
| 14 | `c2Div` | positive zero, negative zero, infinity, and NaN divisor | [x] |
| 15 | `c2GJKSimplexMetric` | simplex count 1 | [x] |
| 16 | `c2GJKSimplexMetric` | simplex count 2 (segment length) | [x] |
| 17 | `c2GJKSimplexMetric` | simplex count 3 (signed triangle determinant) | [x] |
| 18 | `c2Mulrv`, `c2MulrvT`, `c2Mulxv` | arbitrary rotations/transforms and vectors | [x] |
| 19 | `c22` | `v <= 0` selects vertex A | [x] |
| 20 | `c22` | `v > 0 && u <= 0` selects vertex B | [x] |
| 21 | `c22` | `v > 0 && u > 0` keeps edge AB | [x] |
| 22 | `c23` | vertex-A Voronoi branch | [x] |
| 23 | `c23` | vertex-B Voronoi branch | [x] |
| 24 | `c23` | vertex-C Voronoi branch | [x] |
| 25 | `c23` | edge-AB Voronoi branch | [x] |
| 26 | `c23` | edge-BC Voronoi branch | [x] |
| 27 | `c23` | edge-CA Voronoi branch | [x] |
| 28 | `c23` | triangle-interior fallback branch | [x] |
| 29 | `c2D` | simplex count 1 | [x] |
| 30 | `c2D` | simplex count 2 and positive determinant selects `c2Skew` | [x] |
| 31 | `c2D` | simplex count 2 and nonpositive determinant selects `c2CCW90` | [x] |
| 32 | `c2Support` | one vertex | [x] |
| 33 | `c2Support` | multiple vertices with a unique later maximum | [x] |
| 34 | `c2Support` | multiple vertices with tied maxima; first maximum retained | [x] |
| 35 | `c2Witness` | simplex count 1 | [x] |
| 36 | `c2Witness` | simplex count 2 weighted witness | [x] |
| 37 | `c2Witness` | simplex count 3 weighted witness | [x] |
| 38 | `c2L` | simplex count 1 | [x] |
| 39 | `c2L` | simplex count 2 weighted closest point | [x] |
| 40 | `c2GJK` | each ordered shape pair in `{circle,AABB,capsule}²`, identity transforms via null pointers, `use_radius=0`, separated/touching/overlapping data | [x] |
| 41 | `c2GJK` | each ordered shape pair, identity transforms, `use_radius!=0`, separated/touching/overlapping data | [x] |
| 42 | `c2GJK` | all four null/provided transform-pointer combinations with translated/rotated shapes | [x] |
| 43 | `c2GJK` | all optional output-pointer combinations (`outA`, `outB`, `iterations`) including all null and all provided | [x] |
| 44 | `c2GJK` | null cache (cold start) | [x] |
| 45 | `c2GJK` | provided zero-count cache (cold start then cache write) | [x] |
| 46 | `c2GJK` | provided populated reusable cache (cache read then write) | [x] |
| 47 | `c2GJK` | populated cache rejected by the metric guard, forcing cold start | [x] |
| 48 | `c2GJK` | loop exits by simplex hit (`count == 3`) | [x] |
| 49 | `c2GJK` | loop exits by duplicate support pair / no progress | [x] |
| 50 | `c2GJK` | radius adjustment keeps separated witnesses (`dist > rA+rB` and epsilon) | [x] |
| 51 | `c2GJK` | radius adjustment collapses witnesses for overlap/touch/epsilon distance | [x] |
| 52 | `c2AABBtoAABB` | overlap/intersection including touching boundary | [x] |
| 53 | `c2AABBtoAABB` | separated along each of the four tested axes | [x] |
| 54 | `c2AABBtoCapsule` | collision and separation | [x] |
| 55 | `c2CapsuletoCapsule` | collision and separation | [x] |
| 56 | `c2CircletoCircle` | overlap, exact tangent (strict false), and separation | [x] |
| 57 | `c2CircletoAABB` | center below/inside/above each slab; overlap, tangent, separation | [x] |
| 58 | `c2CircletoCapsule` | closest point before endpoint A (`da < 0`) | [x] |
| 59 | `c2CircletoCapsule` | closest point on segment (`da >= 0 && db < 0`) | [x] |
| 60 | `c2CircletoCapsule` | closest point after endpoint B (`db >= 0`) | [x] |
| 61 | `c2Collided` | all 9 ordered valid type pairs, exercising argument reversal branches | [x] |
| 62 | `reverse_collide` | randomized `(x,y,r)` plus boundary/tangent values spanning returned collision bitmasks | [x] |
