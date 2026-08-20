# CONFIGS.md — Configuration surface table (Phase B)

## Mechanical derivation of the axes

There are no `#ifdef`s and no runtime global options in `c_src/src/lib.c`
(`grep -c '#ifdef\|#if \|static.*=' c_src/src/lib.c` -> 0). Every axis below is a
**function parameter** or an **input shape** that the C source visibly branches
on. Axes, with the C line that branches on them:

| axis | values the C distinguishes | branch site |
|------|---------------------------|-------------|
| `typeA` / `typeB` | `C2_TYPE_CIRCLE`(0) / `C2_TYPE_AABB`(1) / `C2_TYPE_CAPSULE`(2) | `c2MakeProxy` switch L109 |
| proxy `count` implied by type | 1 (circle) / 4 (aabb) / 2 (capsule) | L113,L119,L125 |
| proxy `radius` implied by type | `c->r` / `0` / `c->r` | L112,L118,L124 |
| `ax_ptr` / `bx_ptr` | NULL (-> identity) / non-NULL | L363,L367 |
| transform content | identity / pure translation / pure rotation / rotation+translation / non-unit `c2r` | `c2Mulxv` L177, `c2MulrvT` L354 |
| `use_radius` | 0 / nonzero | L477 |
| `cache` | NULL / count==0 (cold) / count 1,2,3 (warm) | L378,L379 |
| `outA`/`outB`/`iterations` | NULL / non-NULL | L505,L507,L509 |
| simplex `count` | 1 / 2 / 3 / other (`default:`) | `c2D` L278, `c2L` L343, `c2Witness` L308, `c2GJKSimplexMetric` L156, main switch L426 |
| `c22` Voronoi region | vertex-A / vertex-B / edge | L186,L190,L195 |
| `c23` Voronoi region | vtx-A / vtx-B / vtx-C / edge-AB / edge-BC / edge-CA / interior (7 arms) | L217-L256 |
| geometry relation | far apart / near / touching / overlapping / one inside other | drives `hit`, L436 + L480 |
| `c2Support` `count` | 1 / 2 / 4 / 8 (and the tie rule) | L296,L298 |
| `gjk` `reverse` | 0 / nonzero | L525 |
| float value class | normal / zero / negative-zero / subnormal / huge / NaN / Inf | all comparisons |

`[x]` = differential test passes across all randomised inputs for that row
(fixed-seed xorshift PRNG, iteration counts noted per row).

## Level 0 — pure vector/scalar helpers (no pointers)

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `c2V` | 200k random `(x,y)` bit patterns incl. NaN/Inf/subnormal/-0 | `row01_c2V` | [x] |
| 2 | `c2Sub`, `c2Add` | 200k random vector pairs, all value classes | `row02_add_sub` | [x] |
| 3 | `c2Mulvs`, `c2Div` | 200k random `(vec, scalar)`; scalar incl. 0, -0, Inf, NaN | `row03_mulvs_div` | [x] |
| 4 | `c2Dot` | 200k random pairs; incl. cancelling terms and Inf*0 | `row04_dot` | [x] |
| 5 | `c2Det2` | 200k random pairs; incl. `area == 0` collinear cases | `row05_det2` | [x] |
| 6 | `c2Len`, `c2Norm` | 200k random vectors; incl. zero vector, huge (overflow to Inf), subnormal | `row06_len_norm` | [x] |
| 7 | `c2Neg`, `c2Skew`, `c2CCW90` | 200k random vectors; sign of zero matters | `row07_neg_skew_ccw` | [x] |
| 8 | `c2Maxv`, `c2Minv` | 200k random pairs; equal, NaN, +0 vs -0 | `row08_maxv_minv` | [x] |
| 9 | `c2Clampv` | 200k random `(a, lo, hi)`; lo<hi, lo==hi, lo>hi, NaN bounds | `row09_clampv` | [x] |
| 10 | `c2RotIdentity`, `c2xIdentity` | no inputs — constant, compared field-by-field | `row10_identities` | [x] |
| 11 | `c2Mulrv`, `c2MulrvT` | 200k random `(c2r, c2v)`; unit rotations, non-unit, zero, NaN | `row11_mulrv` | [x] |
| 12 | `c2Mulrv`/`c2MulrvT` round trip | `c2MulrvT(r, c2Mulrv(r, v))` for unit `r` (both libs) | `row12_mulrv_roundtrip` | [x] |
| 13 | `c2Mulxv` | 200k random `(c2x, c2v)`: identity / translation-only / rotation-only / both | `row13_mulxv` | [x] |

## Level 1 — pointer / buffer helpers

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 14 | `c2BBVerts` | 50k random AABBs, out-buffer of 8 poisoned verts (checks only 4 written, winding order) | `row14_bbverts` | [x] |
| 15 | `c2MakeProxy` | `type=CIRCLE`, poisoned `c2Proxy`, 20k random circles (radius incl. 0/neg/NaN) | `row15_16_17_makeproxy_valid_types` | [x] |
| 16 | `c2MakeProxy` | `type=AABB`, poisoned proxy, 20k random AABBs (normal + inverted + degenerate) | `row15_16_17_makeproxy_valid_types` | [x] |
| 17 | `c2MakeProxy` | `type=CAPSULE`, poisoned proxy, 20k random capsules | `row15_16_17_makeproxy_valid_types` | [x] |
| 18 | `c2Support` | `count=1`, 50k random dirs | `row18_21_support_counts` | [x] |
| 19 | `c2Support` | `count=2` (capsule shape), 50k random dirs | `row18_21_support_counts` | [x] |
| 20 | `c2Support` | `count=4` (AABB shape), 50k random dirs incl. axis-aligned ties | `row18_21_support_counts` | [x] |
| 21 | `c2Support` | `count=8` (full proxy), 50k random vert sets and dirs | `row18_21_support_counts` | [x] |
| 22 | `c2Support` | duplicated verts forcing `dot == dmax` ties, 20k random | `row22_support_ties` | [x] |

## Level 2 — simplex primitives (the low-level entry points, driven directly)

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 23 | `c2GJKSimplexMetric` | `count=1`, 20k random simplices | `row23_25_simplex_metric` | [x] |
| 24 | `c2GJKSimplexMetric` | `count=2`, 20k random simplices | `row23_25_simplex_metric` | [x] |
| 25 | `c2GJKSimplexMetric` | `count=3`, 20k random simplices (incl. collinear -> 0 area) | `row23_25_simplex_metric` | [x] |
| 26 | `c22` | 100k random 2-simplices, full `c2Simplex` compared byte-for-byte (all 3 arms hit) | `row26_c22_random` | [x] |
| 27 | `c22` | structured inputs forcing each arm: `v<=0`, `u<=0`, edge | `row27_c22_forced_arms` | [x] |
| 28 | `c23` | 200k random 3-simplices, full struct compared (all 7 arms hit, counted) | `row28_c23_random` | [x] |
| 29 | `c23` | structured inputs forcing each of the 7 arms individually | `row29_c23_forced_arms` | [x] |
| 30 | `c2D` | `count=1,2,3,other`; 100k random; det>0 / det<0 / det==0 sub-cases | `row30_c2D` | [x] |
| 31 | `c2L` | `count=1,2,other`; 100k random; `div` normal / 0 / NaN / huge | `row31_c2L` | [x] |
| 32 | `c2Witness` | `count=1`, 50k random simplices | `row32_34_witness` | [x] |
| 33 | `c2Witness` | `count=2`, 50k random; `div` normal / tiny / 0 | `row32_34_witness` | [x] |
| 34 | `c2Witness` | `count=3`, 50k random; `div` normal / tiny / 0 | `row32_34_witness` | [x] |
| 35 | `c2Witness` | pipeline: random simplex -> `c22`/`c23` -> `c2Witness` (composed, not per-wrapper) | `row35_simplex_pipeline` | [x] |

## Level 3 — `c2GJK`, the full cross-product of options

Shape-type pairs (rows 36-44) each run with the full option matrix below.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 36 | `c2GJK` | `CIRCLE` vs `CIRCLE` | `row36_circle_circle` | [x] |
| 37 | `c2GJK` | `CIRCLE` vs `AABB` | `row37_circle_aabb` | [x] |
| 38 | `c2GJK` | `CIRCLE` vs `CAPSULE` | `row38_circle_capsule` | [x] |
| 39 | `c2GJK` | `AABB` vs `CIRCLE` | `row39_aabb_circle` | [x] |
| 40 | `c2GJK` | `AABB` vs `AABB` | `row40_aabb_aabb` | [x] |
| 41 | `c2GJK` | `AABB` vs `CAPSULE` | `row41_aabb_capsule` | [x] |
| 42 | `c2GJK` | `CAPSULE` vs `CIRCLE` | `row42_capsule_circle` | [x] |
| 43 | `c2GJK` | `CAPSULE` vs `AABB` | `row43_capsule_aabb` | [x] |
| 44 | `c2GJK` | `CAPSULE` vs `CAPSULE` | `row44_capsule_capsule` | [x] |
| 45 | `c2GJK` | option: `ax_ptr=NULL, bx_ptr=NULL` (identity substitution), all 9 type pairs | `row45_46_null_vs_explicit_identity` | [x] |
| 46 | `c2GJK` | option: explicit identity `c2x` for both (must equal row 45 exactly) | `row45_46_null_vs_explicit_identity` | [x] |
| 47 | `c2GJK` | option: pure translation on A, identity on B, all 9 type pairs | `row47_50_transform_modes` | [x] |
| 48 | `c2GJK` | option: pure rotation (unit `c2r` from `sincosf`) on both | `row47_50_transform_modes` | [x] |
| 49 | `c2GJK` | option: rotation + translation on both, all 9 type pairs | `row47_50_transform_modes` | [x] |
| 50 | `c2GJK` | option: non-unit / denormalised `c2r` (c,s random, not on unit circle) | `row47_50_transform_modes` | [x] |
| 51 | `c2GJK` | option: `use_radius = 1` (radius correction active) | `row51_52_use_radius` | [x] |
| 52 | `c2GJK` | option: `use_radius = 0` (raw simplex distance) | `row51_52_use_radius` | [x] |
| 53 | `c2GJK` | option: `cache = NULL` | `row53_54_60_cache_none_cold_iters` | [x] |
| 54 | `c2GJK` | option: `cache` cold (`count=0`), then cache contents compared field-by-field | `row53_54_60_cache_none_cold_iters` | [x] |
| 55 | `c2GJK` | option: `cache` warm — call twice with the SAME cache, both results compared | `row55_56_cache_warm_reuse` | [x] |
| 56 | `c2GJK` | option: `cache` warm — call twice with shapes MOVED between calls (real warm-start use) | `row55_56_cache_warm_reuse` | [x] |
| 57 | `c2GJK` | option: `cache` warm, `count=1` hand-built, in-range indices | `row57_59_cache_handbuilt_counts` | [x] |
| 58 | `c2GJK` | option: `cache` warm, `count=2` hand-built | `row57_59_cache_handbuilt_counts` | [x] |
| 59 | `c2GJK` | option: `cache` warm, `count=3` hand-built | `row57_59_cache_handbuilt_counts` | [x] |
| 60 | `c2GJK` | option: `iterations` non-NULL — iteration count compared exactly | `row53_54_60_cache_none_cold_iters` | [x] |
| 61 | `c2GJK` | shape: far apart (dist >> r) — separated path | `row61_67_geometry_relations` | [x] |
| 62 | `c2GJK` | shape: nearly touching (dist ~ rA+rB, straddles the L480 test) | `row61_67_geometry_relations` | [x] |
| 63 | `c2GJK` | shape: exactly touching | `row61_67_geometry_relations` | [x] |
| 64 | `c2GJK` | shape: overlapping -> `hit=1`, `s.count==3` | `row61_67_geometry_relations` | [x] |
| 65 | `c2GJK` | shape: one fully inside the other | `row65_containment` | [x] |
| 66 | `c2GJK` | shape: coincident centres (worst-case degeneracy) | `row61_67_geometry_relations` | [x] |
| 67 | `c2GJK` | shape: axis-aligned / grid-snapped coords (maximal support ties) | `row67_grid_snapped` | [x] |
| 68 | `c2GJK` | shape: huge coordinates (1e30) — overflow to Inf inside `c2Dot` | `row68_70_extreme_magnitudes` | [x] |
| 69 | `c2GJK` | shape: tiny/subnormal coordinates (1e-40) | `row68_70_extreme_magnitudes` | [x] |
| 70 | `c2GJK` | shape: radius 0 on both (rA+rB == 0 -> L480 falls to midpoint) | `row68_70_extreme_magnitudes` | [x] |
| 71 | `c2GJK` | full random sweep: all 9 type pairs x {use_radius 0,1} x {cache NULL,cold,warm} x {4 transform modes}, 2000 random geometries each | `row71_full_cross_product` | [x] |

## Level 4 — the public header entry point

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 72 | `gjk` | `reverse = 0` (AABB vs capsule), 100k random geometries | `row72_forward` | [x] |
| 73 | `gjk` | `reverse = 1` (capsule vs AABB), 100k random geometries | `row73_reverse` | [x] |
| 74 | `gjk` | `reverse` truthy non-1 values (`2`, `-1`, `0x7f`) | `row74_reverse_all_byte_values` | [x] |
| 75 | `gjk` | grid-snapped integer-ish coords (ties + exact touching) | `row75_grid_snapped` | [x] |
| 76 | `gjk` | overlapping AABB/capsule (hit path) | `row76_overlapping` | [x] |
| 77 | `gjk` | separated AABB/capsule (radius-correction path) | `row77_separated` | [x] |
| 78 | `gjk` | capsule radius 0 / huge / negative | `row78_radius_classes` | [x] |
| 79 | `gjk` | degenerate AABB (zero extent) and degenerate capsule (a==b) | `row79_degenerate_shapes` | [x] |

## Level 5 — NaN-payload configuration axis (added after mutation testing)

`ADDSS`/`MULSS` return the DESTINATION operand's NaN in preference to the
source's, so for the commutative `a+b` / `a*b` in the C the compiler's register
choice decides which payload survives. This is a real input axis (a caller can
pass any bit pattern) and `src/lib.rs` models it explicitly with
`addp(dst,src)` / `mulp(dst,src)`. It is invisible unless two operands are NaNs
with DIFFERENT payloads, so it needs its own rows.

Mutation testing showed the *dense* version (every input NaN) MASKS the axis:
one NaN wins early and hides the rest. The *sparse* version (NaN in exactly one
or two slots, everything else finite) is the one with real discriminating power.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 80 | `c2Dot`, `c2Det2` | dense: all 34 pool values x 3 slots | `nan_dot` / `nan_det2` | [x] |
| 81 | `c2Add`,`c2Sub`,`c2Mulvs`,`c2Div` | dense NaN pool cross-product | `nan_add_sub_mul_div` | [x] |
| 82 | `c2Mulrv`,`c2MulrvT`,`c2Mulxv` | dense NaN pool in rotation + vector | `nan_rotations` | [x] |
| 83 | `c2Len`,`c2Norm` | `sqrtf` NaN-payload preservation | `nan_len_norm` | [x] |
| 84 | `c2Maxv`,`c2Minv`,`c2Clampv` | NaN makes the ternary asymmetric | `nan_minmax_clamp` | [x] |
| 85 | `c2Witness`,`c2L` | dense NaN in `div` + `u` + `sA`/`sB` | `nan_witness_and_l` | [x] |
| 86 | `c22`,`c23`,`c2D`,`c2GJKSimplexMetric` | dense NaN vertices | `nan_simplex_reduction` | [x] |
| 87 | `c2GJK` | dense NaN shape data, all 9 type pairs x use_radius | `nan_gjk_end_to_end` | [x] |
| 88 | `gjk` | dense NaN in each of the nine floats | `nan_gjk_wrapper` | [x] |
| 89 | `c2Dot`,`c2Det2` | SPARSE: NaN in 1 or 2 of 4 slots, rest finite, every pair | `sparse_nan_dot_det2` | [x] |
| 90 | `c2Add`,`c2Sub`,`c2Maxv`,`c2Minv` | SPARSE over all 4-slot pairs | `sparse_nan_add_sub_minmax` | [x] |
| 91 | `c2Mulvs`,`c2Div`,`c2Len`,`c2Norm` | SPARSE over all 3-slot pairs | `sparse_nan_mulvs_div_len_norm` | [x] |
| 92 | `c2Clampv` | SPARSE over all 6-slot pairs | `sparse_nan_clampv` | [x] |
| 93 | `c2Mulrv`,`c2MulrvT`,`c2Mulxv` | SPARSE over all 6-slot pairs | `sparse_nan_rotations` | [x] |
| 94 | `c2Witness` | SPARSE over all 22-slot pairs, count 1/2/3 | `sparse_nan_witness` | [x] |
| 95 | `c2L` | SPARSE over all 22-slot pairs, count 0..4 | `sparse_nan_c2l` | [x] |
| 96 | `c22`,`c23`,`c2D`,`c2GJKSimplexMetric` | SPARSE over all 22-slot pairs | `sparse_nan_c22_c23_d_metric` | [x] |
| 97 | `c2GJK` | SPARSE over all 20-slot pairs x 3 type pairs x use_radius x transforms | `sparse_nan_gjk` | [x] |
| 98 | `gjk` | SPARSE over all 9-slot pairs x `reverse` | `sparse_nan_gjk_wrapper` | [x] |

## Level 6 — iteration-count reachability

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 99 | `c2GJK` | 3.6M calls over 5 magnitude scales x 9 type pairs, iteration histogram | `iteration_cap_search` | [x] |
| 100 | `c2GJK` | warm caches (count 1/2/3, random in-range indices) to lengthen the loop | `iteration_cap_via_warm_cache` | [x] |
| 101 | `c2GJK` | exhaustive small-integer lattice (21 600 calls) | `iteration_cap_exhaustive_lattice` | [x] |
| 102 | `c2GJK` | asserts every reported iteration count lies in `0..=20` | `zz_report_max_iterations` | [x] |

**Measured result:** the highest iteration count ANY input produces is **5**
(a proxy has at most 4 verts, so the simplex always resolves quickly). Confirmed
by bisection: replacing the C's `iter < 20` with `iter < 5` makes tests fail,
while `iter < 6` .. `iter < 19` are indistinguishable. The literal `20` is
therefore unreachable — see the mutation-testing notes in `SYMBOLS.md`.
