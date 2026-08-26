# CONFIGS.md — configuration surface of `c_src/src/lib.c`

## Build-time configuration

`Cargo.toml` has **no `[features]` section**, and `c_src/CMakeLists.txt` has no
`option()`/`add_definitions()`/`#ifdef`. The C source contains zero
preprocessor conditionals (`grep -c '^#if' c_src/src/lib.c` → 0).

Therefore the complete set of valid feature combinations is exactly one:

| # | feature combo | cargo invocation |
|---|---------------|------------------|
| 1 | *(empty — no default features, no optional features)* | `cargo test --no-default-features` (identical to `cargo test`) |

Both spellings are exercised in Phase D.

## Runtime configuration axes (derived from the C branches)

| axis | values the C actually distinguishes | C site |
|------|-------------------------------------|--------|
| `C2_TYPE typeA` / `typeB` | `CIRCLE(0)`, `AABB(1)`, `CAPSULE(2)` (+ out-of-range → `ERRORS.md`) | `switch` at L114, L577, L579, L591, L603 |
| proxy shape produced | `radius=r,count=1` / `radius=0,count=4` / `radius=r,count=2` | L115-133 |
| `ax_ptr` / `bx_ptr` | `NULL` (→ identity) vs. real `c2x` — and within "real": identity rotation (`c={1,0}`) vs. general rotation vs. pure translation | L368-375, `c2Mulxv`, `c2MulrvT` |
| `use_radius` | `0` (raw core distance) vs. non-zero (shrink by `rA+rB`) | L482 |
| `outA` / `outB` / `iterations` | `NULL` vs. non-`NULL` (8 combinations) | L510-515 |
| `cache` | `NULL`; non-`NULL` with `count==0` (cold); non-`NULL` warm with `count∈{1,2,3}`; warm-but-rejected by the metric guard; repeated calls that feed the cache back in | L383-408, L500-509 |
| simplex `count` reached | 1 (point), 2 (segment → `c22`), 3 (triangle → `c23`, `hit=1`) | L431-444 |
| `c22` branch | `v<=0` / `u<=0` / else | L191-205 |
| `c23` branch | 7 branches: vertex A / vertex B / vertex C / edge AB / edge BC / edge CA / interior | L222-261 |
| geometric relation | disjoint-far, disjoint-near, exactly touching, overlapping, fully contained, identical shapes | strict `<` at L544/L552/L573, L485 |
| degenerate shapes | zero-radius circle, zero-extent AABB, inverted AABB (`min>max`), zero-length capsule (`a==b`), zero-radius capsule | L520-523, L548, L556-570 |
| float value classes | normal, `0.0`, `-0.0`, denormal (`FLT_MIN/2`), huge (`1e30`, `FLT_MAX`), `±inf`, `NaN` | unchecked arithmetic throughout |
| `c2Support` shape | `count=1`, `2`, `4`, `8`; with/without ties | L298-308 |

Every row below is a combination the C treats differently, and each is driven
with **many randomized inputs from a fixed-seed PCG32** (not a single value).
`[x]` = differential test present *and* passing against the C `.so`
(verified under BOTH feature combinations and BOTH the `dev` and `release`
profiles by `./verify_all_features.sh`).

## Phase B rows

### Level 0 — scalar / vector primitives (`tests/phase_b_level0.rs`)

| #  | entry point(s) | configuration (options set + input shape) | differential test | [x] |
|----|----------------|-------------------------------------------|-------------------|-----|
| B01 | `c2V`, `c2Sub`, `c2Add`, `c2Neg`, `c2Skew`, `c2CCW90` | 4096 random `c2v` pairs over normals, plus the full boundary grid (`0,-0,FLT_MIN,FLT_MAX,±inf,NaN`) cross-product | `b01_v_sub_add_neg_skew_ccw90` | [x] |
| B02 | `c2Mulvs`, `c2Div` | random `c2v` × random scalar; scalar from boundary grid incl. `0`, `-0`, `±inf`, `NaN` | `b02_mulvs_div` | [x] |
| B03 | `c2Dot`, `c2Det2` | random `c2v` pairs; magnitudes spanning `1e-38 … 1e38` to force overflow/underflow of the products | `b03_dot_det2` | [x] |
| B04 | `c2Maxv`, `c2Minv`, `c2Clampv` | random `a`; `lo<=hi`, `lo==hi`, and inverted `lo>hi` boxes; `NaN` in each of the 6 slots | `b04_maxv_minv_clampv` | [x] |
| B05 | `c2Len`, `c2Norm` | random `c2v`; plus zero vector, denormal vector, `1e30` (overflow to `inf`), `NaN`, and negative-dot impossibilities | `b05_len_norm` | [x] |
| B06 | `c2RotIdentity`, `c2xIdentity` | no inputs — bit-exact struct return check | `b06_identities` | [x] |
| B07 | `c2Mulrv`, `c2MulrvT` | random `c2r` (both normalized `cos/sin` pairs and arbitrary unnormalized) × random `c2v` | `b07_mulrv_mulrvT` | [x] |
| B08 | `c2Mulxv` | random `c2x` (identity rot + translation, general rot, `NaN` rot) × random `c2v` | `b08_mulxv` | [x] |
| B09 | `c2Support` | `count ∈ {1,2,4,8}` × random verts × random direction; plus deliberate exact ties (duplicate verts) to pin the first-maximum rule | `b09_support` | [x] |
| B10 | `c2BBVerts` | random AABBs incl. inverted (`min>max`), zero-extent, `NaN` corners; all 4 output verts compared | `b10_bbverts` | [x] |

### Level 1 — proxy + simplex internals (`tests/phase_b_level1.rs`)

| #  | entry point(s) | configuration (options set + input shape) | differential test | [x] |
|----|----------------|-------------------------------------------|-------------------|-----|
| B11 | `c2MakeProxy` | `type=CIRCLE` × random circles (incl. `r=0`, `r<0`, `NaN`); whole 72-byte `c2Proxy` compared | `b11_makeproxy_circle` | [x] |
| B12 | `c2MakeProxy` | `type=AABB` × random AABBs (normal, inverted, zero-extent) | `b12_makeproxy_aabb` | [x] |
| B13 | `c2MakeProxy` | `type=CAPSULE` × random capsules (incl. `a==b`, `r=0`) | `b13_makeproxy_capsule` | [x] |
| B14 | `c2GJKSimplexMetric` | `count=1` (returns 0) × random simplex payload | `b14_b15_b16_simplex_metric` | [x] |
| B15 | `c2GJKSimplexMetric` | `count=2` (segment length) × random `a.p`,`b.p`, incl. `a.p==b.p` and huge coords | `b14_b15_b16_simplex_metric` | [x] |
| B16 | `c2GJKSimplexMetric` | `count=3` (signed area) × random triangle, both winding orders, degenerate collinear | `b14_b15_b16_simplex_metric` | [x] |
| B17 | `c22` | branch `v<=0` (origin behind A): whole 4×`c2sv`+div+count struct compared | `b17_b18_b19_b20_c22` | [x] |
| B18 | `c22` | branch `u<=0` (origin past B) — checks the `s->a = s->b` copy | `b17_b18_b19_b20_c22` | [x] |
| B19 | `c22` | branch else (origin projects inside AB) | `b17_b18_b19_b20_c22` | [x] |
| B20 | `c22` | fully random simplexes (branch chosen by the data), 4096 iterations | `b17_b18_b19_b20_c22` | [x] |
| B21 | `c23` | branch 1 `vAB<=0 && uCA<=0` (vertex A region) | `b21_to_b28_c23` | [x] |
| B22 | `c23` | branch 2 `uAB<=0 && vBC<=0` (vertex B region, `a=b` copy) | `b21_to_b28_c23` | [x] |
| B23 | `c23` | branch 3 `uBC<=0 && vCA<=0` (vertex C region, `a=c` copy) | `b21_to_b28_c23` | [x] |
| B24 | `c23` | branch 4 edge AB (`wABC<=0`) | `b21_to_b28_c23` | [x] |
| B25 | `c23` | branch 5 edge BC (`uABC<=0`, `a=b; b=c` shuffle) | `b21_to_b28_c23` | [x] |
| B26 | `c23` | branch 6 edge CA (`vABC<=0`, `b=a; a=c` shuffle) | `b21_to_b28_c23` | [x] |
| B27 | `c23` | branch 7 interior (origin inside the triangle → `count=3`) | `b21_to_b28_c23` | [x] |
| B28 | `c23` | fully random simplexes incl. degenerate/collinear/duplicated verts, 4096 iterations | `b21_to_b28_c23` | [x] |
| B29 | `c2D` | `count=1` | `b29_to_b32_cD` | [x] |
| B30 | `c2D` | `count=2`, `c2Det2(ab,-a)>0` (skew branch) | `b29_to_b32_cD` | [x] |
| B31 | `c2D` | `count=2`, `c2Det2(ab,-a)<=0` (CCW90 branch), incl. exactly `0` | `b29_to_b32_cD` | [x] |
| B32 | `c2D` | `count=3` | `b29_to_b32_cD` | [x] |
| B33 | `c2L` | `count=1` | `b33_b34_cL` | [x] |
| B34 | `c2L` | `count=2` × random `u`/`div`, incl. `div` huge/denormal | `b33_b34_cL` | [x] |
| B35 | `c2Witness` | `count=1` | `b35_b36_b37_witness` | [x] |
| B36 | `c2Witness` | `count=2` × random `sA/sB/u/div` | `b35_b36_b37_witness` | [x] |
| B37 | `c2Witness` | `count=3` × random `sA/sB/u/div` | `b35_b36_b37_witness` | [x] |

### Level 2 — `c2GJK` (`tests/phase_b_level2_gjk.rs`)

All rows compare the returned `float` bit-exactly **and** `*outA`, `*outB`,
`*iterations`, and the full 36-byte `c2GJKCache` write-back.

| #  | entry point(s) | configuration (options set + input shape) | differential test | [x] |
|----|----------------|-------------------------------------------|-------------------|-----|
| B38 | `c2GJK` | `typeA=CIRCLE,  typeB=CIRCLE`,  xforms NULL, `use_radius=0`, no cache | `b38_to_b46_all_type_pairs_plain` | [x] |
| B39 | `c2GJK` | `typeA=CIRCLE,  typeB=AABB`,    xforms NULL, `use_radius=0`, no cache | `b38_to_b46_all_type_pairs_plain` | [x] |
| B40 | `c2GJK` | `typeA=CIRCLE,  typeB=CAPSULE`, xforms NULL, `use_radius=0`, no cache | `b38_to_b46_all_type_pairs_plain` | [x] |
| B41 | `c2GJK` | `typeA=AABB,    typeB=CIRCLE`,  xforms NULL, `use_radius=0`, no cache | `b38_to_b46_all_type_pairs_plain` | [x] |
| B42 | `c2GJK` | `typeA=AABB,    typeB=AABB`,    xforms NULL, `use_radius=0`, no cache | `b38_to_b46_all_type_pairs_plain` | [x] |
| B43 | `c2GJK` | `typeA=AABB,    typeB=CAPSULE`, xforms NULL, `use_radius=0`, no cache | `b38_to_b46_all_type_pairs_plain` | [x] |
| B44 | `c2GJK` | `typeA=CAPSULE, typeB=CIRCLE`,  xforms NULL, `use_radius=0`, no cache | `b38_to_b46_all_type_pairs_plain` | [x] |
| B45 | `c2GJK` | `typeA=CAPSULE, typeB=AABB`,    xforms NULL, `use_radius=0`, no cache | `b38_to_b46_all_type_pairs_plain` | [x] |
| B46 | `c2GJK` | `typeA=CAPSULE, typeB=CAPSULE`, xforms NULL, `use_radius=0`, no cache | `b38_to_b46_all_type_pairs_plain` | [x] |
| B47 | `c2GJK` | all 9 type pairs, `use_radius=1`, xforms NULL, no cache (radius-shrink path L482-499) | `b47_use_radius` | [x] |
| B48 | `c2GJK` | all 9 type pairs, `ax_ptr` = general rotation+translation, `bx_ptr` = NULL | `b48_to_b52_transforms` | [x] |
| B49 | `c2GJK` | all 9 type pairs, `ax_ptr` = NULL, `bx_ptr` = general rotation+translation | `b48_to_b52_transforms` | [x] |
| B50 | `c2GJK` | all 9 type pairs, both xforms general (exercises `c2MulrvT` in the support call) | `b48_to_b52_transforms` | [x] |
| B51 | `c2GJK` | all 9 type pairs, both xforms **pure translation** (`r = {1,0}`) | `b48_to_b52_transforms` | [x] |
| B52 | `c2GJK` | all 9 type pairs, xforms with unnormalized/huge rotation `c,s` | `b48_to_b52_transforms` | [x] |
| B53 | `c2GJK` | all 9 type pairs × `use_radius∈{0,1}`, shapes forced **far apart** (disjoint, `dist>rA+rB`) | `b53_far_apart` | [x] |
| B54 | `c2GJK` | all 9 type pairs × `use_radius∈{0,1}`, shapes forced **deeply overlapping** (`hit=1`, `count==3`) | `b54_deep_overlap` | [x] |
| B55 | `c2GJK` | all 9 type pairs × `use_radius∈{0,1}`, shapes **exactly touching** (`dist == rA+rB`) | `b55_exactly_touching` | [x] |
| B56 | `c2GJK` | all 9 type pairs, **identical** A and B (`d≈0` epsilon break at L451) | `b56_identical_shapes` | [x] |
| B57 | `c2GJK` | cache non-NULL, `count=0` (cold start), single call; verifies cache write-back | `b57_cold_cache` | [x] |
| B58 | `c2GJK` | cache non-NULL, warm `count=1` with valid indices | `b58_b59_b60_warm_cache` | [x] |
| B59 | `c2GJK` | cache non-NULL, warm `count=2` with valid indices | `b58_b59_b60_warm_cache` | [x] |
| B60 | `c2GJK` | cache non-NULL, warm `count=3` with valid indices (AABB/capsule proxies) | `b58_b59_b60_warm_cache` | [x] |
| B61 | `c2GJK` | cache-feedback loop: 8 successive calls reusing the returned cache while the shapes are perturbed each step (the real consumer pattern) | `b61_cache_feedback_loop` | [x] |
| B62 | `c2GJK` | all 64 combinations of `{outA,outB,iterations}` × `{ax_ptr,bx_ptr,cache}` NULL / non-NULL | `b62_null_optional_pointer_matrix` | [x] |
| B63 | `c2GJK` | degenerate proxies: zero-radius circle, zero-extent AABB, inverted AABB, zero-length capsule | `b63_degenerate_shapes` | [x] |
| B64 | `c2GJK` | huge coordinates (`1e18`, `1e30`) forcing `inf`/`NaN` inside the simplex and the `d1>d0` early break | `b64_huge_coordinates` | [x] |

### Level 3 — boolean collision API + public entry point (`tests/phase_b_level3.rs`)

| #  | entry point(s) | configuration (options set + input shape) | differential test | [x] |
|----|----------------|-------------------------------------------|-------------------|-----|
| B65 | `c2AABBtoAABB` | random pairs: disjoint on x / on y / overlapping / touching / contained / inverted / `NaN` | `b65_aabb_to_aabb` | [x] |
| B66 | `c2CircletoCircle` | disjoint / touching (`d2==r2`) / overlapping / concentric / `r=0` / `r<0` / `NaN` | `b66_circle_to_circle` | [x] |
| B67 | `c2CircletoAABB` | centre inside / outside on each of the 8 Moore-neighbourhood sides / exactly on an edge / inverted AABB | `b67_circle_to_aabb` | [x] |
| B68 | `c2CircletoCapsule` | `da<0` branch, `db<0` branch, `db>=0` branch; degenerate `a==b` capsule; `r=0` | `b68_circle_to_capsule` | [x] |
| B69 | `c2AABBtoCapsule` | random AABB × capsule, disjoint / touching / overlapping (goes through full `c2GJK`) | `b69_aabb_to_capsule` | [x] |
| B70 | `c2CapsuletoCapsule` | parallel / crossing / collinear / identical / degenerate capsules | `b70_capsule_to_capsule` | [x] |
| B71 | `c2Collided` | all 9 valid `(typeA,typeB)` pairs × random shapes (checks the argument **swap** for the reversed pairs at L593/L605/L607) | `b71_collided_all_pairs` | [x] |
| B72 | `reverse_collide` | 65536 random `(x,y,r)` triples over the interesting `[-150,150]²×[0,60]` window | `b72_reverse_collide_random` | [x] |
| B73 | `reverse_collide` | boundary/degenerate `(x,y,r)`: `0`, `-0`, denormal, `FLT_MAX`, `±inf`, `NaN`, negative `r`, plus the exact tangency values for each of the 3 shapes | `b73_reverse_collide_boundaries` | [x] |
| B74 | `reverse_collide` | exhaustive integer lattice sweep `x,y ∈ [-160,160]`, `r ∈ {0,1,5,10,20,50}` → all 8 result bit-masks observed | `b74_reverse_collide_lattice_sweep` | [x] |

### Extra rows beyond the option cross-product

| #  | entry point(s) | configuration | differential test | [x] |
|----|----------------|---------------|-------------------|-----|
| B75 | every arithmetic export | full cross-product of all 8 distinct NaN classes (sign × quiet/signalling × payload) against each other and against `0`, `-0`, `±1`, `±inf` — pins the SSE NaN-payload propagation order | `bnan1_nan_payload_matrix_leaf_functions`, `bnan2_nan_payload_matrix_composites` | [x] |
| B76 | all 38 exports | every exported symbol is proven to be invoked differentially at least once | `g7_every_exported_symbol_is_exercised_differentially` | [x] |
| B77 | `nm -D` | symbol-set parity between the two `.so`s, enforced from inside the suite so it re-runs for every configuration | `a04_nm_dynamic_symbol_parity` | [x] |
| B78 | build config | the C source still has 0 preprocessor conditionals and `Cargo.toml` still has no `[features]` — a guard so this table cannot silently go stale | `a05_no_feature_gated_code_paths_exist` | [x] |

## Branch-coverage evidence

The multi-way branch rows are not merely "hopefully" covered: the tests classify
which branch each random input takes (using the C library's own primitives to do
the classification, so it cannot drift from the implementation under test) and
**assert every branch was hit**. Observed counts from the fixed seeds:

```
c22   3 branches   [9487, 5556, 17728]
c23   7 branches   [6153, 5491, 4482, 13535, 11479, 8748, 15648]
c2D   4 branches   [5397, 1838, 3668, 21865]
c2CircletoCapsule 3 branches [4477, 7524, 7999]
c2GJK hit path (simplex count == 3)   1516 + 1860 + 15862 cases
reverse_collide  all 8 result masks reached
```

## How to reproduce

```bash
# builds the C .so, then for EVERY feature combination:
#   cargo check --all-targets, cargo build (the cdylib the tests dlopen),
#   cargo test, and an `nm -D` symbol-parity diff
./verify_all_features.sh            # dev profile
./verify_all_features.sh --release  # release profile (opt + panic = "abort")
```
