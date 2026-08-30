# CONFIGS.md — Phase B configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Axes derived mechanically from the
branches `c_src/src/lib.c` actually takes.

## Axes enumerated from the source

**A1 — shape type (`C2_TYPE`)**, the `switch` in `c2MakeProxy` / `c2Collided`:
`C2_TYPE_CIRCLE` (proxy: radius = `r`, count = 1), `C2_TYPE_AABB` (radius = 0,
count = 4 via `c2BBVerts`), `C2_TYPE_CAPSULE` (radius = `r`, count = 2).
→ `c2GJK`/`c2Collided` cross-product = 3 × 3 = 9.

**A2 — `use_radius`** (`c2GJK` arg 9): `0` (raw Minkowski distance) vs `!= 0`
(radius shrink + the two sub-branches at lib.c:485 and lib.c:493).

**A3 — `ax_ptr` / `bx_ptr`** (`c2x` transforms): `NULL` → `c2xIdentity()`, vs
non-`NULL` with (a) identity, (b) pure translation, (c) pure rotation,
(d) rotation + translation. Rotation feeds `c2Mulrv`/`c2MulrvT` and changes the
support-point search direction; the `c2r` is **not** required to be normalised,
so unnormalised rotors are a distinct shape.

**A4 — `cache`**: `NULL`; non-`NULL` cold (`count == 0` → `cache_was_good`
false); non-`NULL` warm (result of a previous `c2GJK` call, `count` ∈ {1,2,3}
→ the `cache_was_read` seeding path at lib.c:386-407).

**A5 — output pointers**: `outA`/`outB`/`iterations` all `NULL` vs all
non-`NULL` vs a mix (each has its own `if` at lib.c:510-515).

**A6 — proximity / input shape**: deep overlap (GJK `hit` path, `s.count == 3`),
shallow overlap, exactly touching, near-touching within `FLT_EPSILON`, clearly
disjoint, very distant (large coordinates), coincident shapes.

**A7 — degenerate but valid shapes**: zero-radius circle, zero-area AABB
(`min == max`), zero-length capsule (`a == b`), axis-aligned vs oblique capsule,
tiny (denormal-scale) and huge (`1e30`) magnitudes.

**A8 — simplex vertex count** for the low-level `c22`, `c23`, `c2D`, `c2L`,
`c2Witness`, `c2GJKSimplexMetric` entry points: `count` ∈ {1, 2, 3} × each of
the 3 branches of `c22` and the 7 branches of `c23`.

**A9 — `c2Support` vertex count**: 1, 2, 4, 8 verts × direction sign/quadrant,
incl. exact ties (`dot > dmax` is strict, so ties keep the *lower* index).

**A10 — entry-point level**: pure math leaves (`c2V` … `c2MulrvT`), proxy
builders (`c2BBVerts`, `c2MakeProxy`), simplex ops (`c22`, `c23`, `c2D`, `c2L`,
`c2Witness`, `c2GJKSimplexMetric`, `c2Support`), the core solver (`c2GJK`), the
boolean predicates (`c2*to*`), the dispatcher (`c2Collided`), and the one-shot
wrapper (`reverse_collide`). All ten levels are driven directly.

Every row is exercised with **many** randomized inputs from a fixed-seed
xorshift PRNG (`rand_f32` in `tests/common/mod.rs`), and asserted **bitwise**
(`to_bits()`), not with an epsilon.

## Table

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|--------------------------------------------|------|---|
| 1 | `c2V`, `c2Neg`, `c2Skew`, `c2CCW90` | 1-arg vector leaves × random finite/subnormal/huge floats | `cfg_vec_unary` | [x] |
| 2 | `c2Add`, `c2Sub`, `c2Dot`, `c2Det2` | 2-vector leaves × random pairs incl. cancellation (a≈b) | `cfg_vec_binary` | [x] |
| 3 | `c2Mulvs`, `c2Div` | vector × scalar, scalar random incl. ±0, ±1, huge, tiny | `cfg_vec_scalar` | [x] |
| 4 | `c2Maxv`, `c2Minv`, `c2Clampv` | random pairs/triples, incl. equal components and `lo > hi` (inverted clamp box) | `cfg_minmax_clamp` | [x] |
| 5 | `c2Len`, `c2Norm` | random vectors: unit-scale, `1e-30` (underflow of `x*x`), `1e20` (overflow of `x*x`) | `cfg_len_norm` | [x] |
| 6 | `c2RotIdentity`, `c2xIdentity` | no-arg constructors (constant fold check) | `cfg_identities` | [x] |
| 7 | `c2Mulrv`, `c2MulrvT`, `c2Mulxv` | rotor × vector: identity, normalised angle, **unnormalised** rotor, `(0,0)` rotor; transform with/without translation | `cfg_rotations` | [x] |
| 8 | `c2BBVerts` | valid AABB (min<max), zero-area (min==max), random bounds; all 4 output verts compared | `cfg_bbverts` | [x] |
| 9 | `c2MakeProxy` | type=CIRCLE × random circle → radius/count/verts[0] | `cfg_makeproxy_circle` | [x] |
| 10 | `c2MakeProxy` | type=AABB × random AABB → radius=0, count=4, 4 verts | `cfg_makeproxy_aabb` | [x] |
| 11 | `c2MakeProxy` | type=CAPSULE × random capsule → radius, count=2, verts[0..1] | `cfg_makeproxy_capsule` | [x] |
| 12 | `c2Support` | count=1 (single vert) × random directions | `cfg_support_count1` | [x] |
| 13 | `c2Support` | count=2 (capsule proxy) × random directions incl. perpendicular ties | `cfg_support_count2` | [x] |
| 14 | `c2Support` | count=4 (AABB proxy) × random directions, all 4 quadrants + axis-aligned ties | `cfg_support_count4` | [x] |
| 15 | `c2Support` | count=8 (full proxy) × random verts and directions | `cfg_support_count8` | [x] |
| 16 | `c2GJKSimplexMetric` | count=1 → 0 | `cfg_metric_count1` | [x] |
| 17 | `c2GJKSimplexMetric` | count=2 → `c2Len(b.p-a.p)`, random points | `cfg_metric_count2` | [x] |
| 18 | `c2GJKSimplexMetric` | count=3 → `c2Det2`, random points incl. collinear (area≈0) | `cfg_metric_count3` | [x] |
| 19 | `c22` | branch `v <= 0` (origin beyond A) — whole simplex mutation compared | `cfg_c22_branches` | [x] |
| 20 | `c22` | branch `u <= 0` (origin beyond B, `s->a = s->b` copy) | `cfg_c22_branches` | [x] |
| 21 | `c22` | branch `else` (origin inside the segment, `div = u+v`, count=2) | `cfg_c22_branches` | [x] |
| 22 | `c22` | fully random simplexes (all branches by chance) — 4 verts + div + count compared bitwise | `cfg_c22_random` | [x] |
| 23 | `c23` | branch 1 `vAB<=0 && uCA<=0` (vertex A region) | `cfg_c23_random` | [x] |
| 24 | `c23` | branch 2 `uAB<=0 && vBC<=0` (vertex B region, `a=b`) | `cfg_c23_random` | [x] |
| 25 | `c23` | branch 3 `uBC<=0 && vCA<=0` (vertex C region, `a=c`) | `cfg_c23_random` | [x] |
| 26 | `c23` | branch 4 edge AB (`wABC<=0`) | `cfg_c23_random` | [x] |
| 27 | `c23` | branch 5 edge BC (`uABC<=0`, `a=b; b=c`) | `cfg_c23_random` | [x] |
| 28 | `c23` | branch 6 edge CA (`vABC<=0`, `b=a; a=c`) | `cfg_c23_random` | [x] |
| 29 | `c23` | branch 7 interior (count=3, `div = u+v+w`) | `cfg_c23_random` | [x] |
| 30 | `c23` | negative-orientation (CW) triangles → `area < 0` flips every sign | `cfg_c23_random` | [x] |
| 31 | `c2D` | count=1 → `-a.p`; count=2 both `c2Det2 > 0` (skew) and `<= 0` (ccw90) branches | `cfg_c2d` | [x] |
| 32 | `c2L` | count=1 and count=2 × random `div` and `u` weights | `cfg_c2l` | [x] |
| 33 | `c2Witness` | count=1 / 2 / 3 × random `sA`/`sB`/`u`/`div` | `cfg_witness` | [x] |
| 34 | `c2GJK` | CIRCLE↔CIRCLE, `use_radius=0`, all ptrs NULL, random separated | `cfg_gjk_matrix` | [x] |
| 35 | `c2GJK` | CIRCLE↔CIRCLE, `use_radius=1`, all ptrs NULL, random separated + overlapping | `cfg_gjk_matrix` | [x] |
| 36 | `c2GJK` | CIRCLE↔AABB, `use_radius` ∈ {0,1} | `cfg_gjk_matrix` | [x] |
| 37 | `c2GJK` | CIRCLE↔CAPSULE, `use_radius` ∈ {0,1} | `cfg_gjk_matrix` | [x] |
| 38 | `c2GJK` | AABB↔CIRCLE, `use_radius` ∈ {0,1} | `cfg_gjk_matrix` | [x] |
| 39 | `c2GJK` | AABB↔AABB, `use_radius` ∈ {0,1} | `cfg_gjk_matrix` | [x] |
| 40 | `c2GJK` | AABB↔CAPSULE, `use_radius` ∈ {0,1} | `cfg_gjk_matrix` | [x] |
| 41 | `c2GJK` | CAPSULE↔CIRCLE, `use_radius` ∈ {0,1} | `cfg_gjk_matrix` | [x] |
| 42 | `c2GJK` | CAPSULE↔AABB, `use_radius` ∈ {0,1} | `cfg_gjk_matrix` | [x] |
| 43 | `c2GJK` | CAPSULE↔CAPSULE, `use_radius` ∈ {0,1} | `cfg_gjk_matrix` | [x] |
| 44 | `c2GJK` | all 9 type pairs × non-NULL **identity** transforms (must equal the NULL path) | `cfg_gjk_transform_identity` | [x] |
| 45 | `c2GJK` | all 9 type pairs × non-NULL pure-**translation** transforms on A and B | `cfg_gjk_transform_translate` | [x] |
| 46 | `c2GJK` | all 9 type pairs × non-NULL pure-**rotation** transforms (normalised rotor) | `cfg_gjk_transform_rotate` | [x] |
| 47 | `c2GJK` | all 9 type pairs × **rotation + translation** on both A and B | `cfg_gjk_transform_full` | [x] |
| 48 | `c2GJK` | all 9 type pairs × **unnormalised** rotor (scaling transform) | `cfg_gjk_transform_unnormalised` | [x] |
| 49 | `c2GJK` | all 9 type pairs × cold cache (`count = 0`, zeroed struct); full cache struct compared after | `cfg_gjk_cache_cold` | [x] |
| 50 | `c2GJK` | all 9 type pairs × **warm** cache: call twice with the same cache, shapes unchanged | `cfg_gjk_cache_warm_same` | [x] |
| 51 | `c2GJK` | all 9 type pairs × warm cache reused after the shapes **moved** (metric mismatch path) | `cfg_gjk_cache_warm_moved` | [x] |
| 52 | `c2GJK` | warm cache carried across a long random walk (10 sequential calls, cache threaded through) | `cfg_gjk_cache_sequence` | [x] |
| 53 | `c2GJK` | `iterations` non-NULL: iteration count compared for all 9 type pairs | `cfg_gjk_iterations` | [x] |
| 54 | `c2GJK` | mixed output pointers: `outA` non-NULL / `outB` NULL, and vice versa | `cfg_gjk_mixed_outputs` | [x] |
| 55 | `c2GJK` | deep overlap → `hit` path (`s.count == 3`, returns `+0.0`) for every type pair | `cfg_gjk_deep_overlap` | [x] |
| 56 | `c2GJK` | exactly touching shapes (distance == sum of radii) | `cfg_gjk_touching` | [x] |
| 57 | `c2GJK` | coincident shapes (A == B) for every type pair | `cfg_gjk_coincident` | [x] |
| 58 | `c2GJK` | degenerate-but-valid shapes: r=0 circle, min==max AABB, a==b capsule (all 9 pairs) | `cfg_gjk_degenerate_shapes` | [x] |
| 59 | `c2GJK` | huge coordinates (`±1e18`) and tiny coordinates (`±1e-20`) | `cfg_gjk_extreme_scales` | [x] |
| 60 | `c2AABBtoAABB` | random overlapping / disjoint / edge-touching / nested boxes | `cfg_aabb_to_aabb` | [x] |
| 61 | `c2AABBtoCapsule` | random AABB × capsule: overlapping, disjoint, capsule crossing a corner, zero-length capsule | `cfg_aabb_to_capsule` | [x] |
| 62 | `c2CapsuletoCapsule` | random pairs: crossing, parallel, collinear, zero-length | `cfg_capsule_to_capsule` | [x] |
| 63 | `c2CircletoCircle` | random pairs incl. exact tangency and r=0 | `cfg_circle_to_circle` | [x] |
| 64 | `c2CircletoAABB` | circle centre inside / outside / on each edge / at each corner of the box | `cfg_circle_to_aabb` | [x] |
| 65 | `c2CircletoCapsule` | all three `da`/`db` branches: before A, between, past B; plus zero-length capsule | `cfg_circle_to_capsule` | [x] |
| 66 | `c2Collided` | all 9 valid `typeA` × `typeB` combinations × random shapes (verifies the swapped-argument dispatch at lib.c:593/605/607) | `cfg_collided_matrix` | [x] |
| 67 | `reverse_collide` | random `(x, y, r)` over the whole interesting box (`x,y ∈ [-160,160]`, `r ∈ [0,60]`) — hits all 8 bitmask values | `cfg_reverse_collide_random` | [x] |
| 68 | `reverse_collide` | grid sweep of the three fixed shapes' boundaries (circle at (-70,0) r20, AABB [-40,-40]..[-15,-15], capsule (-40,40)-(-20,100) r10) incl. exact-tangency values | `cfg_reverse_collide_grid` | [x] |
