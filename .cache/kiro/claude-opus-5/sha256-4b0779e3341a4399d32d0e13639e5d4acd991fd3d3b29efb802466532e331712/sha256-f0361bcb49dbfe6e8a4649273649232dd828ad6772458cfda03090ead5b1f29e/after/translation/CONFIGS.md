# CONFIGS.md — Phase A configuration surface table (valid inputs)

Axes the C code actually branches on, derived from `c_src/src/lib.c`:

* **shape type** `C2_TYPE` ∈ {`C2_TYPE_CIRCLE`=0, `C2_TYPE_AABB`=1,
  `C2_TYPE_CAPSULE`=2} — branched on in `c2MakeProxy` and twice in `c2Collided`
  (9 ordered pairs). Also fixes the proxy **vertex count** (1 / 4 / 2) and
  whether **radius** is nonzero (`c->r` / `0` / `c->r`).
* **`c2GJK` transform pointers** `ax_ptr`, `bx_ptr` ∈ {NULL → identity,
  non-NULL} × {identity rotation, non-identity rotation, pure translation}.
* **`c2GJK` `use_radius`** ∈ {0, 1} — selects whether the radius-shrink /
  midpoint-collapse post-pass runs at all.
* **`c2GJK` `cache`** ∈ {NULL, cold (`count==0`), warm (written back by a prior
  call and reused)} — selects the `cache_was_read` path vs. the cold seed.
* **`c2GJK` out params** `outA`, `outB`, `iterations` ∈ {NULL, non-NULL}.
* **simplex `count`** ∈ {1, 2, 3} for `c22` / `c23` / `c2D` / `c2L` /
  `c2Witness` / `c2GJKSimplexMetric`, crossed with the geometric sub-cases each
  function distinguishes (`c22`: 3 branches; `c23`: 7 branches).
* **geometric arrangement** ∈ {separated, touching, overlapping, containment,
  degenerate (zero-radius / zero-extent / `a==b` capsule)}.
* **`c2Support` count** ∈ {1, 2, 4} (the counts the three shape types produce).

One row per combination the C treats differently:

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V`, `c2Sub`, `c2Add`, `c2Neg`, `c2Skew`, `c2CCW90`, `c2Mulvs` | randomized finite `c2v`, incl. `±0`, denormals, large magnitudes | [x] |
| 2 | `c2Dot`, `c2Det2`, `c2Len` | randomized finite `c2v` pairs; also zero vectors and huge values that overflow the product | [x] |
| 3 | `c2Maxv`, `c2Minv` | randomized pairs; each component independently ordered both ways, plus equal components | [x] |
| 4 | `c2Clampv` | `a` below `lo`, inside, above `hi`, per component (9 sub-cases) + inverted `lo>hi` | [x] |
| 5 | `c2Div`, `c2Norm` | randomized nonzero divisor / nonzero vector; near-zero magnitudes | [x] |
| 6 | `c2RotIdentity`, `c2xIdentity` | no inputs — exact bit compare of returned structs | [x] |
| 7 | `c2Mulrv`, `c2MulrvT` | randomized `c2r` (both normalized `cos/sin` pairs and arbitrary `c`,`s`) × randomized `c2v` | [x] |
| 8 | `c2Mulxv` | randomized `c2x` = translation-only, rotation-only, and combined | [x] |
| 9 | `c2BBVerts` | randomized AABBs: normal, degenerate (`min==max`), inverted (`min>max`) — 4 output verts each | [x] |
| 10 | `c2MakeProxy` | `type = C2_TYPE_CIRCLE`, randomized `c2Circle` → `count==1`, `radius==r`, `verts[0]==p`; full 8-vert buffer compared | [x] |
| 11 | `c2MakeProxy` | `type = C2_TYPE_AABB`, randomized `c2AABB` → `count==4`, `radius==0`, 4 verts | [x] |
| 12 | `c2MakeProxy` | `type = C2_TYPE_CAPSULE`, randomized `c2Capsule` → `count==2`, `radius==r`, verts a/b | [x] |
| 13 | `c2Support` | `count==1` (circle proxy shape), randomized direction | [x] |
| 14 | `c2Support` | `count==2` (capsule proxy shape), randomized direction — ties resolved by strict `>` keeping the lower index | [x] |
| 15 | `c2Support` | `count==4` (AABB proxy shape), randomized direction, incl. axis-aligned directions that tie | [x] |
| 16 | `c2GJKSimplexMetric` | `count==1` → `0`; randomized simplex contents to prove contents are ignored | [x] |
| 17 | `c2GJKSimplexMetric` | `count==2` → `c2Len(b.p-a.p)`, randomized points | [x] |
| 18 | `c2GJKSimplexMetric` | `count==3` → `c2Det2(b.p-a.p, c.p-a.p)`, randomized points (both winding orders) | [x] |
| 19 | `c22` | `v <= 0` branch → collapse to vertex a, `count=1`, `div=1` | [x] |
| 20 | `c22` | `u <= 0` branch → `a = b`, `count=1`, `div=1` | [x] |
| 21 | `c22` | interior branch → `count=2`, `div=u+v`, barycentric `u`s | [x] |
| 22 | `c23` | `vAB<=0 && uCA<=0` → vertex A region | [x] |
| 23 | `c23` | `uAB<=0 && vBC<=0` → vertex B region (`a = b`) | [x] |
| 24 | `c23` | `uBC<=0 && vCA<=0` → vertex C region (`a = c`) | [x] |
| 25 | `c23` | `uAB>0 && vAB>0 && wABC<=0` → edge AB | [x] |
| 26 | `c23` | `uBC>0 && vBC>0 && uABC<=0` → edge BC (`a=b; b=c`) | [x] |
| 27 | `c23` | `uCA>0 && vCA>0 && vABC<=0` → edge CA (`b=a; a=c`) | [x] |
| 28 | `c23` | interior/else → `count=3`, barycentric `uABC/vABC/wABC` | [x] |
| 29 | `c23` | degenerate triangle: collinear points (`area==0`) and coincident points | [x] |
| 30 | `c2D` | `count==1` → `-a.p`, randomized | [x] |
| 31 | `c2D` | `count==2`, `c2Det2(ab, -a.p) > 0` → `c2Skew(ab)` | [x] |
| 32 | `c2D` | `count==2`, `c2Det2(ab, -a.p) <= 0` → `c2CCW90(ab)` | [x] |
| 33 | `c2L` | `count==1` → `a.p`; `count==2` → barycentric blend with randomized `div`/`u` | [x] |
| 34 | `c2Witness` | `count==1`, `count==2`, `count==3` with randomized `div`/`u`, both out ptrs non-null | [x] |
| 35 | `c2GJK` | (circle, circle), both transforms NULL, `use_radius=1`, no cache, all out ptrs | [x] |
| 36 | `c2GJK` | (circle, circle), `use_radius=0` | [x] |
| 37 | `c2GJK` | (circle, aabb), NULL transforms, `use_radius=1` and `0` | [x] |
| 38 | `c2GJK` | (circle, capsule), NULL transforms, `use_radius=1` and `0` | [x] |
| 39 | `c2GJK` | (aabb, circle), NULL transforms, `use_radius=1` and `0` | [x] |
| 40 | `c2GJK` | (aabb, aabb), NULL transforms, `use_radius=1` and `0` | [x] |
| 41 | `c2GJK` | (aabb, capsule), NULL transforms, `use_radius=1` and `0` | [x] |
| 42 | `c2GJK` | (capsule, circle), NULL transforms, `use_radius=1` and `0` | [x] |
| 43 | `c2GJK` | (capsule, aabb), NULL transforms, `use_radius=1` and `0` | [x] |
| 44 | `c2GJK` | (capsule, capsule), NULL transforms, `use_radius=1` and `0` | [x] |
| 45 | `c2GJK` | all 9 type pairs × non-NULL `ax_ptr` with **identity** transform (must equal the NULL case) | [x] |
| 46 | `c2GJK` | all 9 type pairs × non-NULL `ax_ptr`/`bx_ptr` with randomized **rotation + translation** | [x] |
| 47 | `c2GJK` | all 9 type pairs × `ax_ptr` non-NULL, `bx_ptr` NULL (mixed nullness) | [x] |
| 48 | `c2GJK` | all 9 type pairs × cold cache (`count=0`, garbage `metric`/`div`/indices) — compare returned distance **and** the written-back cache struct | [x] |
| 49 | `c2GJK` | all 9 type pairs × **warm cache reuse**: call twice with the same cache, compare both distances and the final cache bytes | [x] |
| 50 | `c2GJK` | all 9 type pairs × warm cache carried across a *moved* shape (cache indices still valid, geometry changed) | [x] |
| 51 | `c2GJK` | `outA=NULL`, `outB` non-NULL (and vice versa), `iterations=NULL` | [x] |
| 52 | `c2GJK` | overlapping shapes reaching the `hit` (count==3) path, all 9 type pairs | [x] |
| 53 | `c2GJK` | touching-exactly shapes (`dist` within `FLT_EPSILON`) → midpoint collapse branch | [x] |
| 54 | `c2GJK` | far-separated shapes (large coordinate magnitudes, ~1e6) | [x] |
| 55 | `c2GJK` | degenerate shapes: zero-radius circle, `min==max` AABB, `a==b` capsule, zero-radius capsule | [x] |
| 56 | `c2GJK` | iteration-cap stress: shapes chosen so `*iterations` is >1; compare `*iterations` exactly | [x] |
| 57 | `c2AABBtoAABB` | randomized AABB pairs: separated on x, on y, on both, overlapping, touching edges, nested, inverted | [x] |
| 58 | `c2AABBtoCapsule` | randomized AABB × capsule: separated, touching, overlapping, capsule inside AABB, degenerate capsule | [x] |
| 59 | `c2CapsuletoCapsule` | randomized capsule pairs: parallel, crossing, collinear, coincident, degenerate | [x] |
| 60 | `c2CircletoCircle` | randomized circle pairs: separated, tangent, overlapping, nested, zero radius | [x] |
| 61 | `c2CircletoAABB` | randomized circle × AABB: circle center inside, outside on a face, outside at a corner, exactly on the boundary, degenerate AABB | [x] |
| 62 | `c2CircletoCapsule` | randomized circle × capsule hitting all three `da/db` branches (`da<0`, `da>=0 && db<0`, `db>=0`) + degenerate `a==b` | [x] |
| 63 | `c2Collided` | all 9 valid `(typeA, typeB)` ordered pairs × randomized shapes — verifies the argument-swapping in the asymmetric arms | [x] |
| 64 | `reverse_collide` | randomized `(x, y, r)` over the region covering the fixed circle / AABB / capsule, so all 8 result bit-masks occur | [x] |
| 65 | `reverse_collide` | grid sweep over `x,y ∈ [-120,40]`, `r ∈ {0, 0.5, 5, 20, 100}` — exhaustive bit-exact comparison | [x] |
| 66 | `reverse_collide` | extreme inputs: `±inf`, `NaN`, `±FLT_MAX`, denormals, `-0.0` | [x] |

No binary/driver target exists (`c_src/CMakeLists.txt` builds only `SHARED`;
`translation/Cargo.toml` declares only `[lib] crate-type = ["cdylib"]`), so the
"compare stdout of C and Rust binaries" gate is not applicable.
