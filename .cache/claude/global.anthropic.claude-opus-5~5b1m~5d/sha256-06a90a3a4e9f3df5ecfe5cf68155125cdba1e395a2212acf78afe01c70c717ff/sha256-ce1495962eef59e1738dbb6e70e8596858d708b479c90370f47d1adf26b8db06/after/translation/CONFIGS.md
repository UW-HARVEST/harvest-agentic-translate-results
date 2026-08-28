# CONFIGS.md — configuration surface (Phase A)

`c_src/src/lib.c` has **no build-time options** (`grep -c '#if' src/lib.c` = 0,
no `#ifdef`, no globals, no init/teardown, no allocator hooks) and the Rust crate
has **no cargo features** (`[features]` absent from `Cargo.toml`), so the only
"configuration" axes are the **runtime mode selector** and the **input shapes**
the code branches on:

## Axes derived from the C source

| axis | values the C actually distinguishes | where |
|------|-------------------------------------|-------|
| A. shape dispatch mode | `C2_TYPE_CIRCLE(0)`, `C2_TYPE_AABB(1)`, `C2_TYPE_CAPSULE(2)` | `c2CastRay` `switch` L295 |
| B. entry level | low-level vector helpers · mid-level predicates (`c2AABBtoAABB`, `c2AABBtoPoint`, `c2CircleToPoint`) · raycasts (`c2RaytoCircle/AABB/Capsule`) · dispatcher (`c2CastRay`) · one-shot wrapper (`spec_ray`) | whole file |
| C. hit / miss outcome | hit-and-write-`out` vs reject (see `ERRORS.md`) | every `return 1` / `return 0` |
| D. ray-vs-circle sub-path | `disc<0` · `t<0` · `t>A.t` · `0<=t<=A.t` · ray origin inside circle (`t<0` because `-b-sqrt(disc)<0`) · tangent (`disc==0`) | L100–109 |
| E. ray-vs-AABB axis winner | `t0` (−x face) · `t1` (+x face) · `t2` (−y face) · `t3` (+y face) · all-equal tie (ties resolve to the *first* `>=` chain that succeeds) | L180–192 |
| F. ray-vs-AABB plane sub-path | per axis: `da<0` → 0 · `da*db>0` → 1 · `d==0` → 0 · else `da/d` | `c2RayToPlane_OneDimensional` L125 |
| G. ray-vs-capsule sub-path | origin inside the rotated bb (L245) · origin inside end-cap a (L254) · origin inside end-cap b (L256) · `\|yAp.x\|<r` + `yAp.y<0` → circle a · `\|yAp.x\|<r` + `yAp.y>=0` → circle b · side-plane hit with `y<=0` → circle a · `y>=yBb.y` → circle b · genuine side hit `c>0` (`n=M.x`) · genuine side hit `c<=0` (`n=skew(M.y)`) · full fall-through | L231–291 |
| H. AABB shape | proper (`min<max`) · inverted (`min>max`) · degenerate (`min==max`, zero area) · line (zero width or zero height) | `c2Minv`/`c2Maxv`, `c2AABBtoAABB` |
| I. ray shape | `t>0` · `t==0` (degenerate, zero-length sweep) · `t<0` (backwards) · huge `t` · unnormalised `d` · zero `d` · axis-aligned `d` · diagonal `d` | `c2Add(A.p, c2Mulvs(A.d, A.t))` |
| J. circle/capsule radius | `r>0` · `r==0` · `r<0` (behaves like `\|r\|` through `r*r`) · huge `r` | L97, L228, L264 |
| K. capsule axis | vertical · horizontal · diagonal · reversed (`b` below `a`, so `yBb.y<0` and the bb is inverted) · degenerate `a==b` | L233–242 |
| L. float value class | normals · small/large magnitudes · exact integers (exercise ties) · `±0.0` · denormals · `±inf` · NaN · random bit patterns | all arithmetic |
| M. `out` aliasing / pre-state | fresh `out` · `out` pre-filled with a poison pattern (proves *which* fields get written on which path) · `out == NULL` on non-writing paths | every `out->` store |

`spec_ray` is the only function in the public header (`include/lib.h`), but the
`.so` exports all 22 functions and an external caller can call every one of them,
so all of them are driven directly (axis B) — including through `c2CastRay`
(axis A), not only through the `spec_ray` convenience wrapper.

## Configuration rows (Phase B checklist)

Every row is run with **many randomized inputs** (fixed-seed xorshift PRNG,
`N = 20 000` per row unless noted) *plus* the hand-picked corner values of the
row's axis, and asserts bit-identical return value **and** bit-identical
`c2Raycast` out-struct (poison-prefilled) between the C `.so` and the Rust `.so`.

| #  | entry point(s) | configuration (options set + input shape) | test | ✔ |
|----|----------------|-------------------------------------------|------|---|
|  1 | `c2V` | random bit patterns + all of axis L (incl. NaN/inf/denormal/±0) | `cfg_01_c2v` | [x] |
|  2 | `c2Dot` | random normals; both operands from axis L; equal/opposite vectors | `cfg_02_c2dot` | [x] |
|  3 | `c2Len` | random; zero vector; huge (overflow to inf); denormal; NaN | `cfg_03_c2len` | [x] |
|  4 | `c2Add`, `c2Sub` | random + axis L cross product (±0 cancellation, inf−inf) | `cfg_04_add_sub` | [x] |
|  5 | `c2Mulvs` | random scalar × axis L (0·inf, −0·−0) | `cfg_05_mulvs` | [x] |
|  6 | `c2Div` | random scalar; `b=±0`; `b=±inf`; `b` denormal (reciprocal overflow) | `cfg_06_div` | [x] |
|  7 | `c2Norm` | random; unit; zero vector; huge; denormal; NaN | `cfg_07_norm` | [x] |
|  8 | `c2Minv`, `c2Maxv` | random pairs; equal; ±0 pairs; NaN in first vs second operand | `cfg_08_minv_maxv` | [x] |
|  9 | `c2Skew`, `c2CCW90`, `c2Absv` | random + axis L (`-0.0`, `-NaN` sign handling) | `cfg_09_skew_ccw90_absv` | [x] |
| 10 | `c2MulmvT` | random `c2m` (16-byte 2×SSE arg) × random `c2v`; identity; NaN rows | `cfg_10_mulmvt` | [x] |
| 11 | `c2AABBtoAABB` | random proper boxes, overlapping and disjoint (all 4 separating axes) | `cfg_11_aabbtoaabb_proper` | [x] |
| 12 | `c2AABBtoAABB` | inverted / degenerate / line boxes (axis H) + specials | `cfg_12_aabbtoaabb_degenerate` | [x] |
| 13 | `c2AABBtoPoint` | random point vs random proper box (inside / on each edge / outside) | `cfg_13_aabbtopoint` | [x] |
| 14 | `c2AABBtoPoint` | inverted/degenerate box + point on the exact boundary + specials | `cfg_14_aabbtopoint_degenerate` | [x] |
| 15 | `c2CircleToPoint` | random point vs circle: inside / exactly on rim / outside; `r=0`; `r<0`; huge `r` | `cfg_15_circletopoint` | [x] |
| 16 | `c2RaytoCircle` | proper hit (`0<=t<=A.t`), normalized `d`, `r>0` — axis D hit path | `cfg_16_raytocircle_hit` | [x] |
| 17 | `c2RaytoCircle` | unnormalized / zero / axis-aligned `d`, `A.t` ∈ {0, small, huge, negative} | `cfg_17_raytocircle_t_shapes` | [x] |
| 18 | `c2RaytoCircle` | origin inside the circle (`c<0`, `t<0`) and tangent (`disc≈0`) | `cfg_18_raytocircle_inside_tangent` | [x] |
| 19 | `c2RaytoCircle` | fully random bit-pattern rays/circles (all of axes D+I+J+L at once) | `cfg_19_raytocircle_random_bits` | [x] |
| 20 | `c2RaytoAABB` | proper box, random ray crossing it — exercises axis E winners `t0..t3` and the tie path | `cfg_20_raytoaabb_hit` | [x] |
| 21 | `c2RaytoAABB` | axis-aligned rays (`d=(±1,0)`, `(0,±1)`) → `da*db>0` / `d==0` sub-paths of axis F | `cfg_21_raytoaabb_axis_aligned` | [x] |
| 22 | `c2RaytoAABB` | ray origin inside the box; ray fully inside; zero-length ray (`A.t=0`) | `cfg_22_raytoaabb_inside` | [x] |
| 23 | `c2RaytoAABB` | degenerate/inverted/line boxes + huge coordinates | `cfg_23_raytoaabb_degenerate_box` | [x] |
| 24 | `c2RaytoAABB` | fully random bit patterns (NaN/inf in ray and box) | `cfg_24_raytoaabb_random_bits` | [x] |
| 25 | `c2RaytoCapsule` | vertical capsule, ray origin outside, genuine side hit `c>0` (axis G h) | `cfg_25_raytocapsule_side_hit_pos` | [x] |
| 26 | `c2RaytoCapsule` | vertical capsule, side hit from the other side `c<=0` (axis G i) | `cfg_26_raytocapsule_side_hit_neg` | [x] |
| 27 | `c2RaytoCapsule` | origin inside the capsule body / inside cap a / inside cap b (axis G a–c) | `cfg_27_raytocapsule_origin_inside` | [x] |
| 28 | `c2RaytoCapsule` | `\|yAp.x\|<r` delegation to circle a (`yAp.y<0`) and circle b (`yAp.y>=0`) (axis G d,e) | `cfg_28_raytocapsule_delegate_caps` | [x] |
| 29 | `c2RaytoCapsule` | side-plane hit that leaves the segment → delegation via `y<=0` / `y>=yBb.y` (axis G f,g) | `cfg_29_raytocapsule_delegate_by_y` | [x] |
| 30 | `c2RaytoCapsule` | diagonal / horizontal / reversed capsule axis, `r` ∈ {0, small, huge} (axes J+K) | `cfg_30_raytocapsule_axis_shapes` | [x] |
| 31 | `c2RaytoCapsule` | fully random bit patterns (incl. `a==b`, NaN, inf) | `cfg_31_raytocapsule_random_bits` | [x] |
| 32 | `c2CastRay` | `typeB=C2_TYPE_CIRCLE` — random rays/circles, result compared to the direct `c2RaytoCircle` call as well | `cfg_32_castray_circle` | [x] |
| 33 | `c2CastRay` | `typeB=C2_TYPE_AABB` — random rays/boxes | `cfg_33_castray_aabb` | [x] |
| 34 | `c2CastRay` | `typeB=C2_TYPE_CAPSULE` — random rays/capsules | `cfg_34_castray_capsule` | [x] |
| 35 | `c2CastRay` | the three valid modes interleaved in one random stream, `out` reused across calls (state carry-over) | `cfg_35_castray_mixed_stream` | [x] |
| 36 | `spec_ray` | random mouse point / circle / ray origin: hit configuration | `cfg_36_spec_ray_hit` | [x] |
| 37 | `spec_ray` | miss configuration (circle behind or beyond the mouse point) | `cfg_37_spec_ray_miss` | [x] |
| 38 | `spec_ray` | grid sweep of exact integer coordinates (ties, `t == A.t` boundary) | `cfg_38_spec_ray_integer_grid` | [x] |
| 39 | `spec_ray` | fully random bit patterns for all 7 floats (NaN/inf/denormal heavy) | `cfg_39_spec_ray_random_bits` | [x] |
| 40 | all raycasts | `out` pre-filled with a poison pattern and *not* reset between calls — proves which fields each path writes (axis M) | `cfg_40_out_poison_write_tracking` | [x] |

## Verification evidence

Every row above is a `#[test]` in `tests/phase_b_valid.rs` that calls **both**
`.so` files through `libloading` and compares the return value and the whole
`c2Raycast` out-struct bit for bit (poison-prefilled, so an unwritten field is
detectable).

* `SPEC_RAY_N` randomized inputs per row (default 20 000, run at 200 000 for the
  final pass) plus the hand-picked corner values of each row's axis, from a fixed
  seed per row.
* Result of the final pass: **62 929 792 comparisons, 0 hard mismatches**
  (37 157 NaN-payload-only differences, all from NaN inputs — see the table in
  `ERRORS.md`).
* Sub-path coverage is *proved*, not assumed: `tests/common/mod.rs` contains
  classifiers that recompute each function's branch conditions **using the C
  library's own exported helpers** (`c2Norm`, `c2MulmvT`, `c2AABBtoPoint`, …), and
  every row asserts that its intended sub-path was actually reached. The
  histograms printed with `--nocapture` show all sub-paths of every axis are hit:

  | function | sub-paths | covered by |
  |---|---|---|
  | `c2RaytoCircle` | `disc<0`, `t<0`, `t>A.t`, HIT, NaN | rows 16-19, 32 |
  | `c2RaytoAABB` | broadphase reject, SAT reject, no-plane-hit, winner `t0`/`t1`/`t2`/`t3` | rows 20-24, 33 |
  | `c2RaytoCapsule` | all 10 (in-bb, in-cap-a, in-cap-b, `|yAp.x|<r`→a/b, side-plane→a/b, side hit `c>0`/`c<=0`, fall-through) | rows 25-31, 34 |

  Rows 25-29 drive the capsule through a **capsule-local frame** (`CapFrame`), so
  a specific local `(lx, ly)` maps to the world point `a + lx*M.x + ly*M.y` and
  the intended branch is hit deterministically (19 973/20 000 for the `c>0` side
  hit, 20 000/20 000 for both `|yAp.x|<r` delegations, etc.).
* The full matrix (both feature combinations x dev/release cdylib x C `-O0`/`-O2`)
  is run by `./verify.sh`.
