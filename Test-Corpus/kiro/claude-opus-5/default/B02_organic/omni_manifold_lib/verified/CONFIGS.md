# CONFIGS.md — configuration-surface table (Phase A, gates Phase B)

## Axes the C actually branches on

Derived from `c_src/src/lib.c` + `c_src/include/lib.h`, not from guesswork.

**A. Shape-type axis (the `C2_TYPE` enum, 4 values + out-of-range).**
`c2MakeProxy` (`lib.c:126`), `c2Collide` (`lib.c:855`) and `ptr_from_parts`
(`lib.c:906`) all `switch` on it. Note `C2_TYPE_CAPSULE == 0`,
`CIRCLE == 1`, `AABB == 2`, `POLY == 3`. `c2Collide` is a full 4×4 dispatch of
which only the 9 non-poly cells are implemented; `c2MakeProxy` and
`ptr_from_parts` have no poly case at all.

**B. `c2GJK` runtime options** (`lib.c:420`, 11 parameters):
1. `ax_ptr` — `NULL` (⇒ identity) / identity / rotation+translation
2. `bx_ptr` — same three
3. `use_radius` — `0` / non-zero (selects the radius-shrink post-pass at `lib.c:562`)
4. `cache` — `NULL` / zeroed (`count == 0`, cache rejected) / primed with a
   previous run's simplex (`count ∈ {1,2,3}`, cache *read*) / primed with
   out-of-range `iA`/`iB`
5. `outA` / `outB` — `NULL` vs written
6. `iterations` — `NULL` vs written
7. `typeA` × `typeB` — axis A

**C. Simplex-state axis** for the low-level entry points `c22`, `c23`, `c2D`,
`c2L`, `c2Witness`, `c2GJKSimplexMetric`: `count ∈ {0,1,2,3,4,...}` × `div`
(`0`, `1`, arbitrary) × vertex geometry (which barycentric region the origin
falls in — `c23` alone has **7** mutually exclusive branches at `lib.c:270`).

**D. Geometric-relationship axis** (input shape): disjoint far / disjoint near /
exactly touching / shallow overlap / deep overlap / fully contained /
coincident centres.

**E. Degeneracy axis**: zero radius / negative radius / `capsule.a == capsule.b`
/ `aabb.min == aabb.max` / inverted AABB (`min > max`) / poly with duplicate
consecutive verts / `count` of 0, negative, `> 8`.

**F. Float-value axis**: normal magnitudes / very large (`1e30`) / very small
(`1e-30`, denormals) / exact zeros incl. `-0.0` / `±inf` / `NaN`.

**G. Poly axis** for `c2CapsuletoPolyManifold` / `c2Support` / `c2PlaneAt` /
`c2Norms` / `c2Incident`: vertex `count ∈ {0,1,2,3,4,5,6,7,8}`, convex vs
non-convex winding, normals consistent vs garbage, and which of the three
separating-axis `code` paths (0 = poly face, 1 = capsule `ab_h0`,
2 = capsule `ab_h1`) is selected at `lib.c:777`.

## Rows

One row per meaningful combination the C treats differently. Each row is
exercised with **many randomized inputs** (fixed seed, see
`tests/differential.rs`), comparing the C `.so` and the Rust `.so` bit-for-bit.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| **Scalar / vector primitives** | | | |
| 1 | `c2V`, `c2Add`, `c2Sub`, `c2Mulvs`, `c2Neg`, `c2Skew`, `c2CCW90` | random `f32` bit patterns incl. `±0`, `±inf`, `NaN`, denormals | [x] |
| 2 | `c2Dot`, `c2Det2`, `c2Len` | same value axis (F); checks fma-free evaluation order and `sqrtf` rounding | [x] |
| 3 | `c2Maxv`, `c2Minv` | random pairs incl. `-0.0 vs +0.0` and `NaN` on either side (C's ternary is not `fmaxf`) | [x] |
| 4 | `c2Clampv` | `a` inside / below / above `[lo,hi]`, plus **inverted** `lo > hi`, plus `NaN` in each of the 3 args | [x] |
| 5 | `c2Absv` | positives, negatives, `-0.0` (C returns `-0.0` unchanged), `NaN` | [x] |
| 6 | `c2Div`, `c2Norm` | non-zero divisor / `b == 0` / zero-length vector (⇒ `NaN`) / huge & tiny vectors | [x] |
| 7 | `c2Intersect` | `da != db` / `da == db` (⇒ `inf`/`NaN`) / `da == 0` / opposite signs | [x] |
| 8 | `c2Dist` | random plane × point, incl. `NaN` plane normal | [x] |
| **Rotations & transforms** | | | |
| 9 | `c2RotIdentity`, `c2xIdentity` | no inputs — exact bit pattern of the returned struct | [x] |
| 10 | `c2Mulrv`, `c2MulrvT` | identity rot / unit-norm rot / non-normalized rot / zero rot / `NaN` rot | [x] |
| 11 | `c2Mulxv`, `c2MulxvT` | identity `c2x` / translation only / rotation only / both / `NaN` components | [x] |
| **Poly / AABB helpers** | | | |
| 12 | `c2BBVerts` | normal AABB, degenerate (`min == max`), **inverted** (`min > max`), `NaN` bounds | [x] |
| 13 | `c2PlaneAt` | `i ∈ [0,8)` over a randomized poly (all 8 slots populated) | [x] |
| 14 | `c2Norms` | `count ∈ {1..8}`, convex CCW poly, convex CW poly, duplicate consecutive verts (⇒ `NaN` norm), `count == 0` | [x] |
| 15 | `c2Support` | `count ∈ {1..8}` × random direction, plus direction `(0,0)` (all dots equal ⇒ index 0), plus `NaN` direction | [x] |
| **`c2MakeProxy` (all 4 enum values)** | | | |
| 16 | `c2MakeProxy` | `type = C2_TYPE_CIRCLE` (1), random circle | [x] |
| 17 | `c2MakeProxy` | `type = C2_TYPE_AABB` (2), random / degenerate / inverted AABB | [x] |
| 18 | `c2MakeProxy` | `type = C2_TYPE_CAPSULE` (0), random capsule incl. `a == b` | [x] |
| 19 | `c2MakeProxy` | `type = C2_TYPE_POLY` (3) — no case; `*p` must be left byte-identical to its pre-call contents (pre-filled with a random pattern to detect any write) | [x] |
| **Simplex solvers (lowest-level entry points, driven directly)** | | | |
| 20 | `c22` | `count = 2`, random `a.p`/`b.p`; hits all 3 branches (`v <= 0`, `u <= 0`, interior) | [x] |
| 21 | `c22` | `a.p == b.p` (degenerate: `u == v == 0` ⇒ first branch) | [x] |
| 22 | `c23` | `count = 3`, random triangles — sweeps all **7** branches of `lib.c:270` | [x] |
| 23 | `c23` | origin strictly inside the triangle (final `else`, `count` stays 3) | [x] |
| 24 | `c23` | degenerate triangle (`area == 0`, collinear points) ⇒ `uABC = vABC = wABC = 0`, `div == 0` | [x] |
| 25 | `c2D` | `count = 1` / `2` with `det > 0` / `2` with `det <= 0` / `3` / out-of-range `count` | [x] |
| 26 | `c2L` | `count = 1` / `2` / `3` (⇒ `(0,0)`) / `0`, each with `div != 0` and `div == 0` | [x] |
| 27 | `c2Witness` | `count ∈ {1,2,3}` × `div ∈ {1, random, 0}` — random `sA`/`sB`/`u` | [x] |
| 28 | `c2Witness` | `count = 0` and `count = 4` (`default:` ⇒ both outputs `(0,0)`) | [x] |
| 29 | `c2GJKSimplexMetric` | `count ∈ {0,1,2,3,4}` × random simplex vertices | [x] |
| **`c2GJK` — the full option cross-product** | | | |
| 30 | `c2GJK` | CIRCLE↔CIRCLE, `ax=bx=NULL`, `use_radius=0`, no cache, all outputs requested | [x] |
| 31 | `c2GJK` | CIRCLE↔CIRCLE, `use_radius=1` (radius-shrink post-pass) | [x] |
| 32 | `c2GJK` | CIRCLE↔CAPSULE, both `use_radius` values, `ax=bx=NULL` | [x] |
| 33 | `c2GJK` | CIRCLE↔AABB, both `use_radius` values | [x] |
| 34 | `c2GJK` | CAPSULE↔CAPSULE, both `use_radius` values | [x] |
| 35 | `c2GJK` | CAPSULE↔AABB, both `use_radius` values | [x] |
| 36 | `c2GJK` | AABB↔AABB, both `use_radius` values (4-vert proxies ⇒ deepest simplex iteration) | [x] |
| 37 | `c2GJK` | any pair, **non-NULL identity** `ax_ptr`/`bx_ptr` (must equal the `NULL` result exactly) | [x] |
| 38 | `c2GJK` | any pair, `ax_ptr` = pure translation, `bx_ptr = NULL` | [x] |
| 39 | `c2GJK` | any pair, `ax_ptr` = rotation+translation, `bx_ptr` = rotation+translation (random unit `c2r`) | [x] |
| 40 | `c2GJK` | any pair, **non-normalized** `c2r` (`c*c + s*s != 1`) — the C never normalizes | [x] |
| 41 | `c2GJK` | `outA = NULL`, `outB` non-NULL (and vice versa); return value must still match | [x] |
| 42 | `c2GJK` | `iterations` non-NULL — compare the iteration count too (probes the loop-exit branch taken) | [x] |
| 43 | `c2GJK` | `cache` non-NULL, zero-initialized (`count == 0` ⇒ cache rejected); compare the **written-back** cache fields | [x] |
| 44 | `c2GJK` | `cache` non-NULL, primed by a *previous* `c2GJK` call on the same pair (`count ∈ {1,2,3}` ⇒ cache **read**), then re-run — the warm-start path at `lib.c:443` | [x] |
| 45 | `c2GJK` | `cache` primed, then shapes **moved** before the second call (metric mismatch ⇒ cache-validity test at `lib.c:464`) | [x] |
| 46 | `c2GJK` | `typeA = POLY` and/or `typeB = POLY` (proxy never filled — see ERRORS.md NOTE) with a zeroed proxy-equivalent poly | [x] |
| 47 | `c2GJK` | shapes disjoint far apart (early `d1 > d0` exit) | [x] |
| 48 | `c2GJK` | shapes deeply overlapping (`hit` path, `a = b`, `dist = 0`) | [x] |
| 49 | `c2GJK` | shapes exactly touching (`dist ≈ 0`, `use_radius=1` midpoint fallback at `lib.c:573`) | [x] |
| 50 | `c2GJK` | degenerate shapes: zero-radius circle, `capsule.a == capsule.b`, `aabb.min == aabb.max` | [x] |
| 51 | `c2GJK` | coincident shapes (identical circle vs identical circle) — duplicate-support-point `break` at `lib.c:539` | [x] |
| **Manifold generators (each public one, directly)** | | | |
| 52 | `c2CircletoCircleManifold` | disjoint / touching / shallow / deep / coincident centres / zero radii / negative radius | [x] |
| 53 | `c2CircletoAABBManifold` | circle outside / straddling a face / straddling a corner / centre inside (`d2 == 0` deep branch) / `x_overlap == y_overlap` tie / degenerate AABB / inverted AABB | [x] |
| 54 | `c2CircletoCapsuleManifold` | disjoint / overlapping / `d == 0` / degenerate capsule (`a == b` ⇒ `NaN` normal) / zero radii | [x] |
| 55 | `c2AABBtoAABBManifold` | separated on X (`dx < 0`) / on Y (`dy < 0`) / X-minimal overlap / Y-minimal overlap / `dx == dy` tie / `d.x < 0` and `d.x >= 0` sub-branches / identical boxes / degenerate / inverted | [x] |
| 56 | `c2CapsuletoCapsuleManifold` | parallel / crossing / collinear / disjoint / `d == 0` with `A.a == A.b` / zero radii | [x] |
| 57 | `c2CapsuletoPolyManifold` | `bx_ptr = NULL`, convex poly `count = 3,4,5,6,7,8`, capsule outside → `d >= 1e-6 && d >= A.r` (no manifold) | [x] |
| 58 | `c2CapsuletoPolyManifold` | `bx_ptr = NULL`, capsule in the shallow band `1e-6 <= d < A.r` (the `else if` branch) | [x] |
| 59 | `c2CapsuletoPolyManifold` | `bx_ptr = NULL`, capsule overlapping ⇒ `d < 1e-6`, separating-axis `code = 0` (poly face) | [x] |
| 60 | `c2CapsuletoPolyManifold` | overlapping ⇒ `code = 1` (capsule `ab_h0` axis wins) | [x] |
| 61 | `c2CapsuletoPolyManifold` | overlapping ⇒ `code = 2` (capsule `ab_h1` axis wins) | [x] |
| 62 | `c2CapsuletoPolyManifold` | **non-NULL** `bx_ptr`: identity / translation / rotation+translation / non-normalized rot | [x] |
| 63 | `c2CapsuletoPolyManifold` | poly with `count = 0`, `count = 1`, `count = 2` (degenerate; `index` may stay `-1`) | [x] |
| 64 | `c2CapsuletoPolyManifold` | degenerate capsule `A.a == A.b` (⇒ `ab` is `NaN`, `s0`/`s1` `NaN`, `code` forced to 0) | [x] |
| 65 | `c2CapsuletoPolyManifold` | poly whose `norms` are inconsistent with `verts` (C never validates) | [x] |
| 66 | `c2AABBtoCapsuleManifold` | AABB×capsule: disjoint / shallow / deep / degenerate AABB (`NaN` norms) / inverted AABB; also checks the **unconditional** `m->n = c2Neg(m->n)` with a pre-poisoned `m` | [x] |
| **Dispatch layer — full 4×4 + out-of-range** | | | |
| 67 | `c2Collide` | `typeA × typeB` over all 16 `{CAPSULE, CIRCLE, AABB, POLY}²` combinations, randomized shapes, `m` pre-poisoned | [x] |
| 68 | `c2Collide` | out-of-range `typeA`/`typeB` (`-1`, `4`, `99`, `INT_MAX`, `INT_MIN`) | [x] |
| 69 | `ptr_from_parts` | `typ ∈ {CIRCLE, AABB, CAPSULE}` — dereference the returned pointer and compare the allocated struct bytes | [x] |
| 70 | `ptr_from_parts` | `typ = POLY` / out of range — C falls off the end of a non-void function (indeterminate); documented, not asserted | [x] |
| **`omni_manifold` — the top-level API, full cross-product** | | | |
| 71 | `omni_manifold` | all 16 `type_a × type_b` combinations × randomized `a1..a5`/`b1..b5` in a small range (dense overlap) | [x] |
| 72 | `omni_manifold` | all 16 combinations × randomized wide range (mostly disjoint) | [x] |
| 73 | `omni_manifold` | all 16 combinations × values snapped to a coarse grid (forces exact ties, touching, coincidence, zero radii) | [x] |
| 74 | `omni_manifold` | all 16 combinations × values drawn from `{0, -0.0, ±1, ±inf, NaN, FLT_MAX, FLT_MIN, 1e-30, 1e30}` | [x] |
| 75 | `omni_manifold` | out-of-range `type_a`/`type_b` (`-1`, `4`, `99`, `INT_MAX`, `INT_MIN`) with `m` pre-poisoned — must leave `m` identical apart from `count = 0` | [x] |

## Methodology

* Both libraries are loaded with `libloading` and called **only** through their
  exported C symbols — the Rust crate is never linked directly, so the
  `#[no_mangle]`/`extern "C"` wrappers are part of what is tested.
* Comparison is on **raw bytes** (`common::raw`), never `f32 == f32`, so `-0.0`
  vs `+0.0` and differing NaN payloads are caught.
* Every output struct is **pre-poisoned** with a recognisable non-zero pattern
  (`common::poison_manifold`), so a field the C leaves untouched is compared
  rather than silently agreeing on zeros.
* Inputs are property-style randomized from a fixed-seed SplitMix64
  (`common::Rng`), over five value families: tame, grid-snapped (to force exact
  ties, touching and coincidence, which uniform floats never hit), very large,
  very small, and pathological (`±inf`, `NaN`, `±0`, `FLT_MAX`, `FLT_MIN`,
  denormals).
* `common::scrub_stack()` runs before each FFI call. See the ERRORS.md note on
  rows #37/#41: without it the C's poly path reads our leftover stack bytes and
  its own answer becomes caller-dependent.

## Row → test mapping

| rows | file / test |
|------|-------------|
| 1–19 | `tests/phase_b_primitives.rs` (`row01_…` … `row16_19_make_proxy`) |
| 20–29 | `tests/phase_b_primitives.rs` (`row20_21_c22`, `row22_23_24_c23`, `row25_26_29_c2D_c2L_metric`, `row27_28_witness`) |
| 30–42, 46–51 | `tests/phase_b_gjk.rs::rows30_42_gjk_typepairs_transforms_outparams` |
| 43–45 | `tests/phase_b_gjk.rs::rows43_45_gjk_cache`, `row44_gjk_hand_primed_cache` |
| 52–66 | `tests/phase_b_manifolds.rs` (one test per generator) |
| 67, 68 | `tests/phase_b_dispatch.rs::rows67_68_collide_all_type_pairs` |
| 69, 70 | `tests/phase_b_dispatch.rs::rows69_70_ptr_from_parts` |
| 71–75 | `tests/phase_b_dispatch.rs` (`row71_…` … `row75_…`) |

`tests/phase_c_nan_payload.rs` additionally sweeps every exported entry point
with distinct NaN / inf / signed-zero bit patterns; that is what pins the
`fx::{add_l, add_r, mul_l, mul_r}` operand-order choices in `src/lib.rs`.

Run everything, across both build profiles and every feature combination, with
`./translation/verify.sh`.
