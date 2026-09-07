# ERRORS.md — Phase C error-surface table

Mechanically derived from every `return 0` / `return !(...)` / comparison-gate in
`c_src/src/lib.c`. The library has no error enum, no `errno`, no `assert`, and no
`NULL` return; **every rejection is the integer `0`** returned from an `int`
function, and for the raycast functions the out-param is left as-is (or
partially written, in `c2RaytoCapsule`). Rows are one per distinct `return`/branch
in the C source.

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `c2RaytoCircle` | `disc = b*b - c < 0` (ray misses circle line) — L117 | returns `0`, `*out` untouched | `err_01_circle_disc_negative` |
| 2 | `c2RaytoCircle` | `t = -b - sqrtf(disc) < 0` (circle behind ray origin) — L120 | returns `0`, `*out` untouched | `err_02_circle_t_negative` |
| 3 | `c2RaytoCircle` | `t > A.t` (hit beyond ray length; incl. `A.t = 0`) — L120 | returns `0`, `*out` untouched | `err_03_circle_t_beyond_len` |
| 4 | `c2RaytoCircle` | `A.t` is NaN ⇒ `t <= A.t` false | returns `0` | `err_04_circle_t_nan` |
| 5 | `c2RaytoCircle` | `B.r` NaN ⇒ `disc` NaN ⇒ `disc < 0` false, then `t` NaN ⇒ compares false | returns `0` | `err_05_circle_r_nan` |
| 6 | `c2RaytoAABB` | `!c2AABBtoAABB(a_box, B)` — ray's own AABB disjoint from B — L162 | returns `0`, `*out` untouched | `err_06_aabb_bb_reject` |
| 7 | `c2RaytoAABB` | `d > 0` — SAT on the ray's skew normal separates — L173 | returns `0`, `*out` untouched | `err_07_aabb_sat_reject` |
| 8 | `c2RaytoAABB` | `hit == 0`, i.e. all four `t0..t3 > 1.0f` — L191/L211 | returns `0`, `*out` untouched | `err_08_aabb_no_plane_hit` |
| 9 | `c2RaytoAABB` | inverted `B` (`B.min > B.max`) ⇒ negative half-extents | whatever C computes (usually `0` via row 6/7); must match bit-for-bit | `err_09_aabb_inverted` |
| 10 | `c2RaytoAABB` | NaN in `A.p`/`A.d`/`A.t` ⇒ ternary min/max propagate NaN, `<` comparisons false | must match C exactly | `err_10_aabb_nan_inputs` |
| 11 | `c2RaytoCapsule` | ray never comes within `B.r` in capsule-local x: `!(yAe.x*yAp.x < 0 \|\| min(\|yAe.x\|,\|yAp.x\|) < B.r)` — L277/L308 | returns `0`, but `out->n = c2Norm(cap_n)` and `out->t = 0` **were already written** (L260-261) | `err_11_capsule_miss_writes_out` |
| 12 | `c2RaytoCapsule` | degenerate capsule `B.a == B.b` ⇒ `c2Norm` of zero vector ⇒ `1/0 = inf`, `0*inf = NaN` in `M` | must match C exactly (NaN `out->n`) | `err_12_capsule_degenerate` |
| 13 | `c2RaytoCapsule` | `B.r < 0` (negative radius) ⇒ `capsule_bb.min.x = -B.r > capsule_bb.max.x = B.r`, inverted box | must match C exactly | `err_13_capsule_negative_radius` |
| 14 | `c2RaytoCapsule` | delegates to `c2RaytoCircle` (L289/291/298/300) which itself rejects ⇒ `0` propagated, `*out` left as the L260-261 pre-write | `0` with pre-written `out` | `err_14_capsule_delegated_reject` |
| 15 | `c2RaytoCapsule` | `d = yAe.x - yAp.x == 0` in the else branch ⇒ `t = (c-yAp.x)/0 = ±inf` | must match C exactly | `err_15_capsule_zero_dx` |
| 16 | `c2RaytoPoly` | `den == 0 && num < 0` — ray parallel to a face plane and outside it — L347 | returns `0`, `*out` untouched | `err_16_poly_parallel_outside` |
| 17 | `c2RaytoPoly` | `hi < lo` after an iteration — interval collapsed — L356 | returns `0`, `*out` untouched | `err_17_poly_interval_empty` |
| 18 | `c2RaytoPoly` | `index == ~0` at the end — no entering face found (ray starts inside, or `hi/lo` never narrowed by a `den<0` face) — L359/L364 | returns `0`, `*out` untouched | `err_18_poly_index_unset` |
| 19 | `c2RaytoPoly` | `B->count == 0` ⇒ loop body never executes ⇒ falls to row 18 | returns `0`, `*out` untouched | `err_19_poly_count_zero` |
| 20 | `c2RaytoPoly` | `B->count < 0` (e.g. `-1`, `INT_MIN`) ⇒ `i < count` false immediately ⇒ row 18 | returns `0` | `err_20_poly_count_negative` |
| 21 | `c2RaytoPoly` | `A.t == 0` ⇒ `hi = 0` ⇒ `num < hi*den` becomes `num < 0`; typically `hi < lo` or `index==~0` | must match C exactly | `err_21_poly_zero_t` |
| 22 | `c2RaytoPoly` | `A.t < 0` (negative ray length) ⇒ `hi < lo == 0` on first `hi` update | must match C exactly | `err_22_poly_negative_t` |
| 23 | `c2RaytoPoly` | `bx_ptr == NULL` ⇒ C substitutes `c2xIdentity()` (L338). Not an error, but the explicit null check. | identical result to passing an explicit identity `c2x` | `err_23_poly_null_bx` |
| 24 | `c2RaytoPoly` | `bx.r` not a unit rotation (`c*c+s*s != 1`) — C never validates | no rejection; must match C's arithmetic exactly | `err_24_poly_nonunit_rot` |
| 25 | `c2RaytoPoly` | zero-`norms` polygon (`den == 0 && num == 0` for every face) ⇒ neither branch taken ⇒ row 18 | returns `0` | `err_25_poly_zero_norms` |
| 26 | `c2CastRay` | `typeB` out of enum range: `4`, `-1`, `INT_MAX`, `INT_MIN`, `100` — no `case` matches, falls through `switch` — L378 | returns `0`, `*out` untouched, `B` never dereferenced | `err_26_castray_bad_enum` |
| 27 | `c2CastRay` | `typeB == C2_TYPE_POLY (3)` with `bx == NULL` — the only path where `bx` may legally be `NULL` | delegates to row 23 | `err_27_castray_poly_null_bx` |
| 28 | `c2CastRay` | `typeB` valid but not `POLY`, with `bx != NULL` ⇒ `bx` is silently ignored | result independent of `bx` | `err_28_castray_bx_ignored` |
| 29 | `c2AABBtoAABB` | any of the 4 separating-axis conditions true ⇒ `!(d0\|d1\|d2\|d3)` — L134 | returns `0` | `err_29_aabbaabb_reject` |
| 30 | `c2AABBtoAABB` | NaN coordinate ⇒ every `<` false ⇒ `d0..d3 == 0` ⇒ returns `1` (a NaN box "overlaps") | returns `1` | `err_30_aabbaabb_nan` |
| 31 | `c2AABBtoPoint` | point outside on any of 4 sides — L239 | returns `0` | `err_31_aabbpoint_reject` |
| 32 | `c2AABBtoPoint` | NaN point ⇒ all comparisons false ⇒ returns `1` | returns `1` | `err_32_aabbpoint_nan` |
| 33 | `c2CircleToPoint` | `d2 >= A.r * A.r` (point on or outside the circle; **strict** `<`, so on-boundary is a miss) — L245 | returns `0` | `err_33_circlepoint_reject` |
| 34 | `c2CircleToPoint` | `A.r == 0` ⇒ `d2 < 0` never true ⇒ always `0`; `A.r < 0` ⇒ `r*r > 0` so a negative radius still "contains" points | must match C exactly | `err_34_circlepoint_zero_neg_r` |
| 35 | `c2CircleToPoint` | NaN ⇒ `d2 < r*r` false ⇒ returns `0` | returns `0` | `err_35_circlepoint_nan` |
| 36 | `c2Div` / `c2Norm` | `b == 0.0` ⇒ `1.0f/0.0f = +inf` ⇒ components become `±inf` or `NaN` (for a `0` component). No check in C. | `±inf`/`NaN` per IEEE; must match bit-for-bit | `err_36_div_by_zero` |
| 37 | `c2Div` / `c2Norm` | `b == -0.0` ⇒ `1.0f/-0.0f = -inf` | `-inf` sign must match | `err_37_div_by_negative_zero` |
| 38 | `c2Norm` | zero vector `(0,0)` ⇒ `c2Len = 0` ⇒ `0 * inf = NaN` for both lanes | both lanes NaN | `err_38_norm_zero_vector` |
| 39 | `c2Len` | negative `c2Dot` impossible, but NaN/inf input ⇒ `sqrtf(NaN) = NaN`, `sqrtf(inf) = inf` | must match | `err_39_len_special` |
| 40 | `c2Absv` / `c2Minv` / `c2Maxv` | NaN and `-0.0` inputs — C uses **ternaries**, not `fabsf`/`fminf`/`fmaxf`: `c2Absv(NaN)` keeps the sign bit, `c2Absv(-0.0)` returns `-0.0`, `c2Minv(NaN, x)` returns `x` | must match the ternary semantics bit-for-bit, **not** the Rust intrinsics | `err_40_ternary_nan_negzero` |
| 41 | `poly_ray` | `cast1`/`cast2` are always written by the hard-coded rays? No — row 18 can leave them untouched. NULL out-params are UB in C. | not testable (UB); documented, out of scope | *(n/a — UB)* |
| 42 | `c2RaytoPoly` | `B->count > 8` ⇒ reads past `verts[8]`/`norms[8]` (into `norms`, then past the struct). UB in C. | not testable (UB); Rust mirrors the unchecked raw-pointer indexing | *(n/a — UB, but see `cfg_poly_count_5_to_8`)* |
| 43 | `c2CastRay` | `B == NULL` with a valid `typeB` ⇒ NULL dereference. UB in C. | not testable (UB) | *(n/a — UB)* |

## Notes on non-rows

* There is **no** `RETURN_ERROR` macro, no error enum, no `return NULL`, no
  `assert`, and no `errno` use anywhere in `c_src/`. `grep -n 'assert\|errno\|
  RETURN_ERROR\|return NULL' c_src/src/lib.c c_src/include/lib.h` → no matches.
* `MIN`/`MAX`-style constants: none. The only fixed capacity is the literal `8`
  in `c2Poly::verts`/`norms` (row 42).
* Rows 41–43 are genuine C undefined behaviour (NULL deref / OOB read). A
  differential test cannot assert equal *behaviour* for UB, so they are recorded
  and excluded rather than silently dropped.

---

## Phase C result — every row has a passing differential test

All 40 testable rows pass (`cargo test --test phase_c`, 42 tests including the
two extra boundary tests), in both the `debug` and `release` builds:

```
test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Notes on individual rows:

* **Row 8 (`hit == 0`) is only reachable via NaN.** Working through
  `c2RayToPlane_OneDimensional`, its result is always `<= 1` for finite inputs:
  `da < 0` returns `0`; `da*db > 0` returns exactly `1`; otherwise `db <= 0`, so
  `d = da - db >= da >= 0` and hence `da/d <= 1`. The only way `t <= 1.0f` can be
  false is `t` being NaN. A NaN-free random search never reaches this branch, so
  `err_08_aabb_no_plane_hit` injects NaN deliberately; it now reports reaching
  the branch 33 times. This is precisely the blind spot the error-surface table
  exists to catch.
* **Rows 11 and 14 assert a PARTIAL out-param write.** `c2RaytoCapsule` writes
  `out->n = c2Norm(cap_n)` and `out->t = 0` at lines 260-261 *before* any of its
  `return 0` paths, so on a miss the caller's `c2Raycast` is modified even though
  the function reports no hit. Every test starts the out-param from a
  recognisable `dirty()` bit pattern instead of zeros, so this is asserted rather
  than accidentally masked.
* **Row 26 (out-of-range enum) is checked with a NULL shape pointer too.** Since
  the `switch` matches nothing, `B` must never be dereferenced; passing
  `NULL` for both `B` and `bx` proves neither implementation dispatches. Values
  tested: `4,5,6,7,8,100,255,256,0x7FFF,INT_MAX,INT_MAX-1,-1,-2,-100,INT_MIN,
  INT_MIN+1`.
* **Rows 30 and 32 assert counter-intuitive C behaviour.** A NaN coordinate makes
  every `<`/`>` false, so an all-NaN `c2AABB` *overlaps* everything and an
  all-NaN point is *inside* every box. The Rust must match, which it does.
* **Rows 41 / 43 remain excluded as genuine UB** (NULL out-param, NULL shape
  pointer with a valid `typeB`). Row 42's in-bounds portion — `count` sweeping
  `0..=8`, the fixed array capacity — is covered by
  `err_42_poly_count_gt_8_bounded`.
