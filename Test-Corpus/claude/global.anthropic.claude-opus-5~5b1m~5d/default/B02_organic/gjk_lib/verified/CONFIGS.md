# CONFIGS.md — Phase B configuration-surface table

## How this was derived

The axes below are read off the branches the C actually takes, not guessed:

```sh
grep -nE 'switch|case |default:|if |else' c_src/src/lib.c    # every branch
grep -nE '^[a-z].*\(|^void |^float |^int |^c2[a-z]' c_src/src/lib.c  # entry points
nm -D --defined-only c_src/build/libharvest-work-IbhHLG.so   # the real public API
```

`include/lib.h` declares only `gjk()`, but the `.so` exports **31** symbols
(nothing is `static`), so the public API is all 31 — and the low-level ones
(`c22`, `c23`, `c2D`, `c2L`, `c2Witness`, `c2Support`, `c2MakeProxy`,
`c2GJKSimplexMetric`) are driven **directly**, with hand-built `c2Simplex`
states, not only through `c2GJK`/`gjk`.

### Axis inventory

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `A1` shape type | `C2_TYPE_CIRCLE` (1 vert, r), `C2_TYPE_AABB` (4 verts, r=0), `C2_TYPE_CAPSULE` (2 verts, r) | `c2MakeProxy` switch, lib.c:109 |
| `A2` transform | `NULL` (-> `c2xIdentity`) vs a real `c2x` (translation + rotation) | lib.c:363/367 |
| `A3` `use_radius` | `0`, non-zero | lib.c:477 |
| `A4` `cache` | `NULL`; cold (`count == 0`); warm (`count` 1/2/3) | lib.c:378 |
| `A5` out params | `outA`/`outB`/`iterations` each NULL or not | lib.c:505-509 |
| `A6` separation | disjoint, touching, overlapping (`hit`), coincident | drives `c22`/`c23`/`hit` |
| `A7` radius sum | `dist > rA+rB`, `dist <= rA+rB`, `dist <= FLT_EPSILON` | lib.c:480 |
| `A8` simplex `count` | 1, 2, 3 (and the invalid ones -> `ERRORS.md`) | `c22`/`c23`/`c2D`/`c2L`/`c2Witness` switches |
| `A9` `c22` region | `v<=0` (vertex A), `u<=0` (vertex B), else (edge) | lib.c:186-195 |
| `A10` `c23` region | 7 arms: A, B, C, AB, BC, CA, interior | lib.c:217-250 |
| `A11` `c2D` orientation | `count==1`; `count==2` with `c2Det2>0` (`c2Skew`) vs `<=0` (`c2CCW90`) | lib.c:277-291 |
| `A12` `c2Support` count | 1, 2, 4, 8 (the `c2Proxy.verts[8]` max) | lib.c:293 |
| `A13` `gjk` `reverse` | low byte zero vs non-zero | lib.c:525 |
| `A14` float value class | normal, subnormal, `±0`, `±inf`, qNaN/sNaN with random payloads, `FLT_MAX`, `FLT_EPSILON` | pervasive |
| `A15` degenerate shape | AABB with `min==max` / inverted (`min>max`), capsule with `a==b`, `r==0`, huge `r` | no guard anywhere -> all valid inputs |

`#ifdef` axis: **none** — `c_src/src/lib.c` contains no conditional
compilation, and `translation/Cargo.toml` declares no `[features]`, so there is
a single build configuration (see `SYMBOLS.md`).

Every row is driven with **many randomized inputs** from a fixed-seed
(`0x5eed_1234_c0ffee`) SplitMix64 generator with a mixed value-class
distribution (`A14`), not one hand-picked value. Counts are per row and listed
in the test source.

## Table — leaf arithmetic (lowest level first)

| # | entry point(s) | configuration (options set + input shape) | [x] | test |
|---|----------------|-------------------------------------------|-----|------|
| 1 | `c2V` | random `(x,y)` over all of `A14` incl. NaN payloads | [x] | `cfg_leaf_c2v` |
| 2 | `c2Sub`, `c2Add` | random pairs; both-NaN, one-NaN, `±0`, `inf-inf`, `inf+-inf` | [x] | `cfg_leaf_add_sub` |
| 3 | `c2Mulvs` | random vector × random scalar; `0*inf`, NaN vector vs NaN scalar (destination-operand order) | [x] | `cfg_leaf_mulvs` |
| 4 | `c2Dot` | random pairs; NaN in `a` only, in `b` only, distinct payloads in both (fixes the `add_r`/`mul_r` order) | [x] | `cfg_leaf_dot` |
| 5 | `c2Det2` | random pairs; same NaN-placement matrix as row 4 | [x] | `cfg_leaf_det2` |
| 6 | `c2Len` | random vectors; `±0`, subnormals, `inf`, NaN payloads (checks `sqrtf@plt` vs `sqrtss`) | [x] | `cfg_leaf_len` |
| 7 | `c2Maxv`, `c2Minv` | random pairs incl. `+0 vs -0`, equal values, NaN in either slot | [x] | `cfg_leaf_minmax` |
| 8 | `c2Clampv` | random `(a, lo, hi)`; inverted range `lo > hi`; NaN in each of the three slots | [x] | `cfg_leaf_clampv` |
| 9 | `c2Neg`, `c2Skew`, `c2CCW90` | random vectors; `±0` sign flips; NaN sign flip **without** quieting | [x] | `cfg_leaf_neg_skew` |
| 10 | `c2Div` | random vector × random divisor incl. normal divisors (zero/NaN -> `ERRORS.md` 22-24) | [x] | `cfg_leaf_div` |
| 11 | `c2Norm` | random vectors of every magnitude class incl. subnormal and `FLT_MAX` | [x] | `cfg_leaf_norm` |
| 12 | `c2RotIdentity`, `c2xIdentity` | no inputs; exact bit pattern of the returned structs | [x] | `cfg_leaf_identities` |
| 13 | `c2Mulrv` | random `c2r` × random `c2v`; unit rotations, non-unit, NaN in `c`/`s`/`x`/`y` separately | [x] | `cfg_leaf_mulrv` |
| 14 | `c2MulrvT` | as row 13 (independent operand order: `-a.s*b.x + a.c*b.y`) | [x] | `cfg_leaf_mulrvt` |
| 15 | `c2Mulxv` | random `c2x` × random `c2v`; identity, pure translation, pure rotation, both | [x] | `cfg_leaf_mulxv` |

## Table — shape / proxy construction

| # | entry point(s) | configuration | [x] | test |
|---|----------------|---------------|-----|------|
| 16 | `c2BBVerts` | random AABBs: normal, `min == max`, inverted `min > max`, NaN corners | [x] | `cfg_bbverts` |
| 17 | `c2MakeProxy` | `type = C2_TYPE_CIRCLE`, random `c2Circle`, `r` in {0, normal, huge, NaN} — asserts all 72 proxy bytes incl. the 7 slots left untouched | [x] | `cfg_makeproxy_circle` |
| 18 | `c2MakeProxy` | `type = C2_TYPE_AABB`, random `c2AABB` incl. degenerate/inverted | [x] | `cfg_makeproxy_aabb` |
| 19 | `c2MakeProxy` | `type = C2_TYPE_CAPSULE`, random `c2Capsule`, `a == b`, `r == 0` | [x] | `cfg_makeproxy_capsule` |
| 20 | `c2MakeProxy` | pre-dirtied destination buffer for each valid type — verifies exactly which bytes are overwritten | [x] | `cfg_makeproxy_partial_write` |

## Table — simplex primitives, driven directly

| # | entry point(s) | configuration | [x] | test |
|---|----------------|---------------|-----|------|
| 21 | `c2Support` | `count = 1` | [x] | `cfg_support_counts` |
| 22 | `c2Support` | `count = 2` | [x] | `cfg_support_counts` |
| 23 | `c2Support` | `count = 4` (AABB shape) | [x] | `cfg_support_counts` |
| 24 | `c2Support` | `count = 8` (the `c2Proxy.verts[8]` maximum) | [x] | `cfg_support_counts` |
| 25 | `c2Support` | ties (`dot == dmax`, must keep the **first** index) and `±0` dots | [x] | `cfg_support_ties` |
| 26 | `c2GJKSimplexMetric` | `count = 2`, random `a.p`/`b.p` | [x] | `cfg_simplexmetric` |
| 27 | `c2GJKSimplexMetric` | `count = 3`, random `a.p`/`b.p`/`c.p` incl. collinear (`area == 0`) | [x] | `cfg_simplexmetric` |
| 28 | `c22` | `count = 2`, random `a.p`/`b.p` — all three arms (`v<=0`, `u<=0`, edge) hit and counted | [x] | `cfg_c22_all_regions` |
| 29 | `c22` | `a.p == b.p` (degenerate segment: `u == v == 0` -> first arm) | [x] | `cfg_c22_degenerate` |
| 30 | `c22` | full `c2sv` payload preserved: `sA`/`sB`/`iA`/`iB` must be copied by `s->a = s->b` | [x] | `cfg_c22_all_regions` |
| 31 | `c23` | random triangles — all 7 arms hit and counted, asserting all 152 simplex bytes | [x] | `cfg_c23_all_regions` |
| 32 | `c23` | collinear / duplicate points (`area == 0`, so `uABC = vABC = wABC = 0` -> final `else`) | [x] | `cfg_c23_degenerate` |
| 33 | `c23` | winding: triangles with `area > 0` and `area < 0` (flips which arm wins) | [x] | `cfg_c23_all_regions` |
| 34 | `c2D` | `count = 1`, random `a.p` | [x] | `cfg_c2d_counts` |
| 35 | `c2D` | `count = 2`, `c2Det2(ab, -a.p) > 0` -> `c2Skew` | [x] | `cfg_c2d_counts` |
| 36 | `c2D` | `count = 2`, `c2Det2(ab, -a.p) <= 0` -> `c2CCW90` (incl. exactly `0`) | [x] | `cfg_c2d_counts` |
| 37 | `c2Witness` | `count = 1` | [x] | `cfg_witness_counts` |
| 38 | `c2Witness` | `count = 2`, random weights and `div` | [x] | `cfg_witness_counts` |
| 39 | `c2Witness` | `count = 3`, random weights and `div` | [x] | `cfg_witness_counts` |
| 40 | `c2L` | `count = 1` | [x] | `cfg_c2l_counts` |
| 41 | `c2L` | `count = 2`, random weights and `div` | [x] | `cfg_c2l_counts` |

## Table — `c2GJK`, the composed pipeline

The 9 `typeA × typeB` combinations are each crossed with the transform,
`use_radius` and cache axes.

| # | entry point(s) | configuration | [x] | test |
|---|----------------|---------------|-----|------|
| 42 | `c2GJK` | `CIRCLE × CIRCLE`, identity transforms, `use_radius = 1`, no cache | [x] | `cfg_gjk_type_matrix` |
| 43 | `c2GJK` | `CIRCLE × AABB` | [x] | `cfg_gjk_type_matrix` |
| 44 | `c2GJK` | `CIRCLE × CAPSULE` | [x] | `cfg_gjk_type_matrix` |
| 45 | `c2GJK` | `AABB × CIRCLE` | [x] | `cfg_gjk_type_matrix` |
| 46 | `c2GJK` | `AABB × AABB` | [x] | `cfg_gjk_type_matrix` |
| 47 | `c2GJK` | `AABB × CAPSULE` | [x] | `cfg_gjk_type_matrix` |
| 48 | `c2GJK` | `CAPSULE × CIRCLE` | [x] | `cfg_gjk_type_matrix` |
| 49 | `c2GJK` | `CAPSULE × AABB` | [x] | `cfg_gjk_type_matrix` |
| 50 | `c2GJK` | `CAPSULE × CAPSULE` | [x] | `cfg_gjk_type_matrix` |
| 51 | `c2GJK` | all 9 type pairs × `ax_ptr = NULL`, `bx_ptr` real | [x] | `cfg_gjk_transform_matrix` |
| 52 | `c2GJK` | all 9 × `ax_ptr` real, `bx_ptr = NULL` | [x] | `cfg_gjk_transform_matrix` |
| 53 | `c2GJK` | all 9 × both transforms real (rotation + translation, non-unit `c2r` too) | [x] | `cfg_gjk_transform_matrix` |
| 54 | `c2GJK` | all 9 × `use_radius = 0` | [x] | `cfg_gjk_use_radius_off` |
| 55 | `c2GJK` | all 9 × `use_radius = 1` | [x] | `cfg_gjk_type_matrix` |
| 56 | `c2GJK` | `use_radius` = values other than 0/1 (`2`, `-1`, `INT_MIN`) — C tests `!= 0` | [x] | `cfg_gjk_use_radius_other` |
| 57 | `c2GJK` | overlapping shapes so `s.count` reaches 3 -> `hit` path | [x] | `cfg_gjk_overlap_hit` |
| 58 | `c2GJK` | disjoint shapes, `dist > rA+rB` -> radius-shrink path | [x] | `cfg_gjk_radius_shrink` |
| 59 | `c2GJK` | touching shapes, `dist <= rA+rB` -> midpoint path | [x] | `cfg_gjk_radius_shrink` |
| 60 | `c2GJK` | `iterations` non-NULL for all 9 type pairs — asserts the exact iteration count | [x] | `cfg_gjk_type_matrix` |
| 61 | `c2GJK` | cold cache (`count = 0`), then assert the full 36-byte written-back cache | [x] | `cfg_gjk_cache_roundtrip` |
| 62 | `c2GJK` | warm cache: feed back the cache produced by call *n* into call *n+1* with the shapes moved slightly, 3 generations deep | [x] | `cfg_gjk_cache_roundtrip` |
| 63 | `c2GJK` | hand-built warm cache, `count = 1`, in-range indices, for each of the 9 type pairs | [x] | `cfg_gjk_cache_warm_1` |
| 64 | `c2GJK` | hand-built warm cache, `count = 2`, in-range indices | [x] | `cfg_gjk_cache_warm_2` |
| 65 | `c2GJK` | hand-built warm cache, `count = 3`, in-range indices | [x] | `cfg_gjk_cache_warm_3` |
| 66 | `c2GJK` | warm cache × `metric` sweep across the `-1.0e8f` threshold and `±inf`/NaN | [x] | `cfg_gjk_cache_metric_sweep` |
| 67 | `c2GJK` | warm cache × `div` in {0, 1, random, `inf`, NaN} | [x] | `cfg_gjk_cache_div_sweep` |
| 68 | `c2GJK` | degenerate AABB (`min == max`), inverted AABB (`min > max`) | [x] | `cfg_gjk_degenerate_shapes` |
| 69 | `c2GJK` | degenerate capsule (`a == b`), `r = 0`, `r` huge | [x] | `cfg_gjk_degenerate_shapes` |
| 70 | `c2GJK` | circle with `r = 0` and with `r` huge | [x] | `cfg_gjk_degenerate_shapes` |
| 71 | `c2GJK` | coincident shapes (A and B identical geometry) | [x] | `cfg_gjk_degenerate_shapes` |
| 72 | `c2GJK` | shapes at `FLT_MAX`/subnormal scale (overflow in `c2Dot`) | [x] | `cfg_gjk_extreme_scale` |
| 73 | `c2GJK` | NaN / `inf` coordinates in either shape (loop guards all fail -> runs to the 20-iteration cap) | [x] | `err_gjk_nan_shape_coords` |
| 74 | `c2GJK` | `outA`/`outB` non-NULL, `iterations` NULL, `cache` NULL (the `gjk()` wrapper's exact argument shape) | [x] | `cfg_gjk_wrapper_reverse` |

## Table — `gjk`, the public wrapper from `include/lib.h`

| # | entry point(s) | configuration | [x] | test |
|---|----------------|---------------|-----|------|
| 75 | `gjk` | `reverse = 0`: AABB=A, capsule=B; randomized `a1..a4`, `b1..b5` | [x] | `cfg_gjk_wrapper_reverse` |
| 76 | `gjk` | `reverse = 1`: capsule=A, AABB=B | [x] | `cfg_gjk_wrapper_reverse` |
| 77 | `gjk` | `reverse` = `-1`, `2`, `0x7f`, `-128` (any non-zero low byte) | [x] | `cfg_gjk_wrapper_reverse` |
| 78 | `gjk` | overlapping AABB/capsule (hit path through the wrapper) | [x] | `cfg_gjk_wrapper_regions` |
| 79 | `gjk` | disjoint AABB/capsule at increasing separation | [x] | `cfg_gjk_wrapper_regions` |
| 80 | `gjk` | degenerate: zero-area AABB, zero-length capsule, `b5 = 0` | [x] | `cfg_gjk_wrapper_regions` |
| 81 | `gjk` | `b5` (capsule radius) passed on the **stack**, not in an XMM register (9 floats > 8 SSE arg slots) — large/NaN/negative `b5` | [x] | `cfg_gjk_wrapper_stack_arg` |
| 82 | `gjk` | fully random float bit patterns for all 9 floats (NaN/inf/subnormal soup) | [x] | `cfg_gjk_wrapper_fuzz` |

## Table — distinct-NaN-payload configurations (added after a mutation audit)

Several sites combine two values that can both be NaN simultaneously. Because an
SSE arithmetic instruction returns the **destination** operand's NaN, those sites
are only pinned when the two operands carry *different* payloads — a combination
that randomized NaNs hit far too rarely to rely on. These rows drive every float
field of the simplex with its own distinct payload.

| # | entry point(s) | configuration | [x] | test |
|---|----------------|---------------|-----|------|
| 83 | `c2GJKSimplexMetric` | `count` 1/2/3/4 × every `p` field a distinct NaN payload (alternating sign, alternating quiet/signalling) | [x] | `err_simplexmetric_distinct_nan_payloads` |
| 84 | `c22`, `c23` | all simplex fields distinct NaN payloads, so every guarded arm fails and the unguarded `else` arm stores `uABC`/`vABC`/`wABC` | [x] | `err_c22_c23_distinct_nan_payloads` |
| 85 | `c2Witness`, `c2L` | `div` NaN (making `den` NaN) × NaN vertex weights, all payloads distinct, `count` 1/2/3/4 | [x] | `err_witness_c2l_distinct_nan_payloads` |
| 86 | `c2GJK` | shape coordinates as distinct NaN payloads per field, plus NaN/finite mixes, × `use_radius` 0/1, all 7×7 shape pairs | [x] | `err_gjk_nan_shape_coords` |
| 87 | `c2GJK` | exact-integer geometry making the computed simplex metric land exactly on `-1.0e8f` and on `min_metric == 2*max_metric` | [x] | `err_gjk_cache_metric_threshold`, `err_gjk_cache_metric_double_boundary` |

## Measured branch coverage

Printed by the tests themselves (`cargo test --release -- --nocapture`), so the
claim that each arm is exercised is checked rather than asserted:

```
c22  arm hits (vertexA, vertexB, edge)        = [2731, 2725, 10544]
c23  arm hits (A, B, C, AB, BC, CA, interior) = [4424, 4323, 4319, 5067, 5123, 4966, 3778]
c2D  count=2 arms: c2Skew=3559  c2CCW90=4441
c2GJK type matrix: dist==0 in 4769 runs, dist>0 in 8731 runs
c2GJK radius arms: shrink=6404  midpoint=5596
c2GJK overlap: 3248 runs ended with dist=+0.0 and a==b
c2GJK max iterations observed: 4 (the `iter < 20` cap is unreachable)
radius-collapse arm: 123 genuine hits / 3038 candidates
```

Each of `cfg_c22_all_regions`, `cfg_c23_all_regions`, `cfg_c2d_counts`,
`cfg_gjk_type_matrix`, `cfg_gjk_radius_shrink` and `cfg_gjk_overlap_hit` **fails**
if any of its arms is under-exercised, so a future change to the input
distribution cannot silently stop covering an arm.
