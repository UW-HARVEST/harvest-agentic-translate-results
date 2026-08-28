# CONFIGS.md — Phase B valid-configuration surface table

Mechanically derived from the branch structure of `c_src/src/lib.c` and the
public symbol list (`SYMBOLS.md`). Every row is a *combination* of the axes the C
code actually distinguishes; each is driven through **both** `.so`s with many
randomized inputs (fixed seed) and compared bit-for-bit.

## The axes the C code branches on

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| `C2_TYPE` for A | `CAPSULE=0`, `CIRCLE=1`, `AABB=2`, `POLY=3` (+ out-of-range) | `c2Collide` :855, `c2MakeProxy` :126, `ptr_from_parts` :906 |
| `C2_TYPE` for B | same | `c2Collide` :857/:870/:884 |
| `c2GJK use_radius` | `0` / non-zero | :541 |
| `c2GJK ax_ptr` | `NULL` (⇒ identity) / non-NULL rotation+translation | :427 |
| `c2GJK bx_ptr` | `NULL` / non-NULL | :431 |
| `c2GJK outA/outB/iterations` | `NULL` / non-NULL | :569,:571,:573 |
| `c2GJK cache` | `NULL` / cold (`count==0`) / warm (`count`=1,2,3) / warm+stale metric | :442,:443,:464,:559 |
| separation regime | deep overlap (`hit`) / shallow overlap / exact touch / separated | :500,:538,:544 |
| `c2Simplex.count` | 0,1,2,3,4+ (and negative) | `c22`,`c23`,`c2D`,`c2L`,`c2Witness`,`c2GJKSimplexMetric` |
| `c23` region | 7 barycentric regions | :304,:308,:313,:318,:323,:330,:337 |
| `c22` region | 3 regions (`v<=0`, `u<=0`, interior) | :273,:277,:282 |
| `c2Poly.count` | 1,2,3,4,5,6,7,8 (`verts[8]` max) | :716,:753, index wrap `i+1==count?0` |
| poly winding / convexity | CCW convex (normals outward) vs other | `c2Norms` :815 |
| `c2CapsuletoPolyManifold` `code` | 0 (face of B) / 1 (capsule side h0) / 2 (capsule side h1) | :777 |
| `c2CircletoAABBManifold` branch | `d2 != 0` (outside) / `d2 == 0` (centre inside) | :604 |
| `c2CircletoAABBManifold` deep axis | `x_overlap < y_overlap` / else | :620 |
| `c2AABBtoAABBManifold` axis | `dx < dy` / else, then `d.x<0` / `d.y<0` sign | :672,:674,:683 |
| `c2Clip` result | `sp` = 0, 1, 2 — the reachable set. The "both on plane" double-push only fires when `sp` is still 0 (`d0<0`/`d1<0` are both false there), so `sp` maxes out at exactly 2 and `out[2]`/`out[3]` are never written | :210-226 |
| `c2KeepDeep` result | `cp` = 0,1,2 | :697-710 |
| value shape | zero, ±tiny/subnormal, ±1, large, `FLT_MAX`, `±inf`, `NaN`, `-0.0` | pervasive float compares |
| `c2Support` count | 1, 2, 4, 8 (circle/capsule/aabb/poly proxies) | :369 |

## Rows

`entry point(s)` = the exported symbol(s) driven directly through the `.so`.

### Group 1 — leaf vector / scalar primitives (lowest level)

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `c2V` | 2048 random f32 bit patterns (any class incl. NaN/inf/subnormal) | `cfg_leaf_v` | [x] |
| 2 | `c2Mulvs` | random vec × random scalar; plus 0, ±inf, NaN, `-0.0`, `FLT_MAX` scalars | `cfg_leaf_mulvs` | [x] |
| 3 | `c2Add`, `c2Sub` | random vec pairs; plus inf−inf, 0+(-0), NaN mixes | `cfg_leaf_addsub` | [x] |
| 4 | `c2Dot` | random vec pairs; plus 0·inf, inf+(−inf), mixed NaNs (both operands NaN) | `cfg_leaf_dot` | [x] |
| 5 | `c2Det2` | random vec pairs; plus overflow/cancellation, NaN mixes | `cfg_leaf_det2` | [x] |
| 6 | `c2Len` | random vec; plus zero, `FLT_MAX` (overflow to inf), subnormals, NaN | `cfg_leaf_len` | [x] |
| 7 | `c2Neg`, `c2Skew`, `c2CCW90`, `c2Absv` | random bit patterns incl. `±0.0`, `±NaN` (sign-bit semantics) | `cfg_leaf_unary` | [x] |
| 8 | `c2Maxv`, `c2Minv` | random pairs; plus equal, `±0.0`, one/both NaN (operand-order selection) | `cfg_leaf_minmax` | [x] |
| 9 | `c2Clampv` | random `a`,`lo`,`hi`; `lo > hi` inverted; NaN in each slot | `cfg_leaf_clampv` | [x] |
| 10 | `c2Div` | random vec ÷ random scalar; ÷0, ÷−0, ÷inf, ÷NaN | `cfg_leaf_div` | [x] |
| 11 | `c2Norm` | random vec; unit, huge, tiny/subnormal, zero, NaN | `cfg_leaf_norm` | [x] |
| 12 | `c2Dist` | random `c2h` × random `c2v`; NaN/inf in `n` and `d` | `cfg_leaf_dist` | [x] |
| 13 | `c2Intersect` | random `a`,`b`,`da`,`db`; `da==db`, `da==db==0`, opposite signs, NaN | `cfg_leaf_intersect` | [x] |
| 14 | `c2RotIdentity`, `c2xIdentity` | no inputs (constant) | `cfg_leaf_identity` | [x] |

### Group 2 — transforms

| # | entry point(s) | configuration | test | [x] |
|---|----------------|---------------|------|-----|
| 15 | `c2Mulrv` | identity rotation | `cfg_xform_rot` | [x] |
| 16 | `c2Mulrv` | random unit rotation (`c=cosθ,s=sinθ`) × random vec | `cfg_xform_rot` | [x] |
| 17 | `c2Mulrv` | non-unit / NaN / inf rotation components | `cfg_xform_rot` | [x] |
| 18 | `c2MulrvT` | identity, random unit, non-unit/NaN/inf rotation | `cfg_xform_rot` | [x] |
| 19 | `c2Mulxv` | identity `c2x`; random translation only; random rotation only; both | `cfg_xform_x` | [x] |
| 20 | `c2MulxvT` | same four configurations | `cfg_xform_x` | [x] |
| 21 | `c2Mulxv`/`c2MulxvT` | non-finite translation or rotation | `cfg_xform_x` | [x] |

### Group 3 — AABB / poly construction

| # | entry point(s) | configuration | test | [x] |
|---|----------------|---------------|------|-----|
| 22 | `c2BBVerts` | proper box (`min<max`) | `cfg_bbverts` | [x] |
| 23 | `c2BBVerts` | degenerate (`min==max`), inverted (`min>max`), non-finite | `cfg_bbverts` | [x] |
| 24 | `c2Norms` | `count`=1 | `cfg_norms` | [x] |
| 25 | `c2Norms` | `count`=2 | `cfg_norms` | [x] |
| 26 | `c2Norms` | `count`=3 (CCW triangle) | `cfg_norms` | [x] |
| 27 | `c2Norms` | `count`=4 (box, from `c2BBVerts`) | `cfg_norms` | [x] |
| 28 | `c2Norms` | `count`=8 (max) random convex polygon | `cfg_norms` | [x] |
| 29 | `c2Norms` | CW (reversed) winding — normals point inward | `cfg_norms` | [x] |
| 30 | `c2Norms` | duplicate consecutive verts (⇒ NaN normals) | `cfg_norms` | [x] |
| 31 | `c2PlaneAt` | every `i` in `[0,count)` for count 1..8 | `cfg_planeat` | [x] |
| 32 | `c2MakeProxy` | `type = CIRCLE` (⇒ radius=r, count=1) | `cfg_makeproxy` | [x] |
| 33 | `c2MakeProxy` | `type = AABB` (⇒ radius=0, count=4, 4 verts) | `cfg_makeproxy` | [x] |
| 34 | `c2MakeProxy` | `type = CAPSULE` (⇒ radius=r, count=2) | `cfg_makeproxy` | [x] |
| 35 | `c2Support` | count=1 / 2 / 4 / 8 verts, random direction | `cfg_support` | [x] |
| 36 | `c2Support` | ties (`dot == dmax`, strict `>` keeps the first) | `cfg_support` | [x] |

### Group 4 — simplex machinery (low-level GJK internals, exported)

| # | entry point(s) | configuration | test | [x] |
|---|----------------|---------------|------|-----|
| 37 | `c2GJKSimplexMetric` | `count = 2` (segment length) | `cfg_simplex_metric` | [x] |
| 38 | `c2GJKSimplexMetric` | `count = 3` (signed area) | `cfg_simplex_metric` | [x] |
| 39 | `c22` | region `v <= 0` | `cfg_c22` | [x] |
| 40 | `c22` | region `u <= 0` | `cfg_c22` | [x] |
| 41 | `c22` | interior region (`count` stays 2) | `cfg_c22` | [x] |
| 42 | `c22` | fully random simplex (all regions, 4096 samples) | `cfg_c22` | [x] |
| 43 | `c23` | region A (`vAB<=0 && uCA<=0`) | `cfg_c23` | [x] |
| 44 | `c23` | region B (`uAB<=0 && vBC<=0`) | `cfg_c23` | [x] |
| 45 | `c23` | region C (`uBC<=0 && vCA<=0`) | `cfg_c23` | [x] |
| 46 | `c23` | edge AB (`wABC<=0`) | `cfg_c23` | [x] |
| 47 | `c23` | edge BC (`uABC<=0`) | `cfg_c23` | [x] |
| 48 | `c23` | edge CA (`vABC<=0`) | `cfg_c23` | [x] |
| 49 | `c23` | interior (`count` stays 3) | `cfg_c23` | [x] |
| 50 | `c23` | fully random simplex (4096 samples, all 7 regions) | `cfg_c23` | [x] |
| 51 | `c2D` | `count = 1` | `cfg_c2d` | [x] |
| 52 | `c2D` | `count = 2`, `c2Det2 > 0` (skew branch) | `cfg_c2d` | [x] |
| 53 | `c2D` | `count = 2`, `c2Det2 <= 0` (CCW90 branch) | `cfg_c2d` | [x] |
| 54 | `c2L` | `count = 1` | `cfg_c2l` | [x] |
| 55 | `c2L` | `count = 2`, random `u`/`div` | `cfg_c2l` | [x] |
| 56 | `c2Witness` | `count = 1` | `cfg_witness` | [x] |
| 57 | `c2Witness` | `count = 2` | `cfg_witness` | [x] |
| 58 | `c2Witness` | `count = 3` | `cfg_witness` | [x] |

### Group 5 — `c2GJK` (the low-level distance entry point, all option combos)

Shape pairs enumerated over `{CIRCLE, AABB, CAPSULE}²` = 9, each with random
shapes in 4 separation regimes.

| # | entry point(s) | configuration | test | [x] |
|---|----------------|---------------|------|-----|
| 59 | `c2GJK` | `use_radius=0`, both transforms `NULL`, no cache — all 9 type pairs × random shapes | `cfg_gjk_matrix` | [x] |
| 60 | `c2GJK` | `use_radius=1`, both transforms `NULL`, no cache — all 9 type pairs | `cfg_gjk_matrix` | [x] |
| 61 | `c2GJK` | `use_radius=0`, `ax` non-NULL (rot+trans), `bx` NULL | `cfg_gjk_matrix` | [x] |
| 62 | `c2GJK` | `use_radius=0`, `ax` NULL, `bx` non-NULL | `cfg_gjk_matrix` | [x] |
| 63 | `c2GJK` | `use_radius=0`, both transforms non-NULL | `cfg_gjk_matrix` | [x] |
| 64 | `c2GJK` | `use_radius=1`, both transforms non-NULL | `cfg_gjk_matrix` | [x] |
| 65 | `c2GJK` | deep-overlap regime (⇒ `hit`, `dist = 0`) | `cfg_gjk_overlap` | [x] |
| 66 | `c2GJK` | exact-touch regime | `cfg_gjk_touch` | [x] |
| 67 | `c2GJK` | far-separated regime | `cfg_gjk_matrix` | [x] |
| 68 | `c2GJK` | `use_radius=1` + separated by more than `rA+rB` (shrink branch) | `cfg_gjk_use_radius` | [x] |
| 69 | `c2GJK` | `use_radius=1` + `dist <= rA+rB` (midpoint branch) | `cfg_gjk_use_radius` | [x] |
| 70 | `c2GJK` | `cache` non-NULL, cold (`count=0`) then re-used warm across 3 calls | `cfg_gjk_cache_cold` | [x] |
| 71 | `c2GJK` | `cache` warm with `count=1` and valid indices | `cfg_gjk_cache_warm` | [x] |
| 72 | `c2GJK` | `cache` warm with `count=2` | `cfg_gjk_cache_warm` | [x] |
| 73 | `c2GJK` | `cache` warm with `count=3` | `cfg_gjk_cache_warm` | [x] |
| 74 | `c2GJK` | `cache` warm with stale `metric` (triggers the `min<max*2 && metric<-1e8` test) | `cfg_gjk_cache_warm` | [x] |
| 75 | `c2GJK` | `iterations` non-NULL — iteration count compared for all of the above | `cfg_gjk_matrix` | [x] |
| 76 | `c2GJK` | `typeB = POLY` with a real `c2Poly` + `bx` (the only POLY-capable path) | `cfg_gjk_poly` | [x] |
| 77 | `c2GJK` | degenerate shapes: circle `r=0`, AABB `min==max`, capsule `a==b` | `cfg_gjk_degenerate` | [x] |
| 78 | `c2GJK` | the loop-exit branches and the whole **reachable** iteration range `0..=4`. The `iter < 20` cap is unreachable: the largest proxy `c2MakeProxy` builds has 4 vertices (AABB), so the duplicate-support test always fires first — measured max over 500 000 randomized configs (warm caches, degenerate and non-finite shapes included) is 4, and the test asserts both that every value 0..4 occurs and that 5+ never does | `cfg_gjk_iteration_cap`, `gjk_iteration_bound_is_four` | [x] |

### Group 6 — manifold producers (mid level)

| # | entry point(s) | configuration | test | [x] |
|---|----------------|---------------|------|-----|
| 79 | `c2CircletoCircleManifold` | random circles, overlapping | `cfg_circle_circle` | [x] |
| 80 | `c2CircletoCircleManifold` | separated / exact touch / coincident centres / r=0 / negative r | `cfg_circle_circle` | [x] |
| 81 | `c2CircletoAABBManifold` | circle centre outside box, overlapping (`d2 != 0`) | `cfg_circle_aabb` | [x] |
| 82 | `c2CircletoAABBManifold` | circle centre inside box (`d2 == 0`), `x_overlap < y_overlap` | `cfg_circle_aabb` | [x] |
| 83 | `c2CircletoAABBManifold` | circle centre inside box, `x_overlap >= y_overlap` | `cfg_circle_aabb` | [x] |
| 84 | `c2CircletoAABBManifold` | on-corner / on-edge / degenerate box / inverted box | `cfg_circle_aabb` | [x] |
| 85 | `c2CircletoCapsuleManifold` | overlapping, `d != 0` | `cfg_circle_capsule` | [x] |
| 86 | `c2CircletoCapsuleManifold` | `d == 0` (centre on the capsule spine) | `cfg_circle_capsule` | [x] |
| 87 | `c2CircletoCapsuleManifold` | separated; degenerate capsule (`a==b`); `r=0` | `cfg_circle_capsule` | [x] |
| 88 | `c2AABBtoAABBManifold` | overlap with `dx < dy`, `d.x < 0` | `cfg_aabb_aabb` | [x] |
| 89 | `c2AABBtoAABBManifold` | overlap with `dx < dy`, `d.x >= 0` | `cfg_aabb_aabb` | [x] |
| 90 | `c2AABBtoAABBManifold` | overlap with `dx >= dy`, `d.y < 0` | `cfg_aabb_aabb` | [x] |
| 91 | `c2AABBtoAABBManifold` | overlap with `dx >= dy`, `d.y >= 0` | `cfg_aabb_aabb` | [x] |
| 92 | `c2AABBtoAABBManifold` | identical boxes / touching edges / inverted boxes | `cfg_aabb_aabb` | [x] |
| 93 | `c2CapsuletoCapsuleManifold` | crossing (deep, `d==0`) | `cfg_capsule_capsule` | [x] |
| 94 | `c2CapsuletoCapsuleManifold` | parallel overlapping; end-to-end; separated | `cfg_capsule_capsule` | [x] |
| 95 | `c2CapsuletoCapsuleManifold` | degenerate (`a==b`) on one/both; `r=0` | `cfg_capsule_capsule` | [x] |
| 96 | `c2CapsuletoPolyManifold` | `bx = NULL`, box poly (count 4), `code = 0` (face) | `cfg_capsule_poly` | [x] |
| 97 | `c2CapsuletoPolyManifold` | `bx = NULL`, `code = 1` (capsule side plane h0) | `cfg_capsule_poly` | [x] |
| 98 | `c2CapsuletoPolyManifold` | `bx = NULL`, `code = 2` (capsule side plane h1) | `cfg_capsule_poly` | [x] |
| 99 | `c2CapsuletoPolyManifold` | `bx` non-NULL (rotated+translated poly), all codes | `cfg_capsule_poly` | [x] |
| 100 | `c2CapsuletoPolyManifold` | `d >= 1e-6` but `d < A.r` (shallow radius branch) | `cfg_capsule_poly` | [x] |
| 101 | `c2CapsuletoPolyManifold` | poly `count` = 3,4,5,6,7,8 random convex | `cfg_capsule_poly_counts` | [x] |
| 102 | `c2CapsuletoPolyManifold` | poly `count` = 1, 2 (degenerate) | `cfg_capsule_poly_counts` | [x] |
| 103 | `c2CapsuletoPolyManifold` | `c2KeepDeep` yielding `cp` = 0, 1 and 2 contact points | `cfg_capsule_poly` | [x] |
| 104 | `c2AABBtoCapsuleManifold` | capsule crossing the box (deep) | `cfg_aabb_capsule` | [x] |
| 105 | `c2AABBtoCapsuleManifold` | capsule outside, shallow (radius branch) | `cfg_aabb_capsule` | [x] |
| 106 | `c2AABBtoCapsuleManifold` | capsule separated (reject, but `n` still negated) | `cfg_aabb_capsule` | [x] |
| 107 | `c2AABBtoCapsuleManifold` | degenerate box (`min==max`) ⇒ NaN poly normals ⇒ `verts[-1]` OOB read | `cfg_aabb_capsule_degenerate` | [x] |
| 108 | `c2AABBtoCapsuleManifold` | degenerate capsule (`a==b`) | `cfg_aabb_capsule_degenerate` | [x] |

### Group 7 — dispatchers (top level)

| # | entry point(s) | configuration | test | [x] |
|---|----------------|---------------|------|-----|
| 109 | `ptr_from_parts` | `typ = CIRCLE` — returned struct contents verified | `cfg_ptr_from_parts` | [x] |
| 110 | `ptr_from_parts` | `typ = AABB` | `cfg_ptr_from_parts` | [x] |
| 111 | `ptr_from_parts` | `typ = CAPSULE` | `cfg_ptr_from_parts` | [x] |
| 112 | `c2Collide` | all 9 handled `(typeA,typeB)` pairs × random shapes × 4 separation regimes | `cfg_collide_matrix` | [x] |
| 113 | `c2Collide` | `m` pre-seeded with a known sentinel to observe untouched fields & stale-`n` negation | `cfg_collide_matrix` | [x] |
| 114 | `omni_manifold` | all 16 `(type_a,type_b)` pairs incl. `POLY` × random parameters | `cfg_omni_matrix` | [x] |
| 115 | `omni_manifold` | CIRCLE/CIRCLE random (overlap-biased) | `cfg_omni_matrix` | [x] |
| 116 | `omni_manifold` | CIRCLE/AABB and AABB/CIRCLE random | `cfg_omni_matrix` | [x] |
| 117 | `omni_manifold` | CIRCLE/CAPSULE and CAPSULE/CIRCLE random | `cfg_omni_matrix` | [x] |
| 118 | `omni_manifold` | AABB/AABB random | `cfg_omni_matrix` | [x] |
| 119 | `omni_manifold` | AABB/CAPSULE and CAPSULE/AABB random | `cfg_omni_matrix` | [x] |
| 120 | `omni_manifold` | CAPSULE/CAPSULE random | `cfg_omni_matrix` | [x] |
| 121 | `omni_manifold` | grid sweep on a small integer lattice (exhaustive, all 9 pairs) | `cfg_omni_lattice` | [x] |
| 122 | `omni_manifold` | axis-aligned / degenerate lattice (values from {−1,0,1} incl. `min==max`) | `cfg_omni_lattice` | [x] |
| 123 | `omni_manifold` | non-finite parameters (`NaN`, `±inf`, `-0.0`, subnormal, `FLT_MAX`) | `cfg_omni_extreme` | [x] |

### Group 8 — NaN operand-position surface (`tests/phase_c_nan_order.rs`)

x86 SSE scalar ops choose *which* NaN to propagate by operand position, so gcc's
register allocation fixes the answer per expression. Random NaNs mostly collapse
onto the default QNaN `0x7fc00000` and hide a wrong choice, so each row below
sweeps the **full cross-product of a function's float inputs** over a pool of
5 pairwise-distinct values (one ordinary, +qNaN, −qNaN, +sNaN, −sNaN).

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 124 | `c2Add`, `c2Sub`, `c2Dot`, `c2Det2`, `c2Maxv`, `c2Minv` | all 5⁴ distinct-NaN assignments to the 4 input floats | `nan_order_binary_vec_ops` | [x] |
| 125 | `c2Neg`, `c2Skew`, `c2CCW90`, `c2Absv`, `c2Len`, `c2Norm` | all 5² assignments | `nan_order_unary_vec_ops` | [x] |
| 126 | `c2Mulvs`, `c2Div` | all 5³ assignments (vector × scalar) | `nan_order_unary_vec_ops` | [x] |
| 127 | `c2Clampv` | all 5⁶ assignments (`a`, `lo`, `hi`) | `nan_order_clampv` | [x] |
| 128 | `c2Dist` | all 5⁵ assignments (`h.n`, `h.d`, `p`) | `nan_order_dist` | [x] |
| 129 | `c2Mulrv`, `c2MulrvT` | all 5⁴ assignments (rotation × vector) | `nan_order_rotations` | [x] |
| 130 | `c2Mulxv`, `c2MulxvT` | all 5⁶ assignments (transform × vector) | `nan_order_rotations` | [x] |
| 131 | `c2Intersect` | all 5⁶ assignments (`a`, `b`, `da`, `db`) | `nan_order_intersect` | [x] |
| 132 | `c2Witness`, `c2L` | `div` × `a.u` × `b.u` × `c.u` over the pool, for `count` 0..=4 — this is the row that pins `mulss dst = u, src = den` | `nan_order_witness_and_l` | [x] |
| 133 | `c2Witness`, `c2L`, `c2D`, `c2GJKSimplexMetric` | NaN simplex *points* combined with NaN weights, `count` 1..=3 | `nan_order_witness_and_l` | [x] |
| 134 | `c22`, `c23` | all 5⁶ assignments to the three simplex points | `nan_order_simplex_reduction` | [x] |
| 135 | `c2Support`, `c2Norms`, `c2PlaneAt` | NaN vertex arrays, `count` ∈ {1,4,8}, every index | `nan_order_poly_helpers` | [x] |
| 136 | all six manifold producers | NaN in every shape field (5⁵ assignments) | `nan_order_manifolds` | [x] |
| 137 | `c2GJK` | NaN shapes × 9 type pairs × `use_radius` × NULL/non-NULL transforms × warm cache with NaN metric/div | `nan_order_gjk` | [x] |
| 138 | `omni_manifold` | NaN parameters × all 16 type pairs | `nan_order_omni` | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. Verified by

```sh
grep -n '\[features\]' translation/Cargo.toml   # no match
```

The `x86_mul`/`x86_add`/`x86_sub`/`fneg` helpers in `src/lib.rs` pin down which
`NaN` an expression propagates and stop LLVM from fusing a sign flip into
neighbouring arithmetic, so they are codegen-sensitive. `run_all.sh` therefore
rebuilds the `cdylib` at **opt-level 0, 1, 2, 3, s and z** and re-runs the whole
suite plus the symbol diff against each one. All 121 tests pass in all six.

## Harness invariant that these rows depend on

`c2MakeProxy` has no `case C2_TYPE_POLY`, so `c2GJK` reads its `c2Proxy` locals
uninitialised on the poly path — which the public API reaches via
`omni_manifold(AABB, CAPSULE)` → … → `c2CapsuletoPolyManifold` → `c2GJK`. On a
pristine stack (any normal C program) those bytes are zero, and that is the
behaviour the Rust implements. `tests/common::scrub_stack()` re-establishes that
condition before every FFI call, and `tests/phase_a_stack_ub.rs` asserts the
invariant still holds. See the "Ground-truth behaviours that no translation can
reproduce" section of `ERRORS.md` for the full story, including the two harness
bugs (per-call `dlsym`, lazy PLT binding) that had to be fixed to make rows
96..108 and 114..123 deterministic.
