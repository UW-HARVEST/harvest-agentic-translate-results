# Configuration surface

Derived from every exported definition in `../c_src/src/lib.c`, its `switch`
cases, `if`/`else if` branches, nullable-option checks, shape tags, simplex
counts, support counts, and GJK constants. There are no Cargo features declared
in `Cargo.toml`; therefore the only semantic feature set is the empty set
(tested both normally and with `--no-default-features`).

| # | entry point(s) | configuration (options set + input shape) | |
|---:|----------------|--------------------------------------------|:-:|
| 1 | `c2V` | Arbitrary raw `f32` pairs, including signed zero, infinities, and NaNs | [x] |
| 2 | `c2Mulvs` | Arbitrary vector/scalar multiplication, including zero and negative scalar | [x] |
| 3 | `c2Maxv` | `a.x>b.x`, `a.y>b.y` (both components selected from `a`) | [x] |
| 4 | `c2Maxv` | `a.x>b.x`, `a.y<=b.y` (mixed selection) | [x] |
| 5 | `c2Maxv` | `a.x<=b.x`, `a.y>b.y` (mixed selection) | [x] |
| 6 | `c2Maxv` | `a.x<=b.x`, `a.y<=b.y` (both components selected from `b`) | [x] |
| 7 | `c2Maxv` | Equality and unordered/NaN comparisons, which select `b` | [x] |
| 8 | `c2Minv` | `a.x<b.x`, `a.y<b.y` (both components selected from `a`) | [x] |
| 9 | `c2Minv` | `a.x<b.x`, `a.y>=b.y` (mixed selection) | [x] |
| 10 | `c2Minv` | `a.x>=b.x`, `a.y<b.y` (mixed selection) | [x] |
| 11 | `c2Minv` | `a.x>=b.x`, `a.y>=b.y` (both components selected from `b`) | [x] |
| 12 | `c2Minv` | Equality and unordered/NaN comparisons, which select `b` | [x] |
| 13 | `c2Clampv` | x below / y below bounds | [x] |
| 14 | `c2Clampv` | x below / y inside bounds | [x] |
| 15 | `c2Clampv` | x below / y above bounds | [x] |
| 16 | `c2Clampv` | x inside / y below bounds | [x] |
| 17 | `c2Clampv` | x inside / y inside bounds | [x] |
| 18 | `c2Clampv` | x inside / y above bounds | [x] |
| 19 | `c2Clampv` | x above / y below bounds | [x] |
| 20 | `c2Clampv` | x above / y inside bounds | [x] |
| 21 | `c2Clampv` | x above / y above bounds | [x] |
| 22 | `c2Sub` | Arbitrary vector subtraction | [x] |
| 23 | `c2Dot` | Arbitrary vectors, covering positive, negative, zero, and cancellation results | [x] |
| 24 | `c2RotIdentity`, `c2xIdentity` | No-input identity constructors | [x] |
| 25 | `c2BBVerts` | Nondegenerate, degenerate, and inverted AABBs; four output slots | [x] |
| 26 | `c2MakeProxy` | Circle tag: one vertex and circle radius | [x] |
| 27 | `c2MakeProxy` | AABB tag: four vertices and zero radius | [x] |
| 28 | `c2MakeProxy` | Capsule tag: two vertices and capsule radius | [x] |
| 29 | `c2Len` | Zero and nonzero vectors | [x] |
| 30 | `c2Det2` | Positive, negative, and zero determinant | [x] |
| 31 | `c2GJKSimplexMetric` | simplex count 1 | [x] |
| 32 | `c2GJKSimplexMetric` | simplex count 2 (segment length) | [x] |
| 33 | `c2GJKSimplexMetric` | simplex count 3 (signed triangle determinant) | [x] |
| 34 | `c2Mulrv` | General rotation pair and vector | [x] |
| 35 | `c2Add` | Arbitrary vector addition | [x] |
| 36 | `c2Mulxv` | General translation/rotation transform and vector | [x] |
| 37 | `c22` | `v<=0`: reduce to original vertex `a` | [x] |
| 38 | `c22` | `v>0 && u<=0`: reduce to vertex `b` | [x] |
| 39 | `c22` | `u>0 && v>0`: retain two-point edge | [x] |
| 40 | `c23` | `vAB<=0 && uCA<=0`: vertex A Voronoi region | [x] |
| 41 | `c23` | `uAB<=0 && vBC<=0`: vertex B Voronoi region | [x] |
| 42 | `c23` | `uBC<=0 && vCA<=0`: vertex C Voronoi region | [x] |
| 43 | `c23` | AB edge region (`wABC<=0`) | [x] |
| 44 | `c23` | BC edge region (`uABC<=0`) | [x] |
| 45 | `c23` | CA edge region (`vABC<=0`) | [x] |
| 46 | `c23` | Triangle interior region; retain three points | [x] |
| 47 | `c2Neg` | Arbitrary vector negation, including signed zero | [x] |
| 48 | `c2Skew` | Arbitrary vector clockwise/perpendicular mapping | [x] |
| 49 | `c2CCW90` | Arbitrary vector opposite perpendicular mapping | [x] |
| 50 | `c2D` | simplex count 1 | [x] |
| 51 | `c2D` | simplex count 2 and positive determinant branch | [x] |
| 52 | `c2D` | simplex count 2 and nonpositive determinant branch | [x] |
| 53 | `c2D` | simplex count 3 terminal zero direction | [x] |
| 54 | `c2Support` | one vertex | [x] |
| 55 | `c2Support` | many vertices with a unique strict maximum | [x] |
| 56 | `c2Support` | many vertices with tied maxima; first maximum retained | [x] |
| 57 | `c2Witness` | simplex count 1 | [x] |
| 58 | `c2Witness` | simplex count 2 with weighted interpolation | [x] |
| 59 | `c2Witness` | simplex count 3 with weighted interpolation | [x] |
| 60 | `c2Div` | finite nonzero divisor | [x] |
| 61 | `c2Div` | zero, infinity, and NaN divisors | [x] |
| 62 | `c2Norm` | nonzero vector | [x] |
| 63 | `c2Norm` | zero and non-finite vector | [x] |
| 64 | `c2L` | simplex count 1 | [x] |
| 65 | `c2L` | simplex count 2 with weighted interpolation | [x] |
| 66 | `c2L` | simplex count 3 terminal zero vector | [x] |
| 67 | `c2MulrvT` | General inverse-rotation pair and vector | [x] |
| 68 | `c2GJK` | circle/circle, identity transforms, `use_radius=0`, separated | [x] |
| 69 | `c2GJK` | circle/circle, identity transforms, `use_radius!=0`, separated beyond radius sum | [x] |
| 70 | `c2GJK` | circle/circle, identity transforms, `use_radius!=0`, touching/overlapping | [x] |
| 71 | `c2GJK` | circle/AABB ordered shape pair | [x] |
| 72 | `c2GJK` | circle/capsule ordered shape pair | [x] |
| 73 | `c2GJK` | AABB/circle ordered shape pair | [x] |
| 74 | `c2GJK` | AABB/AABB ordered shape pair | [x] |
| 75 | `c2GJK` | AABB/capsule ordered shape pair | [x] |
| 76 | `c2GJK` | capsule/circle ordered shape pair | [x] |
| 77 | `c2GJK` | capsule/AABB ordered shape pair | [x] |
| 78 | `c2GJK` | capsule/capsule ordered shape pair | [x] |
| 79 | `c2GJK` | non-null transform for A only | [x] |
| 80 | `c2GJK` | non-null transform for B only | [x] |
| 81 | `c2GJK` | non-null transforms for both A and B | [x] |
| 82 | `c2GJK` | `outA`, `outB`, and `iterations` all non-null | [x] |
| 83 | `c2GJK` | `outA` null; other outputs non-null | [x] |
| 84 | `c2GJK` | `outB` null; other outputs non-null | [x] |
| 85 | `c2GJK` | `iterations` null; witness outputs non-null | [x] |
| 86 | `c2GJK` | all three result pointers null | [x] |
| 87 | `c2GJK` | cache pointer null | [x] |
| 88 | `c2GJK` | cache present with `count==0`; initialize from vertex zero and write cache | [x] |
| 89 | `c2GJK` | readable warm cache with simplex count 1 | [x] |
| 90 | `c2GJK` | readable warm cache with simplex count 2 | [x] |
| 91 | `c2GJK` | readable warm cache with simplex count 3 | [x] |
| 92 | `c2GJK` | cache metric rejection condition using factor `2.0` and threshold `-1.0e8`; restart simplex | [x] |
| 93 | `c2GJK` | initial `FLT_MAX` distance gate and randomized geometries; returned iteration count never exceeds cap 20 | [x] |
| 94 | `c2GJK` | simplex reaches count 3 (`hit` branch) | [x] |
| 95 | `c2GJK` | search direction squared is below `FLT_EPSILON^2` | [x] |
| 96 | `c2GJK` | duplicate support pair terminates search | [x] |
| 97 | `c2GJK` | radius path with `dist > rA+rB` and `dist > FLT_EPSILON` | [x] |
| 98 | `c2GJK` | radius path else-branch (`dist <= rA+rB` or epsilon) collapses witnesses to midpoint | [x] |
| 99 | `c2GJK` | radius adjustment makes witness coordinates exactly equal and forces distance zero | [x] |
| 100 | `c2AABBtoAABB` | strict overlap | [x] |
| 101 | `c2AABBtoAABB` | boundary touching (comparisons are strict `<`) | [x] |
| 102 | `c2AABBtoAABB` | separated on x | [x] |
| 103 | `c2AABBtoAABB` | separated on y | [x] |
| 104 | `c2AABBtoCapsule` | non-colliding | [x] |
| 105 | `c2AABBtoCapsule` | colliding/touching under GJK radius handling | [x] |
| 106 | `c2CapsuletoCapsule` | non-colliding | [x] |
| 107 | `c2CapsuletoCapsule` | colliding/touching under GJK radius handling | [x] |
| 108 | `c2CircletoCircle` | separated | [x] |
| 109 | `c2CircletoCircle` | exact tangency (strict `<` returns false) | [x] |
| 110 | `c2CircletoCircle` | overlap | [x] |
| 111 | `c2CircletoAABB` | center inside box | [x] |
| 112 | `c2CircletoAABB` | nearest point on side | [x] |
| 113 | `c2CircletoAABB` | nearest point at corner | [x] |
| 114 | `c2CircletoAABB` | exact tangency (strict `<` returns false) | [x] |
| 115 | `c2CircletoAABB` | separated | [x] |
| 116 | `c2CircletoCapsule` | projection before endpoint A, separated | [x] |
| 117 | `c2CircletoCapsule` | projection before endpoint A, overlapping | [x] |
| 118 | `c2CircletoCapsule` | projection on segment interior, separated | [x] |
| 119 | `c2CircletoCapsule` | projection on segment interior, overlapping | [x] |
| 120 | `c2CircletoCapsule` | projection after endpoint B, separated | [x] |
| 121 | `c2CircletoCapsule` | projection after endpoint B, overlapping | [x] |
| 122 | `c2Collided` | circle/circle dispatch | [x] |
| 123 | `c2Collided` | circle/AABB dispatch | [x] |
| 124 | `c2Collided` | circle/capsule dispatch | [x] |
| 125 | `c2Collided` | AABB/circle reversed dispatch | [x] |
| 126 | `c2Collided` | AABB/AABB dispatch | [x] |
| 127 | `c2Collided` | AABB/capsule dispatch | [x] |
| 128 | `c2Collided` | capsule/circle reversed dispatch | [x] |
| 129 | `c2Collided` | capsule/AABB reversed dispatch | [x] |
| 130 | `c2Collided` | capsule/capsule dispatch | [x] |
| 131 | `aabb` | randomized and boundary AABB coordinates across all three encoded collision bits | [x] |
| 132 | all exports | empty Cargo feature set, normal/default invocation | [x] |
| 133 | all exports | empty Cargo feature set, explicit `--no-default-features` invocation | [x] |
