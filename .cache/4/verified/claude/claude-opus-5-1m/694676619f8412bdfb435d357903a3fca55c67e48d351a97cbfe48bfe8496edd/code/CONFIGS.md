# CONFIGS.md — configuration surface table (Phase A, gate for Phase B)

Derived mechanically from `c_src/src/lib.c`. This is the *valid-input* mirror of
`ERRORS.md`: one row per combination of runtime options × input shape that the C
code actually treats differently.

## The axes the C branches on

Enumerated from the `if` / `switch` / ternary sites in the source (there are no
`#ifdef`s — `grep -cE '^\s*#\s*(if|ifdef|ifndef)' c_src/src/lib.c` == 0, so
there are **no compile-time axes** and exactly one build configuration).

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| `C2_TYPE typeA` | `CIRCLE` / `AABB` / `CAPSULE` | `c2MakeProxy:113`, `c2Collided:576` |
| `C2_TYPE typeB` | `CIRCLE` / `AABB` / `CAPSULE` | `c2Collided:578,590,602` |
| proxy vertex count implied by type | 1 (circle) / 2 (capsule) / 4 (aabb) | `c2MakeProxy:117,123,129` |
| proxy radius implied by type | `c->r` / `0` / `c->r` | `c2MakeProxy:116,122,128` |
| `ax_ptr` | `NULL` (⇒ identity) / non-NULL | `c2GJK:367` |
| `bx_ptr` | `NULL` (⇒ identity) / non-NULL | `c2GJK:371` |
| transform content | identity / pure translation / pure rotation / rotation+translation / non-normalised `c2r` | `c2Mulxv`, `c2MulrvT` |
| `use_radius` | `0` / non-zero | `c2GJK:481` |
| `cache` | `NULL` / non-NULL & `count==0` / non-NULL & primed by a previous call / hand-crafted | `c2GJK:382,383,499` |
| `outA` / `outB` / `iterations` | `NULL` / non-NULL (independently, 8 combinations) | `c2GJK:509,511,513` |
| `s->count` (all simplex helpers) | `0`, `1`, `2`, `3`, `4`, negative | `c2GJKSimplexMetric:160`, `c22`, `c23`, `c2D:282`, `c2L:347`, `c2Witness:312` |
| `s->div` | `0`, `1`, positive, negative, `±inf`, `NaN` | `c2Witness:311`, `c2L:346` |
| `count` argument of `c2Support` | `1`, `2`, `4`, `8`, `0`, negative | `c2Support:300` |
| AABB shape | properly ordered / inverted (`min > max`) / degenerate point / zero-thickness slab | `c2Clampv`, `c2BBVerts`, `c2AABBtoAABB` |
| capsule shape | `a != b` / degenerate `a == b` / `r == 0` / `r > 0` | `c2CircletoCapsule:559,563` |
| circle shape | `r == 0` / `r > 0` | `c2CircletoCircle:543`, `c2CircletoAABB:551` |
| relative placement | deep overlap / containment / tangent (`dist == r`) / near-miss / far apart / coincident | every `<` test |
| float class | normal / `±0` / denormal / `±inf` / `NaN` / `FLT_MAX` / `FLT_EPSILON` | all arithmetic |

## Rows

Every row is exercised with **many randomized inputs** (fixed seed
`Rng::new(<row seed>)`, reproducible) unless the row is a pure constant.
Tests live in `tests/phase_b_valid.rs`; test names are given in the last column.

### Group 1 — leaf value helpers (lowest level, called directly)

| #  | entry point(s) | configuration (options set + input shape) | [x] | test |
|----|----------------|-------------------------------------------|-----|------|
| 01 | `c2V` | 20k random `(x,y)`: normals, `±0`, denormals, `±inf`, `NaN` (all payloads), `FLT_MAX` | [x] | `b01_c2V` |
| 02 | `c2Mulvs` | 20k random vector × random scalar, incl. `0*inf`, `NaN`, `±0` sign propagation | [x] | `b02_c2Mulvs` |
| 03 | `c2Maxv` / `c2Minv` | 20k random pairs; includes `NaN` on either/both sides (ternary picks `b` when the compare is false) and `+0 vs -0` ties | [x] | `b03_c2Maxv_c2Minv` |
| 04 | `c2Clampv` | 20k random `(a, lo, hi)`; `lo > hi` (inverted range), `lo == hi`, `NaN` in any of the three | [x] | `b04_c2Clampv` |
| 05 | `c2Sub` / `c2Add` | 20k random pairs; `inf - inf`, `±0 + ±0`, `NaN` payload propagation | [x] | `b05_c2Sub_c2Add` |
| 06 | `c2Dot` | 20k random pairs; cancellation (`a·a` with mixed signs), `inf*0`, `NaN` | [x] | `b06_c2Dot` |
| 07 | `c2Det2` | 20k random pairs; collinear (`det == ±0`), `inf`, `NaN` | [x] | `b07_c2Det2` |
| 08 | `c2Len` | 20k random vectors incl. `(0,0)`, `(inf,·)`, `NaN`, `FLT_MAX` (overflow to `inf`), denormals (underflow) | [x] | `b08_c2Len` |
| 09 | `c2Neg` / `c2Skew` / `c2CCW90` | 20k random vectors; `±0` sign flip must be bit-exact | [x] | `b09_c2Neg_Skew_CCW90` |
| 10 | `c2Div` | 20k random `(vector, scalar)`; scalar `0`, `-0`, `±inf`, `NaN`, denormal | [x] | `b10_c2Div` |
| 11 | `c2Norm` | 20k random vectors; `(0,0)` → `NaN`, huge/denormal magnitudes | [x] | `b11_c2Norm` |
| 12 | `c2RotIdentity` / `c2xIdentity` | no inputs — constant result, bit-exact | [x] | `b12_identities` |
| 13 | `c2Mulrv` / `c2MulrvT` | 20k random `(c2r, c2v)`: identity rot, true `cos/sin` rot, non-normalised rot, integer-grid rot, `NaN`/`inf` components | [x] | `b13_c2Mulrv_c2MulrvT` |
| 14 | `c2Mulxv` | 20k random `(c2x, c2v)` — all four transform kinds × vector classes | [x] | `b14_c2Mulxv` |

### Group 2 — proxy construction

| #  | entry point(s) | configuration | [x] | test |
|----|----------------|---------------|-----|------|
| 15 | `c2BBVerts` | 2k random AABBs: ordered, inverted, degenerate point, zero-thickness, `NaN`/`inf` corners. Full 4-vertex output diffed. | [x] | `b15_c2BBVerts` |
| 16 | `c2MakeProxy` | `type = CIRCLE`, 2k random circles (incl. `r = 0`, `r = inf`, `r = NaN`); the whole `c2Proxy` (radius, count, all 8 verts) diffed from a pre-poisoned buffer | [x] | `b16_makeproxy_circle` |
| 17 | `c2MakeProxy` | `type = AABB`, 2k random boxes (ordered / inverted / degenerate) | [x] | `b17_makeproxy_aabb` |
| 18 | `c2MakeProxy` | `type = CAPSULE`, 2k random capsules (incl. `a == b`, `r = 0`) | [x] | `b18_makeproxy_capsule` |

### Group 3 — simplex machinery (lowest-level GJK internals, called directly)

| #  | entry point(s) | configuration | [x] | test |
|----|----------------|---------------|-----|------|
| 19 | `c2GJKSimplexMetric` | `count = 1`, random fully-populated simplex | [x] | `b19_simplexmetric_by_count` |
| 20 | `c2GJKSimplexMetric` | `count = 2` (⇒ `c2Len` path) | [x] | `b19_simplexmetric_by_count` |
| 21 | `c2GJKSimplexMetric` | `count = 3` (⇒ `c2Det2` path), incl. collinear/degenerate triangles | [x] | `b19_simplexmetric_by_count` |
| 22 | `c22` | 20k random 2-simplices from the **integer grid** (so `u == 0`, `v == 0`, `u == v` actually occur); whole `c2Simplex` diffed after the call | [x] | `b22_c22_grid` |
| 23 | `c22` | 20k random 2-simplices with continuous/extreme coordinates (`inf`, `NaN`, `FLT_MAX`) | [x] | `b23_c22_wide` |
| 24 | `c23` | 20k random 3-simplices from the integer grid — hits all 7 branches incl. `area == 0` | [x] | `b24_c23_grid` |
| 25 | `c23` | 20k random 3-simplices, continuous/extreme coordinates | [x] | `b25_c23_wide` |
| 26 | `c2D` | `count = 1` / `2` (both `det > 0` and `det <= 0` sub-branches) / `3`, random simplices | [x] | `b26_c2D_by_count` |
| 27 | `c2L` | `count = 1` / `2`, `div` ∈ {`0`, `1`, random, negative} | [x] | `b27_c2L_by_count_and_div` |
| 28 | `c2Support` | `count = 1` (circle proxy), random `d` | [x] | `b28_support_by_count` |
| 29 | `c2Support` | `count = 2` (capsule proxy) | [x] | `b28_support_by_count` |
| 30 | `c2Support` | `count = 4` (AABB proxy) | [x] | `b28_support_by_count` |
| 31 | `c2Support` | `count = 8` (full `verts[8]`), with ties and `NaN` dots | [x] | `b28_support_by_count` |
| 32 | `c2Witness` | `count = 1`, random `div` | [x] | `b32_witness_by_count` |
| 33 | `c2Witness` | `count = 2`, `div` ∈ {`0`, `1`, random, negative, `inf`, `NaN`} | [x] | `b32_witness_by_count` |
| 34 | `c2Witness` | `count = 3`, same `div` sweep | [x] | `b32_witness_by_count` |

### Group 4 — `c2GJK`, the low-level composed pipeline

The cross product actually distinguished by the code:
`typeA(3) × typeB(3) × ax(NULL, id, rot+trans) × bx(NULL, id, rot+trans) ×
use_radius(0,1) × cache(NULL, empty, primed) × outsel(8)`.
Rows below are that cross-product pruned to the combinations the code
distinguishes; each row is driven with randomized shapes/placements.

| #  | entry point(s) | configuration | [x] | test |
|----|----------------|---------------|-----|------|
| 35 | `c2GJK` | all 9 `typeA × typeB` pairs, `ax=bx=NULL`, `use_radius=1`, `cache=NULL`, all outs requested; 9 × 3000 random shape pairs (far / near / overlapping / coincident) | [x] | `b35_gjk_all_type_pairs_no_xform` |
| 36 | `c2GJK` | all 9 pairs, `ax=bx=NULL`, `use_radius=0` | [x] | `b36_gjk_all_type_pairs_no_radius` |
| 37 | `c2GJK` | all 9 pairs, `ax` non-NULL (identity content), `bx=NULL` | [x] | `b37_gjk_ax_only` |
| 38 | `c2GJK` | all 9 pairs, `ax=NULL`, `bx` non-NULL (identity content) | [x] | `b38_gjk_bx_only` |
| 39 | `c2GJK` | all 9 pairs, both transforms non-NULL, **pure translation** | [x] | `b39_gjk_translation` |
| 40 | `c2GJK` | all 9 pairs, both transforms non-NULL, **pure rotation** (`cos/sin`) | [x] | `b40_gjk_rotation` |
| 41 | `c2GJK` | all 9 pairs, both transforms non-NULL, **rotation + translation** | [x] | `b41_gjk_rot_trans` |
| 42 | `c2GJK` | all 9 pairs, non-normalised `c2r` (scaling/shearing transform) — exercises `c2MulrvT` with a non-orthonormal basis | [x] | `b42_gjk_non_normalised_rot` |
| 43 | `c2GJK` | `cache` non-NULL, **zeroed** (`count == 0`): cold start + write-back; full cache struct diffed | [x] | `b43_gjk_cache_cold` |
| 44 | `c2GJK` | `cache` **primed by a previous `c2GJK` call**, then re-queried with the *same* shapes (the intended warm-start path) | [x] | `b44_gjk_cache_warm_same` |
| 45 | `c2GJK` | cache primed, then re-queried with **moved** shapes (temporal coherence, the real consumer pattern: 8 sequential frames) | [x] | `b45_gjk_cache_warm_moved_sequence` |
| 46 | `c2GJK` | cache primed, then re-queried with a **different shape type whose proxy has at least as many vertices** (circle→aabb, capsule→aabb, …), so every cached index stays inside `verts[0..count)`. The reverse direction (larger→smaller proxy) reads uninitialised stack memory in the C and is documented as UB in `ERRORS.md` row 33. | [x] | `b46_gjk_cache_warm_type_switch` |
| 47 | `c2GJK` | hand-crafted cache: `count` ∈ {1,2,3} × `metric` ∈ {0, ±1e9, `NaN`, `inf`} × `div` ∈ {0,1,random} × indices swept over the **whole valid range** `0..proxy.count` for both proxies | [x] | `b47_gjk_cache_handcrafted` |
| 48 | `c2GJK` | all 8 `outA/outB/iterations` NULL-ness combinations × `cache` NULL/non-NULL (16 combos) | [x] | `b48_gjk_out_param_matrix` |
| 49 | `c2GJK` | degenerate/duplicate shapes: A and B **identical** (⇒ `c2Dot(d,d) < eps²` break, `c2Norm((0,0))`) | [x] | `b49_gjk_identical_shapes` |
| 50 | `c2GJK` | zero-radius circle vs zero-radius circle, zero-size AABB vs zero-size AABB, `a==b` capsules | [x] | `b50_gjk_zero_sized_shapes` |
| 51 | `c2GJK` | deliberately tangent placements (`dist` exactly `rA+rB`) to straddle the `dist > rA+rB` test | [x] | `b51_gjk_tangent_placements` |
| 52 | `c2GJK` | large-magnitude coordinates (`1e30`, `FLT_MAX/4`) → overflow inside `c2Dot`/`c2Len` | [x] | `b52_gjk_huge_coords` |
| 53 | `c2GJK` | denormal / `FLT_EPSILON`-scale coordinates → underflow, `d1 > d0` regress branch | [x] | `b53_gjk_tiny_coords` |

### Group 5 — the boolean convenience wrappers

| #  | entry point(s) | configuration | [x] | test |
|----|----------------|---------------|-----|------|
| 54 | `c2AABBtoAABB` | 50k random box pairs: ordered/inverted/degenerate, separated on each of the 4 axes, exact-touch, `NaN`/`inf` | [x] | `b54_aabb_to_aabb` |
| 55 | `c2CircletoCircle` | 50k random pairs incl. `r=0`, exact tangency, coincident centres, `NaN` | [x] | `b55_circle_to_circle` |
| 56 | `c2CircletoAABB` | 50k random pairs; centre inside / outside / exactly on an edge / on a corner; inverted boxes | [x] | `b56_circle_to_aabb` |
| 57 | `c2CircletoCapsule` | 50k random pairs covering all three `da/db` branches + degenerate `a==b` capsule | [x] | `b57_circle_to_capsule` |
| 58 | `c2AABBtoCapsule` | 20k random pairs (drives the whole `c2GJK` pipeline with `use_radius=1`) | [x] | `b58_aabb_to_capsule` |
| 59 | `c2CapsuletoCapsule` | 20k random pairs, incl. parallel, crossing, collinear, degenerate | [x] | `b59_capsule_to_capsule` |
| 60 | `c2Collided` | all 9 valid `typeA × typeB` pairs × 5k random shape pairs each (dispatch table incl. the argument-swapping cases at 592/604/606) | [x] | `b60_collided_all_pairs` |

### Group 6 — the public header entry point

| #  | entry point(s) | configuration | [x] | test |
|----|----------------|---------------|-----|------|
| 61 | `aabb` (`include/lib.h`) | 200k random `(min_x,min_y,max_x,max_y)` in the geometrically interesting `[-150,150]` window (all 8 result bit-mask values reachable) | [x] | `b61_aabb_entry_random` |
| 62 | `aabb` | integer-grid inputs around the three hard-coded shapes (`-70±20`, `-40..-15`, capsule `(-40,40)-(-20,100)` r=10) so every boundary is hit exactly | [x] | `b62_aabb_entry_grid` |
| 63 | `aabb` | extreme inputs: `±0`, `±inf`, `NaN`, `FLT_MAX`, `FLT_MIN`, denormals, inverted boxes (full 6^4 sweep of the special pool) | [x] | `b63_aabb_entry_specials` |

## Result

All **63 rows** are covered by 56 test functions in `tests/phase_b_valid.rs`
(plus `tests/smoke.rs`), every one driven with randomized inputs from a
fixed-seed generator, and **all pass bit-for-bit** (`f32::to_bits` on every float
in every output, so `+0.0` vs `-0.0` and NaN sign/payload differences are
caught):

```
$ cargo build && cargo test --test phase_b_valid
test result: ok. 56 passed; 0 failed; 0 ignored
```

Coverage diagnostics printed by the tests (`-- --nocapture`):

```
b58: 0 case(s) skipped (C hit the 20-iteration cap => UB)
b59: 0 case(s) skipped (C hit the 20-iteration cap => UB)
b60: 0 case(s) skipped (C hit the 20-iteration cap => UB)
b61: result bit-masks observed: [true, true, true, true, true, true, true, true], 0 skipped
b62: 8000 grid cases, 0 skipped
b63: 104976 special cases compared, 0 skipped (UB cap)
```

`b61` reaching all **8** possible return values of `aabb()` means the public
entry point's whole output space is exercised, and the "0 skipped" lines mean the
one undefined-behaviour path the composed pipeline could have taken (the
20-iteration cap, `ERRORS.md` row 35) is never actually reached.

Test-strength evidence for these rows is in `MUTATION_NOTES.md`: 28 of 31
injected divergences are detected by this suite; the 3 survivors are proven to be
semantically equivalent mutants.
