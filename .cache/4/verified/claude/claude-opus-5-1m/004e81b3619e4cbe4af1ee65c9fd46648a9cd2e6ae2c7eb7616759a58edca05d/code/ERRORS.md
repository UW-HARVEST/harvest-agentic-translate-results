# ERRORS.md — ERROR-SURFACE TABLE (Phase A, gates Phase C)

Every distinct way `c_src/src/lib.c` rejects / errors on input. Derived
mechanically by grepping `lib.c` for every `return 0`, every `return` of a
sentinel / early-out, every explicit comparison guard, every null check and
every division guard. There are no `assert`s, no error enums and no
`RETURN_ERROR` macros in this library — the entire error surface is
"return `0` (no hit)", "return a guard constant", or "produce a non-finite
float from an unguarded division".

Line numbers refer to `c_src/src/lib.c`.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| 1  | `c2RaytoCircle` (L117) | `disc = b*b - c < 0` — ray line misses the circle entirely | returns `0`; `*out` **left untouched** |
| 2  | `c2RaytoCircle` (L120) | `t = -b - sqrtf(disc) < 0` — circle is behind the ray origin | returns `0`; `*out` untouched |
| 3  | `c2RaytoCircle` (L120) | `t > A.t` — hit exists but is beyond the ray's max distance | returns `0`; `*out` untouched |
| 4  | `c2RaytoCircle` (L117) | `disc` is `NaN` (e.g. `A.p`/`B.p`/`B.r` contains `NaN` or `inf-inf`) — `disc < 0` is **false**, `t` becomes `NaN`, `t>=0` is **false** | returns `0`; `*out` untouched |
| 5  | `c2RaytoCircle` (L123) | hit accepted but impact point equals circle centre (`B.r == 0` and ray passes exactly through `B.p`) → `c2Norm` divides by `0` | returns `1` with `out->n` = `NaN`/`±inf` (`0/0`) |
| 6  | `c2RaytoCircle` (L114) | `B.r < 0` (negative radius) — `B.r*B.r` is positive so it behaves like `|B.r|`; no rejection | returns `0` or `1` exactly as for `+B.r` |
| 7  | `c2AABBtoAABB` (L130) | `B.max.x < A.min.x` (separated on `-x`) | returns `0` |
| 8  | `c2AABBtoAABB` (L131) | `A.max.x < B.min.x` (separated on `+x`) | returns `0` |
| 9  | `c2AABBtoAABB` (L132) | `B.max.y < A.min.y` (separated on `-y`) | returns `0` |
| 10 | `c2AABBtoAABB` (L133) | `A.max.y < B.min.y` (separated on `+y`) | returns `0` |
| 11 | `c2AABBtoAABB` (L130-134) | any coordinate is `NaN` — all four `<` are false | returns `1` (**overlap**) |
| 12 | `c2RaytoAABB` (L162) | swept-ray bounding box does not overlap `B` (`!c2AABBtoAABB`) | returns `0`; `*out` untouched |
| 13 | `c2RaytoAABB` (L173) | `d > 0` — the ray's supporting line separates from the box (SAT on the ray normal) | returns `0`; `*out` untouched |
| 14 | `c2RaytoAABB` (L191) | `hit == 0`, i.e. all four `t0..t3 > 1.0` | returns `0`; `*out` untouched |
| 15 | `c2RayToPlane_OneDimensional` (L143) | `da < 0` — start point already behind the plane | returns `0.0f` (guard constant, not an error return) |
| 16 | `c2RayToPlane_OneDimensional` (L149) | `da == db` so `d = da-db == 0` — division-by-zero guard | returns `0.0f` instead of dividing |
| 17 | `c2RaytoAABB` (L156) | `B` inverted (`B.min > B.max`) — no validation at all; `half_extents` become negative | no rejection; returns whatever the arithmetic yields (must match C bit-for-bit) |
| 18 | `c2RaytoAABB` (L158) | `A.t == 0` (degenerate zero-length ray) → `p0 == p1`, `ab == 0`, `n == 0`, `d == -dot(0,he)` | no rejection; `out->t` = `t* * 0` |
| 19 | `c2AABBtoPoint` (L235) | `B.x < A.min.x` | returns `0` |
| 20 | `c2AABBtoPoint` (L236) | `B.y < A.min.y` | returns `0` |
| 21 | `c2AABBtoPoint` (L237) | `B.x > A.max.x` | returns `0` |
| 22 | `c2AABBtoPoint` (L238) | `B.y > A.max.y` | returns `0` |
| 23 | `c2AABBtoPoint` (L235-239) | any coordinate `NaN` — all four comparisons false | returns `1` |
| 24 | `c2CircleToPoint` (L245) | `d2 >= A.r*A.r` — point on or outside the circle (note: boundary is **exclusive**) | returns `0` |
| 25 | `c2CircleToPoint` (L245) | `A.r == 0` — `d2 < 0` impossible | always returns `0` |
| 26 | `c2CircleToPoint` (L245) | `NaN` in `A.p`/`B`/`A.r` — `<` false | returns `0` |
| 27 | `c2RaytoCapsule` (L250) | `B.a == B.b` (degenerate capsule) → `c2Norm(0)` = `0/0` → `M.y` is `NaN`; everything downstream is `NaN` | no rejection; `out->n` is set to `NaN` **before** any branch, then the `NaN` comparisons drive the control flow; must match C exactly |
| 28 | `c2RaytoCapsule` (L262) | `c2AABBtoPoint(capsule_bb, yAp)` true — ray origin already inside the capsule's slab | returns `1` **early** with `out->t = 0` and `out->n = c2Norm(B.b-B.a)` |
| 29 | `c2RaytoCapsule` (L271) | ray origin inside the `B.a` end-cap circle | returns `1` early, `out->t = 0`, `out->n = c2Norm(cap_n)` |
| 30 | `c2RaytoCapsule` (L273) | ray origin inside the `B.b` end-cap circle | returns `1` early, `out->t = 0`, `out->n = c2Norm(cap_n)` |
| 31 | `c2RaytoCapsule` (L308) | outer `if` is false (ray stays on one side and never gets within `B.r` in `x`) | returns `0`, but `*out` **has already been overwritten** with `t=0`, `n=c2Norm(cap_n)` |
| 32 | `c2RaytoCapsule` (L295) | `d = yAe.x - yAp.x == 0` — **unguarded** division `(c - yAp.x)/d` | no rejection; `t` becomes `±inf`/`NaN`, propagates into `y` and the `y<=0` / `y>=yBb.y` branches |
| 33 | `c2RaytoCapsule` (L289/291/298/300) | delegates to `c2RaytoCircle`, which can itself return `0` (rows 1-3) | returns whatever `c2RaytoCircle` returns, `*out` possibly left with the pre-set `t=0`,`n=norm(cap_n)` |
| 34 | `c2RaytoCapsule` (L258) | `B.r < 0` — `capsule_bb.min.x = -B.r > 0 = ...max.x`, i.e. inverted bb | no rejection; `c2AABBtoPoint` on inverted box is always `0` |
| 35 | `c2RaytoPoly` (L347) | `den == 0 && num < 0` — ray parallel to a face plane and outside it | returns `0`; `*out` untouched |
| 36 | `c2RaytoPoly` (L356) | `hi < lo` — the interval collapsed, ray misses the polygon | returns `0`; `*out` untouched |
| 37 | `c2RaytoPoly` (L359) | `index == ~0` (`-1`) after the loop — no face produced an entering hit | returns `0`; `*out` untouched |
| 38 | `c2RaytoPoly` (L344) | `B->count <= 0` (zero or negative vertex count) — loop body never runs | returns `0`; `*out` untouched |
| 39 | `c2RaytoPoly` (L338) | `bx_ptr == NULL` — **null check**, substitutes `c2xIdentity()` | no error; behaves as identity transform |
| 40 | `c2RaytoPoly` (L338) | `bx_ptr != NULL` with a non-unit `c2r` (`c*c+s*s != 1`) — no validation | no rejection; scales/skews the result |
| 41 | `c2RaytoPoly` (L344-345) | `B->count > 8` — reads **past** `verts[8]`/`norms[8]`; C performs the out-of-bounds read with no check | no rejection; result determined by the adjacent bytes (Rust must read the same bytes) |
| 42 | `c2RaytoPoly` (L350/353) | `num`/`den`/`lo`/`hi` become `NaN` (e.g. `NaN` in `A.p` or in a normal) — `den == 0`, `den < 0`, `den > 0`, `hi < lo` are all false | falls through the loop, `index` stays `-1` | returns `0` |
| 43 | `c2CastRay` (L368) | `typeB` is not one of `0..3` (a C enum accepts any `int`, e.g. `4`, `-1`, `0x7fffffff`) — `switch` has no `default`, control reaches L378 | returns `0`; `*out` untouched, `B` never dereferenced |
| 44 | `c2CastRay` (L370-376) | any of the four valid `typeB` values with a `B` whose actual layout is a different shape — no type tag validation | reinterprets the bytes; must match C |
| 45 | `c2CastRay` (L376) | `typeB == C2_TYPE_POLY` and `bx == NULL` | forwarded to `c2RaytoPoly`, row 39 applies |
| 46 | `c2CastRay` (L370-374) | `typeB != C2_TYPE_POLY` — `bx` is **ignored entirely** (even if non-NULL / garbage) | `bx` has no effect on the result |
| 47 | `c2Div` (L83) | `b == 0` — **unguarded** `1.0f/b` | returns `±inf` components (or `NaN` for `0*inf`), no rejection |
| 48 | `c2Norm` (L87) | `a == (0,0)` — `c2Len` is `0`, `c2Div` divides by `0` | returns `(NaN, NaN)` (`0 * inf`) |
| 49 | `c2Norm` (L87) | `a` contains `inf` — `c2Len` is `inf`, `1/inf == 0` | returns `(NaN, NaN)` or `(0,0)` depending on the component |
| 50 | `c2Len` (L61) | `c2Dot(a,a)` overflows to `+inf` | returns `+inf` (no rejection) |
| 51 | `c2Len` (L61) | `a` contains `NaN` | `sqrtf(NaN)` = `NaN` |
| 52 | `c2RaytoCircle` / `c2RaytoAABB` / `c2RaytoPoly` | `out == NULL` **and** the function takes an early-`return 0` path (rows 1-4, 12-14, 35-38) — `out` is never dereferenced on those paths | returns `0` without a fault (testable) |
| 53 | `c2RaytoCapsule` | `out == NULL` — dereferenced **unconditionally** at L260 before any branch | segfault in C; documented as UB, **not** differentially tested |
| 54 | `c2RaytoPoly` / `c2CastRay(POLY)` | `B == NULL` — `B->count` dereferenced unconditionally at L344 | segfault in C; documented as UB, **not** differentially tested |
| 55 | `poly_ray` (L398-399) | the hard-coded geometry: **both** casts miss (measured from the C `.so`), so the bitmask is `0` and **neither** out-param is written | returns `0`; `*cast1` and `*cast2` both left completely untouched |

## Phase C checklist

Rows 53 and 54 are the only rows that are **not** differentially tested: they
are unconditional null dereferences in the C source, i.e. a hard segfault that
cannot be observed as a "same error code". They are recorded here for
completeness of the error surface.

Every other row is covered by `tests/errors.rs` (see the `ERRORS.md row N`
comments) and by the boundary sweeps in `tests/raycast_*.rs` /
`tests/cast_ray.rs`.

| row | test | status |
|-----|------|--------|
| 1  | `err_row01_circle_disc_negative`              | [x] |
| 2  | `err_row02_circle_behind_origin`              | [x] |
| 3  | `err_row03_circle_beyond_max_t`               | [x] |
| 4  | `err_row04_circle_nan_inputs`                 | [x] |
| 5  | `err_row05_circle_zero_radius_norm_div0`      | [x] |
| 6  | `err_row06_circle_negative_radius`            | [x] |
| 7  | `err_row07_aabbtoaabb_sep_neg_x`              | [x] |
| 8  | `err_row08_aabbtoaabb_sep_pos_x`              | [x] |
| 9  | `err_row09_aabbtoaabb_sep_neg_y`              | [x] |
| 10 | `err_row10_aabbtoaabb_sep_pos_y`              | [x] |
| 11 | `err_row11_aabbtoaabb_nan`                    | [x] |
| 12 | `err_row12_raytoaabb_bb_miss`                 | [x] |
| 13 | `err_row13_raytoaabb_sat_separated`           | [x] |
| 14 | `err_row14_raytoaabb_no_hit_flags`            | [x] |
| 15 | `err_row15_raytoplane1d_da_negative`          | [x] |
| 16 | `err_row16_raytoplane1d_div_guard`            | [x] |
| 17 | `err_row17_raytoaabb_inverted_box`            | [x] |
| 18 | `err_row18_raytoaabb_zero_length_ray`         | [x] |
| 19 | `err_row19_aabbtopoint_below_min_x`           | [x] |
| 20 | `err_row20_aabbtopoint_below_min_y`           | [x] |
| 21 | `err_row21_aabbtopoint_above_max_x`           | [x] |
| 22 | `err_row22_aabbtopoint_above_max_y`           | [x] |
| 23 | `err_row23_aabbtopoint_nan`                   | [x] |
| 24 | `err_row24_circletopoint_outside_exclusive`   | [x] |
| 25 | `err_row25_circletopoint_zero_radius`         | [x] |
| 26 | `err_row26_circletopoint_nan`                 | [x] |
| 27 | `err_row27_capsule_degenerate_ab`             | [x] |
| 28 | `err_row28_capsule_origin_in_slab`            | [x] |
| 29 | `err_row29_capsule_origin_in_cap_a`           | [x] |
| 30 | `err_row30_capsule_origin_in_cap_b`           | [x] |
| 31 | `err_row31_capsule_miss_but_out_written`      | [x] |
| 32 | `err_row32_capsule_div_by_zero_dx`            | [x] |
| 33 | `err_row33_capsule_delegates_circle_miss`     | [x] |
| 34 | `err_row34_capsule_negative_radius`           | [x] |
| 35 | `err_row35_poly_parallel_outside`             | [x] |
| 36 | `err_row36_poly_hi_lt_lo`                     | [x] |
| 37 | `err_row37_poly_index_unset`                  | [x] |
| 38 | `err_row38_poly_count_zero_and_negative`      | [x] |
| 39 | `err_row39_poly_null_bx_is_identity`          | [x] |
| 40 | `err_row40_poly_non_unit_rotation`            | [x] |
| 41 | `err_row41_poly_count_gt_8_oob_read`          | [x] |
| 42 | `err_row42_poly_nan_inputs`                   | [x] |
| 43 | `err_row43_castray_invalid_type_enum`         | [x] |
| 44 | `err_row44_castray_type_layout_mismatch`      | [x] |
| 45 | `err_row45_castray_poly_null_bx`              | [x] |
| 46 | `err_row46_castray_bx_ignored_for_non_poly`   | [x] |
| 47 | `err_row47_div_by_zero`                       | [x] |
| 48 | `err_row48_norm_zero_vector`                  | [x] |
| 49 | `err_row49_norm_inf_vector`                   | [x] |
| 50 | `err_row50_len_overflow_to_inf`               | [x] |
| 51 | `err_row51_len_nan`                           | [x] |
| 52 | `err_row52_null_out_on_early_return`          | [x] |
| 53 | (UB — unconditional null deref, not tested)   | n/a |
| 54 | (UB — unconditional null deref, not tested)   | n/a |
| 55 | `err_row55_poly_ray_bitmask`                  | [x] |

## How the rows were verified

Every row is a *differential* test: it builds the exact rejection condition,
calls the C `.so` and the Rust `cdylib` through `dlopen`/`dlsym`, and asserts

1. the **same** `int` sentinel is returned (not merely "both failed"), and
2. the `c2Raycast` out-parameter is **byte-identical**, including the case where
   the C leaves it untouched — every call is made with the out-buffer
   pre-filled with a poison bit pattern, so "did the callee write?" is itself
   part of the comparison. Rows 31 and 33 rely on this: `c2RaytoCapsule`
   overwrites `*out` *before* returning `0`, and the Rust must do the same.

Most rows additionally run a fixed-seed randomized sweep (2048-8192 cases) over
the region of input space that keeps the row's condition true, so the row is not
signed off on a single hand-picked value.

```
cargo test --no-default-features --test errors     # 53 tests, all rows
./run_all.sh                                       # every configuration
```

## The one relaxation, and its proof

`tests/common/mod.rs::feq` compares floats bit-for-bit with a single documented
exception: if **both** results are NaN, the *payload/sign* of the NaN is not
required to match. On x86, an SSE operation with two NaN operands returns the
*destination* register's NaN, so the surviving payload is chosen by the
compiler's register allocator. GCC at `-O0` emits, for `a.x*b.x + a.y*b.y`:

```
mulss %xmm0,%xmm1      ; dst = a.x  -> a.x's NaN wins
mulss %xmm2,%xmm0      ; dst = b.y  -> b.y's NaN wins  (operands swapped!)
addss %xmm1,%xmm0      ; dst = the SECOND product      -> its NaN wins
```

LLVM picks a different order. Neither IEEE-754, nor C, nor Rust specifies which
payload survives, so requiring identical NaN bits would assert on GCC's register
allocator rather than on the translation.

`tests/nan_payload.rs` proves the relaxation hides nothing:

| test | measurement |
|------|-------------|
| `finite_inputs_are_bit_identical_with_no_relaxation` | 579,998 floats from finite inputs — **100% bit-identical**, no relaxation used |
| `nan_payload_is_the_only_difference` | 1,060,000 floats from full-spectrum inputs (`±0.0`, subnormals, `±inf`, NaN, random bit patterns) — 1,054,042 bit-identical, 5,958 (0.56%) differ, and **every single one has both sides NaN**. A case where one side is NaN and the other is not, or where two non-NaN values differ, fails the test. |

Integer return values (`int`) and NaN-ness itself are always compared exactly,
with no relaxation.

## Robustness against the C compiler's flags

The whole suite was additionally re-run against the C library built at `-O1`,
`-O2` and `-O3` (out-of-source, `c_src/` untouched, via the `DIFFTEST_C_SO`
environment override). All 139 tests pass at every optimisation level, which
confirms the translation does not depend on a particular GCC codegen choice.
