# CONFIGS.md — configuration surface for VALID inputs

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axis 0 — build-time configuration

`Cargo.toml` has **no `[features]`** and the C has **no `#ifdef`/`#define` and no
CMake options** ⇒ exactly **one** configuration. `verify_all.sh` enumerates the
powerset of the `[features]` table (empty ⇒ one combination) and runs the whole
suite for it in **both** the dev and the release profile; `cargo check` is also
run with `--no-default-features` and with `--all-features`.

## Axis 1 — runtime option/mode flags the public API can set

The library is stateless; the *only* runtime mode selector is the shape-type
discriminant of the lowest-level dispatcher:

| flag | values the C branches on | code |
|------|--------------------------|------|
| `C2_TYPE typeB` of `c2CastRay` | `C2_TYPE_CIRCLE = 0` → `c2RaytoCircle`; `C2_TYPE_AABB = 1` → `c2RaytoAABB`; `C2_TYPE_CAPSULE = 2` → `c2RaytoCapsule`; anything else → falls off the switch (`ERRORS.md` row 34) | `lib.c:294-304` |

`spec_ray` (the only header-declared entry point) hard-codes
`C2_TYPE_CIRCLE`, so the AABB/capsule modes are reachable **only** through the
low-level `c2CastRay` / `c2Rayto*` exports — those are driven directly below,
together with all 14 vector/predicate primitives.

## Axis 2 — input value classes (each float parameter)

`F` ordinary finite · `Z` `+0.0` · `NZ` `-0.0` · `D` denormal (`1e-45`,
`0x007FFFFF`, `f32::MIN_POSITIVE`) · `H` huge (`f32::MAX`, `1e38`, products
overflow) · `I` `±inf` · `N` quiet `NaN` (`0x7FC00000`), signalling `NaN`
(`0x7FA00000`), negative `NaN` (`0xFFC00000`/`0xFFA00000`) · plus fully random
32-bit patterns.

## Axis 3 — input shapes the code special-cases

Ray: `A.t == 0`/`-0.0` (degenerate sweep), `A.t < 0`, `A.t == +inf`, `A.d` unit
vs. scaled vs. reversed vs. zero vector.
Circle: `r > 0`, `r == 0`, `r < 0`, `r` huge/denormal.
AABB: proper (`min < max`), degenerate (`min == max`), one-dimensional
(`min.x == max.x`), inverted (`min > max`).
Capsule: `a != b`, `a == b` (degenerate ⇒ `c2Norm(0)` NaN cascade), axis-aligned
in each of the four directions, `b` "below" `a`, `r == 0`, `r < 0`.

## Configuration rows (cross-product, pruned to what the C distinguishes)

Every row is exercised with **many randomized inputs** (fixed-seed xorshift64*
PRNG in `tests/common/mod.rs` — reproducible) and compared **bit-for-bit**
(`f32::to_bits`) between the C `.so` and the Rust `.so`: the `int` return value
*and* the full `c2Raycast` out-parameter, which is pre-filled with a sentinel so
that a missing or spurious store is detected.  Rows that target one specific
branch additionally assert branch coverage: `classify_aabb` / `classify_capsule`
re-walk the C control flow **using the C library's own exported primitives**, and
`Diff::require_tag` fails the row if the intended branch was not reached enough
times (so a mis-built generator cannot silently pass).

| #  | entry point(s) | configuration (options set + input shape) | test (`cargo test`) | [x] |
|----|----------------|-------------------------------------------|---------------------|-----|
| 1  | `c2V` | all value classes F/Z/NZ/D/H/I/N + random bit patterns | `row01_c2V_all_value_classes` (4000) | [x] |
| 2  | `c2Dot` | F×F randomized; H×H products overflowing to ±inf; engineered near-total cancellation | `row02_c2Dot_finite_and_overflow` (4000) | [x] |
| 3  | `c2Dot` | Z/NZ operands, `inf*0`, sNaN/qNaN in either operand (the operand *order* of the two `MULSS` and the `ADDSS` is observable) | `row03_c2Dot_zero_inf_nan` (4000) | [x] |
| 4  | `c2Len` | F ordinary, D denormal (`dot` underflows to 0) | `row04_c2Len_ordinary_and_denormal` (4000) | [x] |
| 5  | `c2Len` | H (`dot` overflows ⇒ `sqrtf(+inf)`), I, N (glibc `sqrtf` NaN payload) | `row05_c2Len_huge_inf_nan` (4000) | [x] |
| 6  | `c2Add`, `c2Sub` | F×F; Z/NZ sign rules (`0 + -0`, `0 - 0`); `inf - inf`; NaN×NaN | `row06_c2Add_c2Sub` (8000) | [x] |
| 7  | `c2Mulvs` | F×F; scalar Z/NZ/I/N/H (overflow to ±inf) | `row07_c2Mulvs` (4000) | [x] |
| 8  | `c2Div` | ordinary `b`; `b = ±0` (`1/0 = ±inf`, `0*inf = NaN`); `b = ±inf`; `b` denormal; **reciprocal-multiply vs. divide** rounding (verified to actually differ from `a/b` in 5172/20000 samples, so the row is meaningful) | `row08_c2Div_reciprocal_semantics` (4000 + 20000) | [x] |
| 9  | `c2Norm` | ordinary; zero vector (⇒ NaN); denormal; huge (`len = inf` ⇒ `±0`); NaN | `row09_c2Norm` (4000) | [x] |
| 10 | `c2Minv`, `c2Maxv` | F×F; `+0.0` vs `-0.0` ties (the ternary keeps the *second* argument); NaN in `a` vs. in `b` (the ternary is not commutative for NaN) | `row10_c2Minv_c2Maxv_tie_and_nan_semantics` (8000) | [x] |
| 11 | `c2Skew`, `c2CCW90` | F; Z/NZ (negation flips the sign bit); I; N (sign bit of the NaN) | `row11_c2Skew_c2CCW90` (8000) | [x] |
| 12 | `c2Absv` | negatives; `-0.0` → stays `-0.0`; `-NaN` → stays `-NaN` (both asserted against the C, unlike `fabsf`); I | `row12_c2Absv_ternary_not_fabsf` (4005) | [x] |
| 13 | `c2MulmvT` | random `c2m`; real rotation frames built with `c2Norm`+`c2CCW90` (as `c2RaytoCapsule` does); Z/I/N entries | `row13_c2MulmvT` (4000) | [x] |
| 14 | `c2AABBtoAABB` | proper boxes: separated on each of the 4 axes, touching, overlapping, contained (overlap ratio checked to be balanced) | `row14_c2AABBtoAABB_proper` (4000) | [x] |
| 15 | `c2AABBtoAABB` | degenerate (`min == max`), 1-D and inverted boxes; NaN ⇒ returns 1 | `row15_c2AABBtoAABB_degenerate_inverted_nan` (4001) | [x] |
| 16 | `c2AABBtoPoint` | point inside / on each corner / on each edge / outside each side; degenerate, 1-D and inverted boxes; NaN | `row16_c2AABBtoPoint` (4000) | [x] |
| 17 | `c2CircleToPoint` | inside / exactly on the rim (strict `<` ⇒ 0) / outside; `r = 0`, `r < 0`, `r` huge/denormal/special | `row17_c2CircleToPoint` (4000) | [x] |
| 18 | `c2RaytoCircle` | unit `A.d`, hit with `0 <= t <= A.t` (≥90 % hits required) | `row18_raytocircle_hit` (2000) | [x] |
| 19 | `c2RaytoCircle` | ray line misses (`disc < 0`, ≥90 % misses required) | `row19_raytocircle_line_miss_disc_negative` (2000) | [x] |
| 20 | `c2RaytoCircle` | tangent: lateral offset exactly `±r` and `±1 ulp` around it | `row20_raytocircle_tangent_disc_zero` (8000) | [x] |
| 21 | `c2RaytoCircle` | origin inside the circle; circle entirely behind the origin (`t < 0`) | `row21_raytocircle_origin_inside_or_behind` (2000) | [x] |
| 22 | `c2RaytoCircle` | `A.t` set to the exact hit distance, `nextafter` below/above it, and half of it (inclusive `t <= A.t` bound) | `row22_raytocircle_t_vs_A_t_boundary` (≈8000) | [x] |
| 23 | `c2RaytoCircle` | non-unit / reversed / zero `A.d`; `A.t = 0`, `-0.0`, `+inf`, negative; tiny `d` with huge `t` | `row23_raytocircle_direction_and_length_shapes` (2000) | [x] |
| 24 | `c2RaytoCircle` | `B.r = 0`, `< 0`, huge, denormal, special; NaN in any ray/circle field | `row24_raytocircle_radius_and_nan_shapes` (2000) | [x] |
| 25 | `c2RaytoAABB` | proper box, entering the `-x` face ⇒ `out->n = (-1,0)` (branch `t0`) | `row25_raytoaabb_face_neg_x` (2000, ≥667 tagged) | [x] |
| 26 | `c2RaytoAABB` | entering the `+x` face ⇒ `(1,0)` (branch `t1`) | `row26_raytoaabb_face_pos_x` (2000) | [x] |
| 27 | `c2RaytoAABB` | entering the `-y` face ⇒ `(0,-1)` (branch `t2`) | `row27_raytoaabb_face_neg_y` (2000) | [x] |
| 28 | `c2RaytoAABB` | entering the `+y` face ⇒ `(0,1)` (the final `else`) | `row28_raytoaabb_face_pos_y` (2000) | [x] |
| 29 | `c2RaytoAABB` | ties `t0==t1==t2==t3` (origin inside the box ⇒ every `t = 0`), exact corner diagonals, origin exactly on a corner — the C picks the **first** satisfied branch | `row29_raytoaabb_ties_and_corners` (2000) | [x] |
| 30 | `c2RaytoAABB` | swept bbox rejected early (`!c2AABBtoAABB`) | `row30_raytoaabb_bbox_reject` (2000, ≥1800 tagged) | [x] |
| 31 | `c2RaytoAABB` | bbox overlaps but the separating axis rejects (`d > 0`): thin diagonal just outside each corner | `row31_raytoaabb_separating_axis_reject` (2000, ≥1000 tagged) | [x] |
| 32 | `c2RaytoAABB` | axis-parallel rays in all four directions (⇒ `da == db` for the other axis' planes) | `row32_raytoaabb_axis_parallel_zero_denominator` (2000) | [x] |
| 33 | `c2RaytoAABB` | zero-length sweep (`A.t = 0` ⇒ `p1 == p0`), zero direction, origin inside the box, origin exactly on a face | `row33_raytoaabb_zero_length_sweep_and_inside` (2000) | [x] |
| 34 | `c2RaytoAABB` | degenerate box (`min == max`), 1-D box, inverted box | `row34_raytoaabb_degenerate_and_inverted_boxes` (2000) | [x] |
| 35 | `c2RaytoAABB` | huge coordinates (`c2Skew`/`c2Dot` overflow), `A.t = inf`, `1e30` direction, NaN coordinates, `±3.4e38` box | `row35_raytoaabb_extreme_values` (2000) | [x] |
| 36 | `c2RaytoCapsule` | origin inside the transformed slab bbox ⇒ early `return 1` (`out->t = +0.0`) | `row36_capsule_origin_in_slab_box` (2000, ≥1600 tagged) | [x] |
| 37 | `c2RaytoCapsule` | origin inside cap circle `a` (and outside the slab bbox) | `row37_capsule_origin_in_cap_a` (2000, ≥1400 tagged) | [x] |
| 38 | `c2RaytoCapsule` | origin inside cap circle `b` | `row38_capsule_origin_in_cap_b` (2000, ≥1400 tagged) | [x] |
| 39 | `c2RaytoCapsule` | crossing the slab and hitting the flat side with `c > 0` ⇒ `out->n = M.x` | `row39_capsule_flat_side_positive_c` (2000, ≥1000 tagged) | [x] |
| 40 | `c2RaytoCapsule` | same with `c <= 0` ⇒ `out->n = c2Skew(M.y)` | `row40_capsule_flat_side_negative_c` (2000, ≥1000 tagged) | [x] |
| 41 | `c2RaytoCapsule` | crossing whose `y <= 0` ⇒ delegate to `c2RaytoCircle(Ca)` (hits and misses) | `row41_capsule_cross_delegates_to_cap_a` (2000, ≥500 tagged) | [x] |
| 42 | `c2RaytoCapsule` | crossing whose `y >= yBb.y` ⇒ delegate to `c2RaytoCircle(Cb)` | `row42_capsule_cross_delegates_to_cap_b` (2000, ≥500 tagged) | [x] |
| 43 | `c2RaytoCapsule` | `\|yAp.x\| < B.r` with `yAp.y < 0` ⇒ `c2RaytoCircle(Ca)`; with `yAp.y >= 0` ⇒ `c2RaytoCircle(Cb)` | `row43_capsule_near_axis_delegation` (2000, both tags ≥500) | [x] |
| 44 | `c2RaytoCapsule` | ray parallel to the axis outside the slab ⇒ `return 0` **after** `*out` was overwritten (`t = +0.0`, `n = norm(b-a)` asserted) | `row44_capsule_outside_slab_returns_zero_after_writing_out` (2000) | [x] |
| 45 | `c2RaytoCapsule` | axis-aligned capsules (+x, -x, +y, -y ⇒ includes `b` "below" `a`) with random rays | `row45_capsule_axis_aligned_and_reversed` (2000) | [x] |
| 46 | `c2RaytoCapsule` | degenerate `a == b` (NaN cascade), `r = 0`, `r < 0`, huge/denormal/special `r` | `row46_capsule_degenerate_and_radius_shapes` (2000) | [x] |
| 47 | `c2RaytoCapsule` | `A.t = 0`, `-0.0`, `+inf`, negative; zero direction; `1e30` direction — includes `d = yAe.x - yAp.x = 0/±inf` | `row47_capsule_zero_and_infinite_sweeps` (2000) + `capsule_full_noise_fuzz` (8000) | [x] |
| 48 | `c2CastRay` | `typeB = C2_TYPE_CIRCLE (0)`, randomized rays × circles (hits and misses, plus NaN fields) — additionally asserted to be identical to calling `c2RaytoCircle` directly, in **both** libraries | `row48_castray_circle_mode` (9000) | [x] |
| 49 | `c2CastRay` | `typeB = C2_TYPE_AABB (1)`, same treatment vs. `c2RaytoAABB` | `row49_castray_aabb_mode` (9000) | [x] |
| 50 | `c2CastRay` | `typeB = C2_TYPE_CAPSULE (2)`, same treatment vs. `c2RaytoCapsule` | `row50_castray_capsule_mode` (9000) | [x] |
| 51 | `spec_ray` | ordinary: mouse point beyond the circle, ray origin outside ⇒ hit (≥50 % hits required) | `row51_spec_ray_ordinary_hit` (3000) | [x] |
| 52 | `spec_ray` | mouse point short of the circle ⇒ `ray.t < t_hit` ⇒ `t > A.t` miss; mouse point *inside* the circle ⇒ still a hit (the entry point comes first) | `row52_spec_ray_mouse_short_of_or_inside_circle` (3000) | [x] |
| 53 | `spec_ray` | ray origin inside the circle (`t < 0` ⇒ miss) | `row53_spec_ray_origin_inside_circle` (3000) | [x] |
| 54 | `spec_ray` | `mp == (r_p_x, r_p_y)` ⇒ `c2Norm(0,0)` NaN cascade | `row54_spec_ray_degenerate_direction` (3000) | [x] |
| 55 | `spec_ray` | `c_r = 0`, `-0.0`, `< 0`, huge, denormal; mouse point exactly at the centre | `row55_spec_ray_radius_shapes` (3000) | [x] |
| 56 | `spec_ray` | fully randomized fuzz over all 7 floats: mixed classes, the 20 special values, and completely random bit patterns | `row56_spec_ray_full_noise_fuzz` (12000) | [x] |

## Bit-exactness note: NaN payloads are a property of the *build*

`ADDSS/SUBSS/MULSS/DIVSS dest, src` return **`dest` quieted** when *both*
operands are NaN, so the compiler's choice of which operand becomes the
destination register is observable — and gcc does **not** always pick the
left-hand operand of the C expression (e.g. `c2Add`'s `a.x += b.x` compiles to
`addss %xmm1,%xmm0` with `xmm0 = b.x`, and `c2Dot`'s sum has the *second*
product as the destination). `src/lib.rs` therefore routes every `+`/`*`
through `fadd`/`fmul` helpers whose operand order was read off the reference
`.so`'s disassembly (`objdump -d`), and the NaN rows above verify it.

The reference is the **default** C build prescribed by the task
(`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON` with no `CMAKE_BUILD_TYPE`,
i.e. `-O0`, calling `sqrtf@plt`). Verified out of interest: a `-O2` build of the
*same* C source picks different destination operands, so it disagrees with the
`-O0` build on NaN payloads too (11 of 89 tests, all NaN-only rows, when the
harness is pointed at an `-O2` build via `DIFF_C_SO=…`). No translation can
match both; every non-NaN result is identical for both C builds.
