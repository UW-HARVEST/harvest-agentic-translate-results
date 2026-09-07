# CONFIGS.md — Phase B valid-input configuration surface table

Derived mechanically from `c_src/include/lib.h` (the public header) and
`c_src/src/lib.c` (every exported function and every branch it takes).

## Axis 1 — entry points (all 22 exported symbols)

There is no init/context object and no global state, so "options" are carried
entirely in the argument structs. The public surface is layered:

| layer | symbols |
|-------|---------|
| L0 vector math | `c2V` `c2Dot` `c2Len` `c2Add` `c2Sub` `c2Mulvs` `c2Div` `c2Norm` `c2Minv` `c2Maxv` `c2Skew` `c2Absv` `c2CCW90` `c2MulmvT` |
| L1 predicates | `c2AABBtoAABB` `c2AABBtoPoint` `c2CircleToPoint` |
| L2 raycasts | `c2RaytoCircle` `c2RaytoAABB` `c2RaytoCapsule` |
| L3 dispatcher | `c2CastRay` (tag-driven, reaches all of L2) |
| L4 convenience wrapper | `spec_ray` (the only symbol in `lib.h`) |

`spec_ray` is a one-shot wrapper that hard-codes `C2_TYPE_CIRCLE` and derives
`ray.d`/`ray.t` from a mouse point. Testing only `spec_ray` would leave
`c2RaytoAABB`, `c2RaytoCapsule`, two of three `c2CastRay` tags, and every L0/L1
function completely unexercised, so the table drives all five layers directly.

## Axis 2 — runtime "options" (the fields the code branches on)

| option | values the C distinguishes | branch site |
|--------|---------------------------|-------------|
| `C2_TYPE typeB` | `CIRCLE` \| `AABB` \| `CAPSULE` | `switch` in `c2CastRay` |
| `A.t` (ray length) | `0` \| `> 0` \| `< 0` \| `inf` | `t <= A.t`; `p1 = p + d*t` |
| `A.d` (direction) | unit \| non-unit \| zero \| non-finite | never validated; scales `t` |
| origin position | outside \| exactly on surface \| inside | `t >= 0`, `c2AABBtoPoint`, `c2CircleToPoint` |
| `B.r` (radius) | `> 0` \| `0` \| `< 0` | squared in the algebra; used raw for `capsule_bb` |
| `B.min` vs `B.max` | proper \| equal \| inverted | `c2AABBtoAABB`, `half_extents` sign |
| `B.a` vs `B.b` | distinct \| equal (degenerate) | `c2Norm(b-a)` |
| axis of impact | `-x` \| `+x` \| `-y` \| `+y` | the 4-way `t0..t3` cascade |
| cap selection | slab \| cap A \| cap B | `abs(yAp.x) < B.r`, `y <= 0`, `y >= yBb.y` |

## Axis 3 — input value shapes

`normal` (moderate finite) · `zero` · `negative zero` · `small integers`
(exact-comparison bait) · `tiny` (~1e-6) · `huge` (~1e6) · `denormal` ·
`f32::MAX` · `±inf` · `NaN` with **distinct payloads** (operand-order probe) ·
`-NaN` (sign-bit probe).

## The table

Every row is driven by `translation/tests/configs.rs`. Each row runs
`N = 4000` (L0/L1) or `N = 3000` (L2–L4) randomized cases from a fixed seed
(`Rng::new(<row>)`, splitmix64) plus its hand-picked boundary cases, and
asserts the C and Rust results are **bit-identical** (`f32::to_bits`, so NaN
payloads and signed zeros must agree), including the full `c2Raycast` written
through `out`.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|------------------------------------------|------|-----|
| 1 | `c2V` | 2 floats, all shapes incl. `±0`, `NaN` payloads, `±inf` — identity/round-trip through the struct return ABI | `cfg_01_c2V` | [x] |
| 2 | `c2Dot` | both vectors normal finite | `cfg_02_c2Dot_finite` | [x] |
| 3 | `c2Dot` | pathological: `NaN` in either/both lanes with **different payloads** (pins `a.x*b.x + a.y*b.y` operand order), `±inf`, `inf*0`, overflow to `inf` | `cfg_03_c2Dot_pathological` | [x] |
| 4 | `c2Len` | normal; plus `zero` vector, `f32::MAX` (squares overflow to `inf`), denormal, `NaN`, `±inf` | `cfg_04_c2Len` | [x] |
| 5 | `c2Add` | normal, and pathological incl. `inf + -inf` → `NaN`, `-0.0 + 0.0`, two distinct `NaN` payloads per lane | `cfg_05_c2Add` | [x] |
| 6 | `c2Sub` | normal, and pathological incl. `inf - inf`, `0.0 - 0.0`, `-0.0 - 0.0` | `cfg_06_c2Sub` | [x] |
| 7 | `c2Mulvs` | scalar `= 0`, `-0`, `1`, `-1`, normal, `inf`, `NaN`; vector any shape (covers `0 * inf`) | `cfg_07_c2Mulvs` | [x] |
| 8 | `c2Div` | divisor `> 0`, `< 0`, `+0`, `-0` (→ `±inf`), `1`, `inf` (→ `0`), `NaN`, denormal, `f32::MIN_POSITIVE` (reciprocal overflows). Confirms the reciprocal-then-multiply form, not a true divide. | `cfg_08_c2Div` | [x] |
| 9 | `c2Norm` | unit, non-unit, zero vector (÷0), denormal (reciprocal overflows to `inf`), `f32::MAX` (`c2Len` → `inf`, so `1/inf` → `0`), `NaN`, `±inf` | `cfg_09_c2Norm` | [x] |
| 10 | `c2Minv` / `c2Maxv` | per-lane orderings incl. equal, `+0` vs `-0`, `NaN` in first vs second operand — ternary (not `fminf`/`fmaxf`) selection semantics | `cfg_10_c2Minv_c2Maxv` | [x] |
| 11 | `c2Skew` / `c2CCW90` / `c2Absv` | all shapes; specifically `-0.0` (ternary `abs` keeps the sign bit) and `-NaN` (negation flips the sign bit) | `cfg_11_skew_ccw90_absv` | [x] |
| 12 | `c2MulmvT` | 4-element matrix × vector; normal, then `NaN`-payload matrix so the two independent `+` operand orders are both pinned; `±inf` rows | `cfg_12_c2MulmvT` | [x] |
| 13 | `c2AABBtoAABB` | disjoint on `x` only / `y` only / both; touching edge; nested; identical; inverted (`min > max`); degenerate (`min == max`); `NaN` (all `<` false ⇒ reports overlap) | `cfg_13_c2AABBtoAABB` | [x] |
| 14 | `c2AABBtoPoint` | strictly inside; exactly on each of the 4 edges (inclusive boundary); outside each of 4 sides; corner; inverted box; `NaN` point | `cfg_14_c2AABBtoPoint` | [x] |
| 15 | `c2CircleToPoint` | inside; exactly on the rim (**exclusive** boundary ⇒ 0); outside; `r == 0`; `r < 0`; `NaN`; `inf` radius | `cfg_15_c2CircleToPoint` | [x] |
| 16 | `c2RaytoCircle` | **unit** `d`, origin outside, hit within `A.t` — the canonical hit; randomized over positions/radii | `cfg_16_raytocircle_unit_hit` | [x] |
| 17 | `c2RaytoCircle` | **non-unit** `d` (scaled and un-normalised), so `t` is in `d`-units, not distance | `cfg_17_raytocircle_nonunit_dir` | [x] |
| 18 | `c2RaytoCircle` | `A.t` shapes: `0`, tiny, exactly the hit distance, `inf`, `f32::MAX` | `cfg_18_raytocircle_t_shapes` | [x] |
| 19 | `c2RaytoCircle` | tangent (`disc ≈ 0`) and near-tangent, swept by `next_after` on the origin offset — the branch `disc < 0` right at its boundary | `cfg_19_raytocircle_tangent` | [x] |
| 20 | `c2RaytoCircle` | origin **inside** the circle (`t < 0` ⇒ reject) and origin exactly on the surface (`t == 0` ⇒ `out->n = c2Norm(0,0)`) | `cfg_20_raytocircle_origin_inside_or_on` | [x] |
| 21 | `c2RaytoCircle` | fully random shotgun: every field from `pathological()`, `out` pre-poisoned so "did not write" is observable | `cfg_21_raytocircle_shotgun` | [x] |
| 22 | `c2RaytoAABB` | axis-aligned ray hitting the `-x` face (`t0` wins the 4-way cascade) | `cfg_22_raytoaabb_face_neg_x` | [x] |
| 23 | `c2RaytoAABB` | hitting the `+x` face (`t1` wins) | `cfg_23_raytoaabb_face_pos_x` | [x] |
| 24 | `c2RaytoAABB` | hitting the `-y` face (`t2` wins) | `cfg_24_raytoaabb_face_neg_y` | [x] |
| 25 | `c2RaytoAABB` | hitting the `+y` face (`t3` wins, the `else` arm) | `cfg_25_raytoaabb_face_pos_y` | [x] |
| 26 | `c2RaytoAABB` | diagonal rays through a corner — ties in the `>=` cascade, so the *first* matching arm must be selected identically | `cfg_26_raytoaabb_corner_ties` | [x] |
| 27 | `c2RaytoAABB` | origin **inside** the box | `cfg_27_raytoaabb_origin_inside` | [x] |
| 28 | `c2RaytoAABB` | ray parallel to a slab (`da - db == 0`, the line-132 zero-denominator guard) and axis-aligned rays where `da*db > 0` returns `1.0f` | `cfg_28_raytoaabb_parallel_slabs` | [x] |
| 29 | `c2RaytoAABB` | box shapes: proper, degenerate (`min == max`), zero-width in one axis, inverted, huge, tiny | `cfg_29_raytoaabb_box_shapes` | [x] |
| 30 | `c2RaytoAABB` | `A.t` shapes `0` / normal / `inf` / `f32::MAX` / negative, with `d` unit and non-unit | `cfg_30_raytoaabb_t_shapes` | [x] |
| 31 | `c2RaytoAABB` | fully random shotgun over `pathological()` incl. `NaN`, poisoned `out` | `cfg_31_raytoaabb_shotgun` | [x] |
| 32 | `c2RaytoCapsule` | origin inside the slab ⇒ early `ret 1` with `out->t = 0`, `out->n = norm(b-a)` | `cfg_32_raytocapsule_origin_in_slab` | [x] |
| 33 | `c2RaytoCapsule` | origin inside end cap A, and inside end cap B ⇒ early `ret 1` | `cfg_33_raytocapsule_origin_in_caps` | [x] |
| 34 | `c2RaytoCapsule` | `abs(yAp.x) < B.r` with `yAp.y < 0` ⇒ delegate to `c2RaytoCircle(Ca)`; and `yAp.y >= 0` ⇒ delegate to `Cb` | `cfg_34_raytocapsule_delegate_by_yAp_y` | [x] |
| 35 | `c2RaytoCapsule` | slab-side hit: `y` strictly between `0` and `yBb.y` ⇒ `ret 1`, `out->n = M.x` (for `c > 0`) | `cfg_35_raytocapsule_slab_hit_pos_c` | [x] |
| 36 | `c2RaytoCapsule` | slab-side hit from the other side: `c < 0` ⇒ `out->n = c2Skew(M.y)` | `cfg_36_raytocapsule_slab_hit_neg_c` | [x] |
| 37 | `c2RaytoCapsule` | `y <= 0` ⇒ cap A, and `y >= yBb.y` ⇒ cap B (the post-`t` cap selection, distinct from row 34) | `cfg_37_raytocapsule_cap_by_y` | [x] |
| 38 | `c2RaytoCapsule` | capsule axis orientation sweep: vertical, horizontal, 45°, arbitrary angles, and **reversed** (`b` below `a`, so `yBb.y < 0` and `capsule_bb` is inverted) | `cfg_38_raytocapsule_orientations` | [x] |
| 39 | `c2RaytoCapsule` | radius shapes `> 0` / `0` / `< 0` / tiny / huge, crossed with hit and miss geometry | `cfg_39_raytocapsule_radius_shapes` | [x] |
| 40 | `c2RaytoCapsule` | degenerate `B.a == B.b` (`c2Norm(0)` poisons `M` with `NaN`) | `cfg_40_raytocapsule_degenerate_axis` | [x] |
| 41 | `c2RaytoCapsule` | fully random shotgun over `pathological()`, poisoned `out` | `cfg_41_raytocapsule_shotgun` | [x] |
| 42 | `c2CastRay` | `typeB = CIRCLE`, `B` → `c2Circle`; randomized rays/circles — dispatcher path must match the direct `c2RaytoCircle` call | `cfg_42_castray_circle` | [x] |
| 43 | `c2CastRay` | `typeB = AABB`, `B` → `c2AABB` | `cfg_43_castray_aabb` | [x] |
| 44 | `c2CastRay` | `typeB = CAPSULE`, `B` → `c2Capsule` | `cfg_44_castray_capsule` | [x] |
| 45 | `spec_ray` | mouse point beyond the circle, origin outside ⇒ hit; randomized over all 7 floats in the "normal" shape | `cfg_45_specray_hit` | [x] |
| 46 | `spec_ray` | mouse point short of the circle ⇒ `ray.t` too small ⇒ miss; and mouse point behind the origin (negative `ray.t`) | `cfg_46_specray_short_and_behind` | [x] |
| 47 | `spec_ray` | origin inside the circle; origin exactly on the circle | `cfg_47_specray_origin_inside_or_on` | [x] |
| 48 | `spec_ray` | fully random shotgun over `pathological()` for all 7 floats, poisoned `out` | `cfg_48_specray_shotgun` | [x] |
| 49 | composed pipeline | `c2Norm`→`c2MulmvT`→`c2AABBtoPoint`→`c2RaytoCircle` chained *across* libraries: feed the C's intermediate result into the Rust's next stage and vice versa, so a divergence that two wrappers happen to share is still caught | `cfg_49_cross_library_pipeline` | [x] |
| 50 | all 22 symbols | uniform ABI smoke sweep: every symbol called with the same pathological argument tuple, results compared bitwise — catches a wrapper whose ABI (struct-by-value, register class) differs even if the math agrees | `cfg_50_all_symbols_abi_sweep` | [x] |

## Binary executable

Neither project builds one: `c_src/CMakeLists.txt` has no `add_executable`
(only `add_library(... SHARED ...)`), and `translation/Cargo.toml` declares
`crate-type = ["cdylib"]` with no `[[bin]]` and no `src/main.rs` / `src/bin/`.
The "compare stdout of the C and Rust drivers" gate is therefore **not
applicable**; there is no stdout surface to compare. Confirmed by:

```sh
grep -c add_executable c_src/CMakeLists.txt   # 0
ls translation/src/bin 2>/dev/null            # no such directory
grep -c '\[\[bin\]\]' translation/Cargo.toml  # 0
```

## Feature combinations

`translation/Cargo.toml` has no `[features]` table and the source contains no
`#[cfg(feature = ...)]`, so the only build configuration is the default. The
`#[cfg(target_arch = "x86_64")]` gates in `float_ops` are architecture, not
feature, selections. `scripts/verify_all.sh` still loops over the two
meaningful invocations (`default` and `--no-default-features`) so the gate is
demonstrated rather than assumed.

## Result

All 50 rows pass. `translation/tests/configs.rs` contains one `#[test]` per row,
named `cfg_NN_...` to match the table, and every row was run against both the
release- and debug-built Rust `.so` under both `default` and
`--no-default-features` (see `scripts/verify_all.sh`).

Total randomized cases across the suite: roughly 900 000 differential calls,
all compared on exact bit patterns.

No divergence was found in Phase B: the translation's unusual-looking
choices — the inline-assembly `addss`/`mulss` operand pinning in `float_ops`,
the reciprocal-then-multiply form of `c2Div`, and the ternary `ter_min` /
`ter_max` / `ter_abs` helpers instead of `f32::min` / `max` / `abs` — all turn
out to be load-bearing and correct. Rows 3, 10, 11, 12, 45 and 46 are the ones
that would fail if any of them were replaced with the obvious library call:

* `c2Absv(-0.0)` must stay `-0.0` (`fabsf` would return `+0.0`).
* `c2Absv(-NaN)` must keep the sign bit set.
* `c2Minv(NaN, x)` must return `x`, and `c2Minv(x, NaN)` must return `NaN` —
  the opposite asymmetry from `fminf`.
* `c2Dot` and `c2MulmvT` must propagate the correct NaN *payload* when both
  operands are NaN, which depends on which SSE operand is the destination.
