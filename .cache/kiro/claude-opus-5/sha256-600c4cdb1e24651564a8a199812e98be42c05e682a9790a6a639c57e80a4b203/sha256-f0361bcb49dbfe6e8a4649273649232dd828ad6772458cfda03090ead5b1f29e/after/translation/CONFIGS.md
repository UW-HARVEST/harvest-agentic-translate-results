# CONFIGS.md — Phase B configuration surface table

Derived mechanically from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

**A1 — `C2_TYPE` of shape A and shape B** (`c2Collided`, `c2MakeProxy`,
`c2GJK`): `C2_TYPE_CIRCLE=0`, `C2_TYPE_AABB=1`, `C2_TYPE_CAPSULE=2`.
This drives `c2MakeProxy`'s `p->count` (1 / 4 / 2) and `p->radius`
(`r` / `0` / `r`), which in turn drives `c2Support`'s loop length. 3×3 = 9
ordered pairs; `c2Collided` dispatches **6 distinct** implementations and
*swaps* the arguments for the (AABB,CIRCLE), (CAPSULE,CIRCLE) and
(CAPSULE,AABB) cases — the swap is itself a code path to test.

**A2 — `use_radius`** (`c2GJK` arg 9, lib.c:484): `0` skips the whole radius
shrink block; non-zero enters it. Inside it there are two sub-paths
(`dist > rA+rB && dist > FLT_EPSILON` vs. the midpoint collapse) plus the
`a==b ⇒ dist=0` post-check.

**A3 — transform pointers `ax_ptr` / `bx_ptr`** (lib.c:368-374): `NULL`
(⇒ `c2xIdentity`) vs. a supplied `c2x`. A supplied `c2x` further splits into
identity rotation (`c=1,s=0`), a general rotation (`c²+s²=1`), a
non-normalised `c2r` (accepted, scales the shape), and non-zero translation.
`ax.r` also feeds `c2MulrvT` in the support step.

**A4 — `cache`** (lib.c:381-407, 522-531): `NULL`; non-NULL with
`count == 0` (cold, `cache_was_good` false); non-NULL warm with
`count ∈ {1,2,3}` from a previous call (round-trip reuse); non-NULL warm and
then *re-called with moved shapes* (the interesting stale-cache path, since
the metric guard essentially always sets `cache_was_read = 1`). Cache is also
an **output** — `metric`, `count`, `iA[3]`, `iB[3]`, `div` must all match.

**A5 — `outA` / `outB` / `iterations` pointers** (lib.c:534-539): each
independently `NULL` or non-NULL.

**A6 — spatial relationship / input shape**: separated (positive distance),
just-touching, overlapping (origin enclosed ⇒ `hit=1`, `s.count==3`),
identical shapes, one shape fully inside the other, and degenerate shapes
(zero-radius circle, zero-area AABB, zero-length capsule `a==b`, inverted
AABB `min>max`, negative radius). These select between `c22`/`c23`, the
`hit` path, the early-`break` paths (23/24/25 in ERRORS.md), and the
`iter==20` cap.

**A7 — simplex `count`** for the low-level entry points `c22`, `c23`, `c2D`,
`c2L`, `c2Witness`, `c2GJKSimplexMetric`: `1`, `2`, `3` (and out-of-range,
covered in ERRORS.md). `c22`/`c23` each have 3 / 7 mutually-exclusive
outcome branches selected by the signs of `u*`/`v*`/`w*` — these are the
lowest-level entry points and are driven directly, not just through `c2GJK`.

**A8 — pure scalar/vector helpers**: `c2V`, `c2Sub`, `c2Add`, `c2Mulvs`,
`c2Dot`, `c2Det2`, `c2Len`, `c2Neg`, `c2Skew`, `c2CCW90`, `c2Div`, `c2Norm`,
`c2Maxv`, `c2Minv`, `c2Clampv`, `c2Mulrv`, `c2MulrvT`, `c2Mulxv`,
`c2RotIdentity`, `c2xIdentity`, `c2BBVerts`, `c2Support`. Input shapes:
ordinary finite, ±0.0, denormals, huge (near `FLT_MAX`), `Inf`, `NaN`
(`c2Maxv`/`c2Minv` use raw ternaries, so NaN ordering is observable).

**No compile-time axes**: `grep -c '#if' c_src/src/lib.c` → 0 `#ifdef`
branches; `translation/Cargo.toml` has no `[features]` section. The single
runtime library configuration is therefore the whole surface.

## Rows

Every row is exercised against BOTH `.so`s with many randomized inputs
(fixed seed, see `tests/differential.rs`), and asserted bit-for-bit
(`to_bits()` on floats, so `NaN` and `-0.0` are distinguished).

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `c2V`, `c2Sub`, `c2Add`, `c2Neg`, `c2Skew`, `c2CCW90` | random finite `c2v` pairs, wide exponent range | `row01_vector_algebra_finite` | [x] |
| 2 | `c2V`, `c2Sub`, `c2Add`, `c2Neg`, `c2Skew`, `c2CCW90` | special values: ±0.0, denormal, ±FLT_MAX, ±Inf, NaN | `row02_vector_algebra_special_values` | [x] |
| 3 | `c2Mulvs`, `c2Div` | random `c2v` × random scalar incl. 0.0, -0.0, Inf, NaN | `row03_mulvs_div` | [x] |
| 4 | `c2Dot`, `c2Det2` | random finite pairs + special values (ordering of `*`/`-` matters) | `row04_dot_det2` | [x] |
| 5 | `c2Len`, `c2Norm` | random finite; zero vector (`1/0` ⇒ Inf/NaN); huge (overflow in `Dot`); NaN | `row05_len_norm` | [x] |
| 6 | `c2Maxv`, `c2Minv` | random finite; equal components; ±0.0 pairs; NaN in a/b/both (raw-ternary NaN semantics) | `row06_maxv_minv` | [x] |
| 7 | `c2Clampv` | random `a` vs. ordered `lo<hi`; **inverted** `lo>hi`; `lo==hi`; NaN in any of the three | `row07_clampv` | [x] |
| 8 | `c2RotIdentity`, `c2xIdentity` | no inputs — exact bit pattern of the returned structs | `row08_identities` | [x] |
| 9 | `c2Mulrv`, `c2MulrvT` | identity rot; unit rot `(cosθ,sinθ)`; non-normalised rot; zero rot `(0,0)`; NaN/Inf rot | `row09_mulrv_mulrvT` | [x] |
| 10 | `c2Mulxv` | `c2x` = identity; rotation only; translation only; both; non-normalised `r`; NaN | `row10_mulxv` | [x] |
| 11 | `c2BBVerts` | well-ordered AABB; inverted (`min>max`); zero-area (`min==max`); NaN/Inf corners — all 4 output verts compared | `row11_bbverts` | [x] |
| 12 | `c2Support` | `count=1`; `count=2`; `count=4`; `count=8`; ties (equal dots, `>` keeps the first); random directions incl. zero dir and NaN dir | `row12_support` | [x] |
| 13 | `c2MakeProxy` | `type=CIRCLE` (count 1, radius=r) — verifies all 8 verts + radius + count | `row13_makeproxy_circle` | [x] |
| 14 | `c2MakeProxy` | `type=AABB` (count 4, radius 0), well-ordered / inverted / degenerate box | `row14_makeproxy_aabb` | [x] |
| 15 | `c2MakeProxy` | `type=CAPSULE` (count 2, radius=r), incl. `a==b` and negative `r` | `row15_makeproxy_capsule` | [x] |
| 16 | `c2GJKSimplexMetric` | `count=1` (⇒0), `count=2` (⇒`c2Len`), `count=3` (⇒`c2Det2`), randomized `p` values | `row16_simplex_metric` | [x] |
| 17 | `c22` | `v<=0` branch (⇒count 1, keep a) — randomized points satisfying it | `row17_c22_branch_v_le_zero` | [x] |
| 18 | `c22` | `u<=0` branch (⇒count 1, a=b) | `row18_c22_branch_u_le_zero` | [x] |
| 19 | `c22` | interior branch (⇒count 2, `div=u+v`) | `row19_c22_branch_interior` | [x] |
| 20 | `c22` | fully random simplexes (all 3 branches hit by chance) — full struct compared | `row20_c22_random` | [x] |
| 21 | `c23` | vertex-region branches: `vAB<=0&&uCA<=0`, `uAB<=0&&vBC<=0`, `uBC<=0&&vCA<=0` | `row21_c23_vertex_regions` | [x] |
| 22 | `c23` | edge-region branches: `wABC<=0` (AB), `uABC<=0` (BC), `vABC<=0` (CA) — incl. the `s->b=s->a; s->a=s->c` reorder | `row22_c23_edge_regions` | [x] |
| 23 | `c23` | interior branch (⇒count 3, `div=uABC+vABC+wABC`) | `row23_c23_interior` | [x] |
| 24 | `c23` | fully random simplexes incl. degenerate/collinear triangles (`area==0`) — full struct compared | `row24_c23_random` | [x] |
| 25 | `c2D` | `count=1`; `count=2` with `c2Det2>0` (⇒`c2Skew`); `count=2` with `<=0` (⇒`c2CCW90`); `count=3` | `row25_c2D` | [x] |
| 26 | `c2L` | `count=1`; `count=2` with random `u`/`div` incl. `div=0` (⇒Inf/NaN) | `row26_c2L` | [x] |
| 27 | `c2Witness` | `count=1`, `2`, `3` with random `sA`/`sB`/`u`/`div`; `div=0` (Inf/NaN propagation) | `row27_c2Witness` | [x] |
| 28 | `c2GJK` | (CIRCLE, CIRCLE), `use_radius=0`, both transforms NULL, no cache, separated / touching / overlapping | `row28_gjk_circle_circle_no_radius` | [x] |
| 29 | `c2GJK` | (CIRCLE, CIRCLE), `use_radius=1`, NULL transforms, no cache | `row29_gjk_circle_circle_radius` | [x] |
| 30 | `c2GJK` | (CIRCLE, AABB) and (AABB, CIRCLE), `use_radius` ∈ {0,1}, NULL transforms | `row30a/row30b_gjk_circle_aabb` | [x] |
| 31 | `c2GJK` | (CIRCLE, CAPSULE) and (CAPSULE, CIRCLE), `use_radius` ∈ {0,1}, NULL transforms | `row31a/row31b_gjk_circle_capsule` | [x] |
| 32 | `c2GJK` | (AABB, AABB), `use_radius` ∈ {0,1}, NULL transforms | `row32_gjk_aabb_aabb` | [x] |
| 33 | `c2GJK` | (AABB, CAPSULE) and (CAPSULE, AABB), `use_radius` ∈ {0,1}, NULL transforms | `row33a/row33b_gjk_aabb_capsule` | [x] |
| 34 | `c2GJK` | (CAPSULE, CAPSULE), `use_radius` ∈ {0,1}, NULL transforms; incl. parallel, crossing, collinear, `a==b` capsules | `row34_gjk_capsule_capsule` | [x] |
| 35 | `c2GJK` | all 9 type pairs, `ax_ptr` non-NULL identity + `bx_ptr` NULL (asymmetric transform handling) | `row35_gjk_asymmetric_transform` | [x] |
| 36 | `c2GJK` | all 9 type pairs, both transforms non-NULL with random unit rotations + random translations, `use_radius` ∈ {0,1} | `row36_gjk_both_transforms_rotated` | [x] |
| 37 | `c2GJK` | all 9 type pairs, both transforms non-NULL with **non-normalised** `c2r` (scaling) | `row37_gjk_non_normalised_transforms` | [x] |
| 38 | `c2GJK` | `outA=NULL, outB=non-NULL`; `outA=non-NULL, outB=NULL`; both NULL; `iterations=NULL` — cross product with `use_radius` | `row38_gjk_null_outparams` | [x] |
| 39 | `c2GJK` | cold cache (`count=0`) — cache struct compared field-by-field on output | `row39_gjk_cold_cache` | [x] |
| 40 | `c2GJK` | warm cache: call twice with the same shapes, cache carried over; both call results **and** both cache states compared | `row40_gjk_warm_cache_roundtrip` | [x] |
| 41 | `c2GJK` | stale cache: warm the cache, then move/resize both shapes and call again (exercises `cache_was_read=1` on a mismatched simplex) | `row41_gjk_stale_cache_moved_shapes` | [x] |
| 42 | `c2GJK` | cache reused across a *different type pair* than it was produced with (indices point at a shorter vert list) | `row42/row42b_gjk_cache_reused` | [x] |
| 43 | `c2GJK` | 20-iteration cap / early-break shapes: nearly-coincident, huge-coordinate, and denormal-separation inputs; `iterations` out-param compared | `row43_gjk_iteration_and_early_break` | [x] |
| 44 | `c2GJK` | degenerate shapes: zero-radius circle, zero-area AABB, inverted AABB, zero-length capsule, negative radii, `use_radius` ∈ {0,1} | `row44_gjk_degenerate_shapes` | [x] |
| 45 | `c2AABBtoAABB` | random pairs: separated on x, on y, on both, overlapping, touching exactly, nested, inverted, NaN coords | `row45_aabb_to_aabb` | [x] |
| 46 | `c2CircletoCircle` | random pairs: separated, touching (`d2 == r2`, strict `<` ⇒ 0), overlapping, nested, zero/negative radius | `row46_circle_to_circle` | [x] |
| 47 | `c2CircletoAABB` | circle centre inside / outside / on each edge / on each corner of the box; inverted box; zero radius | `row47_circle_to_aabb` | [x] |
| 48 | `c2CircletoCapsule` | `da<0` branch (before `a`); `db<0` branch (middle, uses `c2Dot(n,n)` division); else branch (past `b`); `a==b` degenerate capsule | `row48_circle_to_capsule` | [x] |
| 49 | `c2AABBtoCapsule` | random pairs incl. touching, nested, capsule crossing the box, `a==b` capsule, zero-area box (goes through full `c2GJK`) | `row49_aabb_to_capsule` | [x] |
| 50 | `c2CapsuletoCapsule` | random pairs incl. parallel, perpendicular, collinear, identical, `a==b`, zero radius (goes through full `c2GJK`) | `row50_capsule_to_capsule` | [x] |
| 51 | `c2Collided` | all 9 valid `(typeA, typeB)` ordered pairs with randomized shapes — verifies the argument-swap dispatch for (AABB,CIRCLE), (CAPSULE,CIRCLE), (CAPSULE,AABB) | `row51_collided_all_type_pairs` | [x] |
| 52 | `capsule` | the public `include/lib.h` entry point: randomized `(min_x,min_y,max_x,max_y,r)` over the range where the 3 hard-coded shapes interact — all 8 bitmask outcomes | `row52_capsule_entry_point` | [x] |
| 53 | `capsule` | special-value args: 0, negative `r`, huge coords, denormals, ±Inf, NaN | `row53_capsule_special_values` | [x] |

## Additional rows: adversarial bit-pattern configurations

The rows above found a whole class of divergence that ordinary randomized
finite inputs miss: the C is compiled at `-O0`, so every float expression is one
SSE instruction with a fixed destination/source assignment, and that assignment
decides which NaN payload and which **sign** survives. LLVM legitimately
commutes `fadd`/`fmul` and rewrites `-a*b + c` as `c - a*b`, which changes the
answer for NaN inputs. These rows drive arbitrary 32-bit patterns (signalling
NaNs, negative NaNs, all-ones payloads, ±Inf, denormals, QNaN-indefinite)
through every export at 40 000 iterations each. Tests in `tests/nan_fuzz.rs`.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 54 | `c2V`, `c2Sub`, `c2Add`, `c2Neg`, `c2Skew`, `c2CCW90`, `c2Mulvs`, `c2Div`, `c2Len`, `c2Dot`, `c2Det2`, `c2Norm`, `c2Maxv`, `c2Minv`, `c2Clampv` | arbitrary 32-bit patterns from an adversarial pool + fully random bits | `fuzz_leaf_vector_ops` | [x] |
| 55 | `c2Mulrv`, `c2MulrvT`, `c2Mulxv`, `c2RotIdentity`, `c2xIdentity` | adversarial `c2r` (NaN `c`, NaN `s`, both) × adversarial `c2v`; this is the configuration that exposed the `-a.s * b.x` reassociation | `fuzz_rotations_and_transforms` | [x] |
| 56 | `c22`, `c23`, `c2D`, `c2L`, `c2Witness`, `c2GJKSimplexMetric` | simplexes with adversarial `p`/`sA`/`sB`/`u`/`div` bit patterns at counts 1, 2, 3 — full struct compared after the call | `fuzz_simplex_reductions` | [x] |
| 57 | `c2Support`, `c2BBVerts`, `c2MakeProxy` | adversarial verts/boxes/circles/capsules; every vert count 1..8; all 8 proxy slots and both proxy scalars compared | `fuzz_support_bbverts_makeproxy` | [x] |
| 58 | `c2GJK` | all 9 type pairs × adversarial geometry × `use_radius` ∈ {0,1} × NULL/non-NULL `ax`/`bx` (adversarial `c2x`) × NULL/non-NULL out-params × NULL/cold cache | `fuzz_gjk_all_type_pairs` | [x] |
| 59 | all six boolean wrappers + `c2Collided` | adversarial geometry through every wrapper, plus `c2Collided` over all 9 ordered type pairs with the same shapes | `fuzz_boolean_wrappers_and_collided` | [x] |
| 60 | `capsule` | 160 000 adversarial argument tuples through the public `include/lib.h` entry point | `fuzz_public_capsule_entry_point` | [x] |
| 61 | `c2GJK` | cache **×** transform interaction: warm the cache under a rotated+translated `ax`/`bx`, replay in the same frame, replay after rotating `ax`, then replay with both transforms dropped to `NULL` — 4 chained calls, all results and all cache states compared | `row61_gjk_cache_with_transforms` | [x] |

## Note on the binary-executable requirement

`c_src/CMakeLists.txt` contains a single `add_library(... SHARED src/lib.c)` and
no `add_executable`; `grep -n add_executable c_src/CMakeLists.txt` returns
nothing, and the crate declares `crate-type = ["cdylib"]` with no `[[bin]]`.
There is therefore no driver binary and no stdout to compare — the entire
observable surface is the 38 exported symbols, which is what the tests drive.
