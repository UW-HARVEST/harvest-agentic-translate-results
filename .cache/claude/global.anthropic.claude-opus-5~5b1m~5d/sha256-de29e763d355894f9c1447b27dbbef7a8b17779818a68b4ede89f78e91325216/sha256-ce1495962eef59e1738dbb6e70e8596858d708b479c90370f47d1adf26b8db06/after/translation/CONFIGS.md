# CONFIGS.md — configuration surface table (valid inputs)

Axes the C code actually branches on, derived from `c_src/src/lib.c`:

* **A1 — shape type** (`C2_TYPE`): `CIRCLE` (proxy: radius=r, count=1),
  `AABB` (radius=0, count=4), `CAPSULE` (radius=r, count=2).
  → drives `c2MakeProxy`, `c2Support`'s `count`, and the `use_radius` term.
* **A2 — `c2Collided` type pair**: the 3×3 dispatch matrix (9 combinations,
  including the two argument-swapping ones `AABB/CIRCLE` and `CAPSULE/*`).
* **A3 — `c2GJK` transforms**: `ax_ptr`/`bx_ptr` `NULL` (identity) vs. a real
  `c2x` (translation + rotation); 2×2 = 4 combinations.
* **A4 — `c2GJK` `use_radius`**: `0` vs `1`.
* **A5 — `c2GJK` cache**: `NULL`, non-NULL with `count == 0` (cold), non-NULL
  with `count ∈ {1,2,3}` (warm / stale), and the *round-trip* case where the
  same cache is fed back into a second call.
* **A6 — `c2GJK` out-params**: `outA`/`outB`/`iterations` NULL vs non-NULL.
* **A7 — simplex `count`** for `c22`/`c23`/`c2D`/`c2L`/`c2Witness`/
  `c2GJKSimplexMetric`: `1`, `2`, `3` and the Voronoi sub-region that `c22`/`c23`
  select (`c22`: 3 regions; `c23`: 7 regions).
* **A8 — geometric relation**: separated / touching / overlapping / identical /
  fully-contained.
* **A9 — degenerate shapes**: zero radius, negative radius, zero-length capsule
  (`a == b`), zero-area AABB (`min == max`), inverted AABB (`min > max`).
* **A10 — float value class**: normal, ±0, denormal, huge (`1e30`, `FLT_MAX`),
  `±inf`, `NaN` (incl. signalling payloads / sign bits).
* **A11 — vertex count for `c2Support`**: 1, 2, 4, 8 (the `c2Proxy` capacity).

Rows are the pruned cross-product of the axes the code distinguishes. Every row
is driven with **many** seeded-random inputs (seed fixed in the test file), not a
single hand-picked value, and compared bit-for-bit (`f32::to_bits`) between the
C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `c2V` | random finite floats + ±0/denormal/huge | [x] |
| C2 | `c2V` | `±inf` / `NaN` (all sign & payload variants) | [x] |
| C3 | `c2Mulvs` | random vector × random scalar, finite | [x] |
| C4 | `c2Mulvs` | scalar `0`, `inf`, `NaN`; vector with `inf`/`NaN` (`0*inf` cases) | [x] |
| C5 | `c2Add` / `c2Sub` | random finite pairs; overflow to `±inf`; cancellation to ±0 | [x] |
| C6 | `c2Add` / `c2Sub` | `inf - inf`, `NaN` operands (NaN-payload propagation) | [x] |
| C7 | `c2Maxv` / `c2Minv` | random pairs; equal components; `+0` vs `-0`; `NaN` in a/b (comparison is false → picks `b`) | [x] |
| C8 | `c2Clampv` | in-range / below-lo / above-hi / lo>hi (inverted) / NaN operands | [x] |
| C9 | `c2Dot` | random finite; huge values that overflow; `0*inf`; NaN | [x] |
| C10 | `c2Det2` | random finite; collinear (result ±0); huge/NaN | [x] |
| C11 | `c2Len` | random finite; zero vector; huge (overflow to inf); NaN | [x] |
| C12 | `c2Div` / `c2Norm` | random finite; unit-length input; zero vector; huge; NaN | [x] |
| C13 | `c2Neg` / `c2Skew` / `c2CCW90` | random finite; ±0 (sign of zero); `inf`; `NaN` (sign-bit flip) | [x] |
| C14 | `c2RotIdentity` / `c2xIdentity` | no inputs — constant result | [x] |
| C15 | `c2Mulrv` / `c2MulrvT` | identity rot; random unit rot (cos/sin θ); non-normalised rot; zero rot; NaN/inf rot | [x] |
| C16 | `c2Mulxv` | identity `c2x`; pure translation; pure rotation; both; NaN/inf | [x] |
| C17 | `c2BBVerts` | random AABB (min<max); zero-area (min==max); inverted (min>max); huge/NaN | [x] |
| C18 | `c2MakeProxy` | `type = CIRCLE`, random circle → radius/count/verts[0] | [x] |
| C19 | `c2MakeProxy` | `type = AABB`, random / inverted / zero-area AABB → count 4 + 4 corners | [x] |
| C20 | `c2MakeProxy` | `type = CAPSULE`, random / zero-length capsule → count 2 | [x] |
| C21 | `c2Support` | `count = 1` (single vertex, loop never runs) | [x] |
| C22 | `c2Support` | `count = 2` (capsule proxy), random direction incl. ties | [x] |
| C23 | `c2Support` | `count = 4` (AABB proxy), random direction, axis-aligned dirs (ties → first max wins) | [x] |
| C24 | `c2Support` | `count = 8` (max proxy capacity), random verts & dir | [x] |
| C25 | `c2Support` | direction `(0,0)` (all dots 0 → strict `>` keeps index 0); NaN direction | [x] |
| C26 | `c2GJKSimplexMetric` | `count = 1` → 0 | [x] |
| C27 | `c2GJKSimplexMetric` | `count = 2` → `c2Len(b.p-a.p)`, random points | [x] |
| C28 | `c2GJKSimplexMetric` | `count = 3` → `c2Det2`, random points incl. degenerate/collinear | [x] |
| C29 | `c22` | region A (`v <= 0`) | [x] |
| C30 | `c22` | region B (`u <= 0`) → `s->a = s->b` copy of the whole `c2sv` | [x] |
| C31 | `c22` | interior edge region (`u > 0 && v > 0`) → count 2, `div = u+v` | [x] |
| C32 | `c22` | fully random `a.p`/`b.p` (plus all other `c2sv` fields random, to prove field copying) | [x] |
| C33 | `c23` | vertex-A region (`vAB<=0 && uCA<=0`) | [x] |
| C34 | `c23` | vertex-B region (`uAB<=0 && vBC<=0`) → `a = b` | [x] |
| C35 | `c23` | vertex-C region (`uBC<=0 && vCA<=0`) → `a = c` | [x] |
| C36 | `c23` | edge-AB region (`wABC<=0`) | [x] |
| C37 | `c23` | edge-BC region (`uABC<=0`) → `a=b; b=c` | [x] |
| C38 | `c23` | edge-CA region (`vABC<=0`) → `b=a; a=c` | [x] |
| C39 | `c23` | interior region → count 3, `div = uABC+vABC+wABC` | [x] |
| C40 | `c23` | fully random triangles (all six barycentric branches hit statistically), incl. degenerate `area == 0` and winding reversal | [x] |
| C41 | `c2D` | `count = 1` → `-a.p` | [x] |
| C42 | `c2D` | `count = 2`, `c2Det2(ab, -a.p) > 0` → `c2Skew` | [x] |
| C43 | `c2D` | `count = 2`, det `<= 0` → `c2CCW90` | [x] |
| C44 | `c2L` | `count = 1`; `count = 2` with random `u`/`div` (incl. `div` not equal to `u+v`) | [x] |
| C45 | `c2Witness` | `count = 1` (direct copy of `sA`/`sB`) | [x] |
| C46 | `c2Witness` | `count = 2` (2-term barycentric blend), random `u`/`div` | [x] |
| C47 | `c2Witness` | `count = 3` (3-term blend, nested `c2Add` order matters) | [x] |
| C48 | `c2GJK` | CIRCLE vs CIRCLE, no transforms, `use_radius=1`, no cache, all out-params | [x] |
| C49 | `c2GJK` | CIRCLE vs CIRCLE, `use_radius=0` | [x] |
| C50 | `c2GJK` | CIRCLE vs AABB, both `use_radius` values | [x] |
| C51 | `c2GJK` | CIRCLE vs CAPSULE, both `use_radius` values | [x] |
| C52 | `c2GJK` | AABB vs AABB, both `use_radius` values | [x] |
| C53 | `c2GJK` | AABB vs CAPSULE, both `use_radius` values | [x] |
| C54 | `c2GJK` | CAPSULE vs CAPSULE, both `use_radius` values | [x] |
| C55 | `c2GJK` | CAPSULE vs CIRCLE / CAPSULE vs AABB / AABB vs CIRCLE (reversed argument order) | [x] |
| C56 | `c2GJK` | any type pair × `ax_ptr = &x` (translation only), `bx_ptr = NULL` | [x] |
| C57 | `c2GJK` | any type pair × `ax_ptr = NULL`, `bx_ptr = &x` (rotation only) | [x] |
| C58 | `c2GJK` | any type pair × both transforms non-NULL (translation + rotation, random θ) | [x] |
| C59 | `c2GJK` | non-normalised `c2r` (`c`,`s` random, not on the unit circle) | [x] |
| C60 | `c2GJK` | cache non-NULL, `count = 0` (cold) — checks the written-back cache too | [x] |
| C61 | `c2GJK` | cache non-NULL, warm `count = 1` with random valid `iA`/`iB` for the proxy | [x] |
| C62 | `c2GJK` | cache non-NULL, warm `count = 2` | [x] |
| C63 | `c2GJK` | cache non-NULL, warm `count = 3` (can trigger the immediate `hit`) | [x] |
| C64 | `c2GJK` | cache round-trip: two consecutive calls sharing one cache, then a third with moved shapes | [x] |
| C65 | `c2GJK` | out-params selectively NULL: `outA` only, `outB` only, `iterations` only, none | [x] |
| C66 | `c2GJK` | overlapping shapes (deep penetration → `hit == 1`) | [x] |
| C67 | `c2GJK` | exactly touching shapes (`dist == rA+rB`) → midpoint branch | [x] |
| C68 | `c2GJK` | identical shapes (coincident, `dist <= FLT_EPSILON`) | [x] |
| C69 | `c2GJK` | far-separated shapes (large coordinate magnitudes, `1e6`..`1e30`) | [x] |
| C70 | `c2GJK` | zero-radius circle/capsule, zero-length capsule, zero-area AABB | [x] |
| C71 | `c2GJK` | negative radii (`r < 0`) | [x] |
| C72 | `c2GJK` | inputs containing `±inf` / `NaN` coordinates (iteration count + returned dist) | [x] |
| C73 | `c2AABBtoAABB` | random pairs: separated on x / on y / overlapping / touching / nested / inverted; ±0 edges | [x] |
| C74 | `c2CircletoCircle` | separated / touching / overlapping / concentric; zero & negative radius | [x] |
| C75 | `c2CircletoAABB` | centre inside / outside on each of the 8 Voronoi regions of the box / on an edge; inverted box; zero radius | [x] |
| C76 | `c2CircletoCapsule` | `da < 0` (before A), `db < 0` (mid-segment), `db >= 0` (past B); zero-length capsule; zero/negative radius | [x] |
| C77 | `c2AABBtoCapsule` | random shapes covering hit & miss (goes through the full GJK pipeline) | [x] |
| C78 | `c2CapsuletoCapsule` | parallel / crossing / collinear / identical / far apart | [x] |
| C79 | `c2Collided` | all 9 valid `(typeA, typeB)` combinations with random shapes | [x] |
| C80 | `c2Collided` | the two argument-swapping rows (`AABB×CIRCLE`, `CAPSULE×CIRCLE`, `CAPSULE×AABB`) with asymmetric shapes, to catch a swapped translation | [x] |
| C81 | `capsule` | random finite `(min_x,min_y,max_x,max_y,r)` — all 8 possible return values | [x] |
| C82 | `capsule` | boundary/degenerate args: all-zero, `r = 0`, negative `r`, `a == b`, huge, `±inf`, `NaN` | [x] |
| C83 | `capsule` | inputs specifically placed to hit each of the 3 result bits alone and in combination | [x] |

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table**, so
`--no-default-features` and the default build are byte-identical; the whole
table therefore has exactly one configuration to verify. `features.sh`
enumerates and re-runs it.

---

## Phase B status — which test covers which row

Test binaries: `tests/phase_b_math.rs` (C1..C47) and `tests/phase_b_gjk.rs`
(C48..C83). All rows verified in the dev **and** release profile, against the
C `.so` built by the specified `cmake ..` recipe. Every row is driven with
seeded-random inputs (`Rng::new(<fixed seed>)`, splitmix64) and compared with
`f32::to_bits()` equality, so `+0`/`-0` and NaN payloads must match too.

| rows | test function | randomized inputs per row |
|------|---------------|---------------------------|
| C1, C2 | `c1_c2_c2v` | 4 000 ordinary + 4 000 "wild" (incl. `±inf`, quiet/negative NaN, `FLT_MAX`, denormals, fully random bit patterns) |
| C3, C4 | `c3_c4_mulvs` | 8 000 + a 5x4 special-value grid |
| C5, C6 | `c5_c6_add_sub` | 12 000 + overflow / `inf-inf` cases |
| C7 | `c7_maxv_minv` | 16 000 + `±0` orderings |
| C8 | `c8_clampv` | 16 000 + inverted bounds + on-boundary |
| C9, C10 | `c9_c10_dot_det2` | 16 000 + collinear + overflow + `0*inf` |
| C11 | `c11_len` | 8 000 + 7 fixed edge vectors |
| C12 | `c12_div_norm` | 16 000 + a 5x7 fixed grid |
| C13 | `c13_neg_skew_ccw90` | 24 000 |
| C14 | `c14_identities` | constant, checked 8x |
| C15, C16 | `c15_c16_rotations` | 12 000 + 800 identity/translation-only/rotation-only |
| C17 | `c17_bbverts` | 4 000 (mixed normal / inverted / zero-area / wild) |
| C18, C19, C20 | `c18_c19_c20_makeproxy` | 4 000 per shape kind, into a sentinel-filled `c2Proxy` (all 8 vertex slots compared) |
| C21..C25 | `c21_to_c25_support` | 4 000 per `count ∈ {1,2,4,8}`, x5 axis-aligned tie directions, plus total-tie and NaN-direction cases, plus 4 000 wild per count |
| C26, C27, C28 | `c26_c27_c28_simplex_metric` | 4 000 per `count ∈ {1,2,3}` + 4 000 collinear |
| C29..C32 | `c29_to_c32_c22` | 16 000 fully random + 4 000 parameter-swept (covers all 3 Voronoi regions) + 20 000 degenerate |
| C33..C40 | `c33_to_c40_c23` | 24 000 fully random + 8 000 shifted/rewound triangles (all 7 regions) + 28 000 degenerate |
| C41, C42, C43 | `c41_to_c43_c2d` | 8 000 per `count ∈ {1,2}` + 8 000 forced det>0 / det==0 + 4 000 wild |
| C44 | `c44_c2l` | 8 000 per count + 4 000 consistent-`div` + 4 000 wild |
| C45, C46, C47 | `c45_to_c47_witness` | 8 000 per `count ∈ {1,2,3}` + 8 000 barycentric-consistent + 12 000 wild |
| C48..C55 | `c48_to_c55_type_pairs` | 9 type pairs x 2 `use_radius` x 1 500 = 27 000 full GJK calls; return value, both witness points and `*iterations` all compared |
| C56..C59 | `c56_to_c59_transforms` | 9 pairs x 4 transform modes x 2 `use_radius` x 750 = 54 000 |
| C60..C63 | `c60_to_c63_caches` | 9 pairs x `cache->count ∈ {0,1,2,3}` x 2 `use_radius` x 750 = 54 000, with the written-back cache compared field by field |
| C64 | `c64_cache_roundtrip` | 9 pairs x 1 500 x 3 chained calls sharing one cache |
| C65 | `c65_null_outparams` | 9 pairs x 8 NULL masks x 375 |
| C66, C67, C68 | `c66_to_c71_relations` | 1 500 x 4 overlap/containment sets x 2 + 1 500 exactly-touching x 2 |
| C69 | `c66_to_c71_relations` | 1 500 x 9 pairs x 2, magnitudes `1e3 … 1e30` |
| C70 | `c66_to_c71_relations` | 1 500 x 25 degenerate shape pairs x 2 |
| C71 | `c66_to_c71_relations` | 1 500 x 9 negative-radius pairs x 2 |
| C72 | `c72_gjk_specials` | 9 pairs x 1 500 x (2 `use_radius` + 1 wild-transform) with `±inf`/NaN coordinates |
| C73 | `c73_aabb_to_aabb` | 36 000 random (x3 orderings) + 1 500 x 7 touching offsets + 1 500 wild |
| C74 | `c74_circle_to_circle` | 24 000 random (x2) + 1 500 x 5 separation ratios + zero/negative/wild |
| C75 | `c75_circle_to_aabb` | 12 000 random + 1 500 x 49 grid positions x2 radii + inverted + wild |
| C76 | `c76_circle_to_capsule` | 12 000 random + 1 500 x 28 (t, perpendicular-offset) positions + zero-length + negative + wild |
| C77 | `c77_aabb_to_capsule` | 9 000 random + 1 500 x 7 arranged + degenerate + wild |
| C78 | `c78_capsule_to_capsule` | 9 000 random (x2) + 1 500 x (5 parallel + crossing + collinear + degenerate + wild) |
| C79, C80 | `c79_c80_collided_matrix` | 9 pairs x 4 500 random + 6 000 x 6 deliberately asymmetric swap cases + 9 pairs x 1 500 wild |
| C81, C82, C83 | `c81_to_c83_capsule_entry` | 30 000 random (>=6 of the 8 possible return values observed, asserted) + 8 targeted bit patterns + 12^3 x 2 special-value tuples |

Extra coverage beyond the table: `tests/e31_search.rs` runs 400 000 further
randomized `c2GJK` configurations mixing shape kinds, transforms, warm caches
and `use_radius`, comparing dist / both witness points / `*iterations` / the
written-back cache each time.

## Note on the C compiler's optimisation level

The reference C `.so` is the one produced by the recipe in the task
(`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`, i.e. no `CMAKE_BUILD_TYPE`,
so `-O0`). Against that build the Rust `.so` is bit-identical on **all** of the
above, including NaN payloads and signed zeros.

Re-running the suite against a `-DCMAKE_BUILD_TYPE=Release` (`-O3`) C build
(supported via the `C2_C_SO` environment variable in `tests/common/mod.rs`)
leaves everything green except `c72_gjk_specials`, where a *quiet-NaN sign bit*
differs (`0x7fc00000` vs `0xffc00000`). That is GCC swapping the operands of a
commutative `mulss`/`addss` at `-O3`, which changes which operand's NaN is
propagated — a property of the C compiler's instruction selection, not of the
translation. Every non-NaN value still matches bit-for-bit at `-O3`.
