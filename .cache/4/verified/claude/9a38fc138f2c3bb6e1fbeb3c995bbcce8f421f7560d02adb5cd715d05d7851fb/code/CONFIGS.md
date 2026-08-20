# CONFIGS.md — Configuration-surface table (Phase A → gate for Phase B)

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the axes
`c_src/src/lib.c` actually branches on, not from what looks important.

## The axes the C code branches on

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `C2_TYPE typeA` | `CIRCLE` \| `AABB` \| `CAPSULE` | `c2MakeProxy` switch (l.114), `c2Collided` outer switch (l.577) |
| `C2_TYPE typeB` | `CIRCLE` \| `AABB` \| `CAPSULE` | `c2MakeProxy`, `c2Collided` inner switches |
| proxy vertex count implied by type | `1` (circle) \| `4` (aabb) \| `2` (capsule) | `c2MakeProxy` → drives `c2Support`'s loop trip count |
| proxy radius implied by type | `c->r` \| `0` \| `c->r` | `c2MakeProxy` → drives the `use_radius` shrink |
| `const c2x *ax_ptr` | `NULL` (⇒ identity) \| non-`NULL` | `c2GJK` l.368 |
| `const c2x *bx_ptr` | `NULL` (⇒ identity) \| non-`NULL` | `c2GJK` l.372 |
| transform content | identity \| translation only \| rotation only \| rotation+translation \| non-normalised `c2r` | `c2Mulxv` / `c2MulrvT` are pure math, no normalisation check |
| `int use_radius` | `0` \| non-zero | `c2GJK` l.482 |
| `c2GJKCache *cache` | `NULL` \| cold (`count == 0`) \| warm (written by a previous call) | `c2GJK` l.383, l.384, l.500 |
| `c2v *outA`, `c2v *outB`, `int *iterations` | `NULL` \| non-`NULL`, independently | `c2GJK` l.510, 512, 514 |
| simplex `count` (low-level entry points) | `1` \| `2` \| `3` \| other | `c22`, `c23`, `c2D`, `c2L`, `c2Witness`, `c2GJKSimplexMetric` |
| `c22` branch | `v<=0` \| `u<=0` \| interior | l.191/195/200 |
| `c23` branch | 3 vertex branches, 3 edge branches, interior | l.222…255 |
| `c2Support` `count` | `<=1` \| `2` \| `4` \| `8`; ties vs unique maximum | l.298-308 |
| GJK termination | `hit` (`count==3`) \| `d1>d0` \| tiny `d` \| duplicate support \| `iter==20` | l.441-472 |
| geometric relation | deeply overlapping \| exactly touching \| just separated \| far apart | strict `<` / `<=` comparisons everywhere |
| shape degeneracy | `r == 0`, `r < 0`, capsule `a == b`, AABB `min == max`, AABB `min > max` | no validation anywhere |
| coordinate magnitude | ~1 \| ~200 \| `1e30`/`FLT_MAX` (overflow to `inf`) \| `1e-40` (denormal) \| `±0` | pure float math |
| float specials | `+0`, `-0`, `+inf`, `-inf`, `NaN`, `FLT_MAX`, `FLT_MIN`, `FLT_EPSILON` | all comparisons |

## Rows

Each row = one meaningful combination the C treats differently. Every row is
driven with **many randomised inputs** (fixed seed `0x5EED_C2C2_A11CE`,
splitmix64 PRNG, so runs are reproducible) through **both** `.so`s via `dlsym`,
and every output — return value *and* every byte of every out-parameter struct —
is compared **bit-for-bit** (`f32::to_bits`).

`[x]` = row passes across its randomised inputs.

### Level 0 — pure vector/scalar math (lowest-level entry points)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|------------------------------------------|-----|
| B01 | `c2V`, `c2Sub`, `c2Add`, `c2Neg`, `c2Skew`, `c2CCW90`, `c2Mulvs` | 4096 random finite coords in ±200 | [x] |
| B02 | same as B01 | huge magnitudes ±1e30 … ±`FLT_MAX` (products/sums overflow to `±inf`) | [x] |
| B03 | same as B01 | denormals / ±1e-40 (products underflow to `±0`) | [x] |
| B04 | same as B01 | full special-value cross product: `{+0,-0,+inf,-inf,NaN,FLT_MAX,FLT_MIN,eps}²` | [x] |
| B05 | `c2Dot`, `c2Det2` | random finite; plus near-parallel / near-equal vectors (catastrophic cancellation) | [x] |
| B06 | `c2Dot`, `c2Det2` | `inf`·`0` ⇒ `NaN`, mixed-sign `inf`, `NaN` operands | [x] |
| B07 | `c2Len`, `c2Div`, `c2Norm` | random finite, unit, zero vector, huge (dot overflows before `sqrtf`), denormal | [x] |
| B08 | `c2Maxv`, `c2Minv`, `c2Clampv` | random finite; `lo > hi` (inverted clamp range); `lo == hi` | [x] |
| B09 | `c2Maxv`, `c2Minv`, `c2Clampv` | `±0.0` and `NaN` operands (strict `>` / `<` ⇒ which argument wins is order-dependent) | [x] |
| B10 | `c2RotIdentity`, `c2xIdentity` | no inputs — exact bit pattern of the returned structs | [x] |
| B11 | `c2Mulrv`, `c2MulrvT`, `c2Mulxv` | normalised rotations from random angles × random vectors | [x] |
| B12 | `c2Mulrv`, `c2MulrvT`, `c2Mulxv` | **non-normalised** `c2r` (arbitrary `c`,`s`), huge/`NaN`/`inf` components, random `c2x.p` | [x] |

### Level 1 — proxy construction

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|------------------------------------------|-----|
| B13 | `c2BBVerts` | well-formed AABBs (`min < max`), random | [x] |
| B14 | `c2BBVerts` | `min == max` (degenerate), `min > max` (inverted), special values | [x] |
| B15 | `c2MakeProxy` | `type = CIRCLE`, random circles; all 72 bytes of `c2Proxy` compared | [x] |
| B16 | `c2MakeProxy` | `type = AABB`, random boxes incl. inverted/degenerate | [x] |
| B17 | `c2MakeProxy` | `type = CAPSULE`, random capsules incl. `a == b`, `r <= 0` | [x] |
| B18 | `c2MakeProxy` | output buffer **pre-poisoned** with a known pattern, each valid type: the slots the C never writes (`verts[1..8]` for circle, `verts[2..8]` for capsule, `verts[4..8]` for aabb) must keep the poison in both libraries | [x] |

### Level 2 — simplex primitives (driven directly, not via `c2GJK`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|------------------------------------------|-----|
| B19 | `c2Support` | `count = 1`, random verts, random direction incl. `(0,0)` | [x] |
| B20 | `c2Support` | `count = 2` (capsule proxy shape), random | [x] |
| B21 | `c2Support` | `count = 4` (AABB proxy shape), random | [x] |
| B22 | `c2Support` | `count = 8` (full proxy), random; plus all-equal verts (tie ⇒ first index), `NaN` dots | [x] |
| B23 | `c22` | random 2-simplex; **all three branches** (`v<=0`, `u<=0`, interior) required to be covered; whole 152-byte `c2Simplex` compared after the in-place mutation | [x] |
| B24 | `c23` | random 3-simplex; **all seven branches** required to be covered; whole struct compared | [x] |
| B25 | `c23` | colinear / duplicate points (`area == 0` ⇒ `uABC=vABC=wABC=0` ⇒ interior branch with `div == 0`) | [x] |
| B26 | `c2GJKSimplexMetric` | `count ∈ {1,2,3}` × random simplex points (incl. huge ⇒ `det2` overflow) | [x] |
| B27 | `c2D` | `count ∈ {1,2,3}` × random simplex, both signs of `c2Det2(ab, -a)` | [x] |
| B28 | `c2L` | `count ∈ {1,2}` × random `div` (incl. `0`, huge, negative) and random `u` weights | [x] |
| B29 | `c2Witness` | `count ∈ {1,2,3}` × random `div`/`u`/`sA`/`sB` | [x] |
| B30 | `c22` → `c2L` → `c2D` → `c2Support` chained by hand (one GJK iteration open-coded) | random simplexes, verifying the composed pipeline, not just each wrapper | [x] |

### Level 3 — `c2GJK` (the full option cross-product)

All rows use randomised shapes drawn from a mix of overlapping / touching /
separated / far / degenerate generators, and compare the `f32` return **plus**
`outA`, `outB`, `iterations` and every field of `c2GJKCache`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|------------------------------------------|-----|
| B31 | `c2GJK` | `circle × circle`, `ax=bx=NULL`, `use_radius=1`, `cache=NULL`, all out-params | [x] |
| B32 | `c2GJK` | `circle × circle`, `use_radius=0` | [x] |
| B33 | `c2GJK` | `circle × aabb`, `use_radius ∈ {0,1}` | [x] |
| B34 | `c2GJK` | `circle × capsule`, `use_radius ∈ {0,1}` | [x] |
| B35 | `c2GJK` | `aabb × circle`, `use_radius ∈ {0,1}` | [x] |
| B36 | `c2GJK` | `aabb × aabb`, `use_radius ∈ {0,1}` | [x] |
| B37 | `c2GJK` | `aabb × capsule`, `use_radius ∈ {0,1}` | [x] |
| B38 | `c2GJK` | `capsule × circle`, `use_radius ∈ {0,1}` | [x] |
| B39 | `c2GJK` | `capsule × aabb`, `use_radius ∈ {0,1}` | [x] |
| B40 | `c2GJK` | `capsule × capsule`, `use_radius ∈ {0,1}` | [x] |
| B41 | `c2GJK` | all 9 type pairs × non-`NULL` **identity** `ax`/`bx` (must equal the `NULL` case) | [x] |
| B42 | `c2GJK` | all 9 type pairs × `ax` = rotation+translation, `bx = NULL` | [x] |
| B43 | `c2GJK` | all 9 type pairs × `ax = NULL`, `bx` = rotation+translation | [x] |
| B44 | `c2GJK` | all 9 type pairs × **both** `ax`,`bx` rotated+translated | [x] |
| B45 | `c2GJK` | all 9 type pairs × translation-only transforms, and rotation-only transforms | [x] |
| B46 | `c2GJK` | all 9 type pairs × **non-normalised** `c2r` in the transforms (scale/shear) | [x] |
| B47 | `c2GJK` | all 9 type pairs × cold cache (`count = 0`), cache write-back compared field by field | [x] |
| B48 | `c2GJK` | all 9 type pairs × **warm** cache: same call issued twice, second one reads the cache the first wrote; both results and the final cache compared | [x] |
| B49 | `c2GJK` | all 9 type pairs × warm cache with the shapes **moved** between calls (real incremental usage: 4 successive frames) | [x] |
| B50 | `c2GJK` | all 9 type pairs × warm cache **and** transforms **and** `use_radius=0` (three axes at once) | [x] |
| B51 | `c2GJK` | out-param subsets: `outA` only, `outB` only, `iterations` only, none at all, cache-only | [x] |
| B52 | `c2GJK` | deeply overlapping shapes (origin enclosed ⇒ `hit` path, `count == 3`), all 9 pairs | [x] |
| B53 | `c2GJK` | exactly touching shapes (integer coordinates chosen so the distance is exactly `rA+rB`), all 9 pairs | [x] |
| B54 | `c2GJK` | far-apart shapes, and coordinates of magnitude `1e18`/`1e30` (metric/`det2` overflow) | [x] |
| B55 | `c2GJK` | degenerate shapes: `r = 0`, `r < 0`, capsule `a == b`, AABB `min == max`, AABB `min > max`, all 9 pairs | [x] |
| B56 | `c2GJK` | identical shapes at the identical position (`A` and `B` byte-identical ⇒ core distance 0) | [x] |
| B57 | `c2GJK` | aliased arguments: the **same pointer** passed as both `A` and `B` | [x] |

### Level 4 — boolean shape-pair predicates

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|------------------------------------------|-----|
| B58 | `c2AABBtoAABB` | random pairs: overlapping, edge-touching, corner-touching, separated, nested, inverted, degenerate | [x] |
| B59 | `c2CircletoCircle` | random: overlapping, exactly touching, concentric, separated, `r = 0`, `r < 0` | [x] |
| B60 | `c2CircletoAABB` | random: centre inside / on an edge / at a corner / outside; `r = 0`; inverted box | [x] |
| B61 | `c2CircletoCapsule` | random covering **all three** distance branches (`da<0`, `db<0`, interior) with per-branch coverage counts; degenerate capsule `a == b` | [x] |
| B62 | `c2AABBtoCapsule` | random: overlapping / touching / separated; capsule crossing a corner; degenerate box and capsule | [x] |
| B63 | `c2CapsuletoCapsule` | random: crossing, parallel, colinear, coincident, separated, degenerate (`a == b`) | [x] |
| B64 | all six predicates | huge-magnitude coordinates and special values (`±inf`, `NaN`, `FLT_MAX`, denormals) | [x] |

### Level 5 — `c2Collided` dispatcher

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|------------------------------------------|-----|
| B65 | `c2Collided` | all 9 `(typeA, typeB)` combinations × random shapes (this also pins the C's **swapped operands** in the `AABB × CIRCLE` case) | [x] |
| B66 | `c2Collided` | all 9 combinations × degenerate/special shapes | [x] |
| B67 | `c2Collided` | aliased arguments (`A == B` pointer) for the 3 same-type combinations | [x] |

### Level 6 — `capsule` (the one entry point in `include/lib.h`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|------------------------------------------|-----|
| B68 | `capsule` | 20 000 random finite arguments in ±200, `r ∈ [0,60]` | [x] |
| B69 | `capsule` | arguments chosen to hit **each of the 8 possible return bit patterns** (0…7), with a coverage assertion | [x] |
| B70 | `capsule` | huge / denormal / negative-`r` / special-value arguments | [x] |
| B71 | `capsule` | grid sweep over the region containing all three hard-coded reference shapes (`circle` at `(-70,0) r20`, `aabb` `[-40,-40]..[-15,-15]`, `capsule` `(-40,40)..(-20,100) r10`) — 44 100 points | [x] |

## Row → test mapping (mechanically extracted)

Generated by scanning `tests/phase_b_lowlevel.rs` (B01–B30) and
`tests/phase_b_gjk.rs` (B31–B71) for each row id, so it cannot drift from the
tests. All 71 rows are covered and passing.

| row | differential test(s) |
|-----|----------------------|
| B01 | `B01_vector_ops_ordinary` |
| B02 | `B02_vector_ops_huge` |
| B03 | `B03_vector_ops_denormal` |
| B04 | `B04_vector_ops_specials_cross_product` |
| B05 | `B05_dot_det2_finite_and_cancellation` |
| B06 | `B06_dot_det2_specials` |
| B07 | `B07_len_div_norm` |
| B08 | `B08_minv_maxv_clampv_ordinary` |
| B09 | `B09_minv_maxv_clampv_specials` |
| B10 | `B10_identities` |
| B11 | `B11_rotations_normalised` |
| B12 | `B12_rotations_non_normalised_and_specials` |
| B13 | `B13_bbverts_wellformed` |
| B14 | `B14_bbverts_degenerate_and_specials` |
| B15 | `B15_makeproxy_circle` |
| B16 | `B16_makeproxy_aabb` |
| B17 | `B17_makeproxy_capsule` |
| B18 | `B18_makeproxy_untouched_slots_keep_poison` |
| B19 | `B19_support_count1` |
| B20 | `B20_support_count2` |
| B21 | `B21_support_count4` |
| B22 | `B22_support_count8_and_ties` |
| B23 | `B23_c22_all_branches` |
| B24 | `B24_c23_all_branches` |
| B25 | `B25_c23_colinear_and_duplicate` |
| B26 | `B26_simplex_metric` |
| B27 | `B27_c2D` |
| B28 | `B28_c2L` |
| B29 | `B29_c2Witness` |
| B30 | `B30_open_coded_gjk_iteration` |
| B31 | `all_pairs_with_xform` |
| B32 | `all_pairs_with_xform` |
| B33 | `all_pairs_with_xform` |
| B34 | `all_pairs_with_xform` |
| B35 | `all_pairs_with_xform` |
| B36 | `all_pairs_with_xform` |
| B37 | `all_pairs_with_xform` |
| B38 | `all_pairs_with_xform` |
| B39 | `all_pairs_with_xform` |
| B40 | `all_pairs_with_xform` |
| B41 | `B41_gjk_identity_transforms` |
| B42 | `B42_gjk_ax_only` |
| B43 | `B43_gjk_bx_only` |
| B44 | `B44_gjk_both_transforms` |
| B45 | `B45_gjk_translation_only_and_rotation_only` |
| B46 | `B46_gjk_non_normalised_rotations` |
| B47 | `B47_gjk_cold_cache` |
| B48 | `B48_gjk_warm_cache_same_call_twice` |
| B49 | `B49_gjk_warm_cache_moving_shapes` |
| B50 | `B50_gjk_cache_and_transforms_and_no_radius` |
| B51 | `B51_gjk_out_param_subsets` |
| B52 | `B52_gjk_deep_overlap_hit_path` |
| B53 | `B53_gjk_exact_touch` |
| B54 | `B54_gjk_far_and_huge` |
| B55 | `B55_gjk_degenerate_shapes` |
| B56 | `B56_gjk_identical_shapes` |
| B57 | `B57_gjk_aliased_arguments` |
| B58 | `B58_aabb_to_aabb` |
| B59 | `B59_circle_to_circle` |
| B60 | `B60_circle_to_aabb` |
| B61 | `B61_circle_to_capsule` |
| B62 | `B62_aabb_to_capsule` |
| B63 | `B63_capsule_to_capsule` |
| B64 | `B64_predicates_specials_and_huge` |
| B65 | `B65_collided_all_type_pairs` |
| B66 | `B66_collided_degenerate_and_special` |
| B67 | `B67_collided_aliased` |
| B68 | `B68_capsule_random` |
| B69 | `B69_capsule_all_result_bit_patterns` |
| B70 | `B70_capsule_specials` |
| B71 | `B71_capsule_grid_sweep` |

## Comparison policy and measured coverage

* Every `f32` is compared with `f32::to_bits()`, so `+0.0 != -0.0`. The one
  tolerated difference is the payload of a **mutually**-NaN result — see the
  "NaN payload bits" section of `ERRORS.md` for the disassembly-level reason and
  for why NaN-ness itself is still compared strictly.
* Structs (`c2Simplex`, `c2Proxy`, `c2GJKCache`, `c2v[8]`, …) are compared field
  by field; none of them has padding, so that covers exactly the same bytes as a
  `memcmp` while applying the NaN policy uniformly.
* Out-parameters are pre-filled with a poison pattern whose every 4-byte group is
  a *finite* float, so "the C never wrote this field" is observable and can never
  be mistaken for a tolerated NaN.
* Randomisation is a splitmix64 PRNG seeded from the fixed constant
  `SEED = 0x5EED_C2C2_A11C_E000`, so every run is reproducible.

Coverage actually achieved (printed by the tests):

| what | measured |
|------|----------|
| `c22` branches (3) | `[1367, 1428, 5397]` — all covered |
| `c23` branches (7) | `[1061, 646, 1016, 2999, 5856, 3530, 4892]` — all covered |
| `c2CircletoCapsule` distance branches (3) | `[3502, 6323, 10175]` — all covered |
| `capsule()` return values (0..7) | `[3645, 4186, 4010, 5558, 15107, 8743, 4400, 14351]` — all 8 produced |
| `c2GJK` zero-distance (`hit`) results | 2939 in row B52 |
| bit-exact comparisons performed | **7 222 349** across the 111 reporting rows |

## How to run

```sh
cargo test --offline                          # all 113 differential tests
cargo test --offline --test phase_b_lowlevel  # rows B01..B30
cargo test --offline --test phase_b_gjk       # rows B31..B71
./verify_all.sh                               # every feature combo x debug/release
./mutation_check.sh                           # prove the suite catches real bugs
```
