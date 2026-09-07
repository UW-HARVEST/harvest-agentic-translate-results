# ERRORS.md — Phase C error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. The library uses **no** error
macros, no `errno`, no `assert`, and no `NULL` checks; every rejection is an
early `return 0` / falsy `int` result, or an IEEE-754 special value produced by
an unguarded division. Every such distinct branch gets one row.

Grep basis:

```sh
grep -n 'return 0\|return 1\|return !\|return d2\|return da\|return 1.0f\|/ \|1.0f /' c_src/src/lib.c
```

| # | function | trigger (exact invalid input / condition) | expected C result | test | ✅ |
|---|----------|--------------------------------------------|-------------------|------|----|
| 1 | `c2RaytoCircle` | `disc = b*b - c < 0` (ray line misses the circle) | `return 0`, `*out` **left untouched** | `err_raytocircle_disc_negative` | ✅ |
| 2 | `c2RaytoCircle` | `t = -b - sqrtf(disc) < 0` (circle behind the ray origin) | `return 0`, `*out` untouched | `err_raytocircle_t_negative` | ✅ |
| 3 | `c2RaytoCircle` | `t > A.t` (hit beyond the ray length) | `return 0`, `*out` untouched | `err_raytocircle_t_beyond_len` | ✅ |
| 4 | `c2RaytoCircle` | `A.t` is NaN → `t <= A.t` false | `return 0`, `*out` untouched | `err_raytocircle_nan_t` | ✅ |
| 5 | `c2RaytoCircle` | `B.r` NaN / `A.p` NaN → `disc` NaN → `disc < 0` false, then `t<=A.t` false | `return 0` | `err_raytocircle_nan_inputs` | ✅ |
| 6 | `c2RaytoCircle` | `B.r == 0` and ray passes exactly through `B.p` (`disc == 0`, `t == -b`) | `return 1` iff `0 <= -b <= A.t` | `err_raytocircle_zero_radius` | ✅ |
| 7 | `c2AABBtoAABB` | `d0`: `B.max.x < A.min.x` | `return 0` | `err_aabbtoaabb_d0` | ✅ |
| 8 | `c2AABBtoAABB` | `d1`: `A.max.x < B.min.x` | `return 0` | `err_aabbtoaabb_d1` | ✅ |
| 9 | `c2AABBtoAABB` | `d2`: `B.max.y < A.min.y` | `return 0` | `err_aabbtoaabb_d2` | ✅ |
| 10 | `c2AABBtoAABB` | `d3`: `A.max.y < B.min.y` | `return 0` | `err_aabbtoaabb_d3` | ✅ |
| 11 | `c2AABBtoAABB` | any coordinate NaN → all four `<` false | `return 1` (**NaN "overlaps"**) | `err_aabbtoaabb_nan` | ✅ |
| 12 | `c2AABBtoAABB` | inverted box (`min > max`) | no validation; result is whatever the 4 compares say | `err_aabbtoaabb_inverted` | ✅ |
| 13 | `c2AABBtoPoint` | `d0`: `B.x < A.min.x` | `return 0` | `err_aabbtopoint_d0` | ✅ |
| 14 | `c2AABBtoPoint` | `d1`: `B.y < A.min.y` | `return 0` | `err_aabbtopoint_d1` | ✅ |
| 15 | `c2AABBtoPoint` | `d2`: `B.x > A.max.x` | `return 0` | `err_aabbtopoint_d2` | ✅ |
| 16 | `c2AABBtoPoint` | `d3`: `B.y > A.max.y` | `return 0` | `err_aabbtopoint_d3` | ✅ |
| 17 | `c2AABBtoPoint` | `B.x`/`B.y` NaN | `return 1` | `err_aabbtopoint_nan` | ✅ |
| 18 | `c2CircleToPoint` | `d2 >= A.r * A.r` (point on/outside the circle; note **strict** `<`, so a point exactly on the rim is rejected) | `return 0` | `err_circletopoint_outside_and_on_rim` | ✅ |
| 19 | `c2CircleToPoint` | `A.r < 0` → `r*r > 0`, so a negative radius still "contains" points | no validation; `return` per `d2 < r*r` | `err_circletopoint_negative_radius` | ✅ |
| 20 | `c2CircleToPoint` | `A.r` or `B` NaN → `d2 < r*r` false | `return 0` | `err_circletopoint_nan` | ✅ |
| 21 | `c2RaytoAABB` | `!c2AABBtoAABB(a_box, B)` — swept-segment box disjoint from `B` | `return 0`, `*out` untouched | `err_raytoaabb_sweep_box_disjoint` | ✅ |
| 22 | `c2RaytoAABB` | `d > 0` — separating-axis test on the segment normal fails | `return 0`, `*out` untouched | `err_raytoaabb_sat_reject` | ✅ |
| 23 | `c2RaytoAABB` | `hit == 0` (`t0..t3` all `> 1.0f`, i.e. every plane crossing beyond the segment) | `return 0`, `*out` untouched | `err_raytoaabb_no_plane_hit` | ✅ |
| 24 | `c2RaytoAABB` | `A.t == 0` (degenerate zero-length ray) → `p1 == p0`, `ab == 0`, `n == 0`, `d == -dot(abs_n,he)` | no rejection; `out->t = t*0 = 0` | `err_raytoaabb_zero_length_ray` | ✅ |
| 25 | `c2RaytoAABB` | `A.d` NaN (e.g. from a degenerate `c2Norm`) → `a_box` NaN → `c2AABBtoAABB` returns 1, `d` NaN, `d>0` false | falls through to the plane tests | `err_raytoaabb_nan_dir` | ✅ |
| 26 | `c2RayToPlane_OneDimensional` (via `c2RaytoAABB`) | `da < 0` | returns `0` (`t=0` ⇒ that plane is a "hit at 0") | `err_raytoplane_da_negative` | ✅ |
| 27 | `c2RayToPlane_OneDimensional` (via `c2RaytoAABB`) | `da * db > 0` (both on the same side) | returns `1.0f` (⇒ `hit == 1` because `1.0f <= 1.0f`) | `err_raytoplane_same_side` | ✅ |
| 28 | `c2RayToPlane_OneDimensional` (via `c2RaytoAABB`) | `da == db` ⇒ `d == 0` — **unguarded divide avoided** by the explicit `d != 0` check | returns `0`, *not* inf/NaN | `err_raytoplane_d_zero` | ✅ |
| 29 | `c2RaytoCapsule` | neither the `yAe.x*yAp.x < 0` nor the `min(|yAe.x|,|yAp.x|) < B.r` condition holds | `return 0`, but `*out` **has already been overwritten** with `n = normalize(b-a)`, `t = 0` | `err_raytocapsule_miss_still_writes_out` | ✅ |
| 30 | `c2RaytoCapsule` | ray origin inside the capsule slab (`c2AABBtoPoint(capsule_bb, yAp)`) | `return 1` with `t = 0`, `n = normalize(b-a)` | `err_raytocapsule_origin_in_slab` | ✅ |
| 31 | `c2RaytoCapsule` | ray origin inside end-cap circle `a` | `return 1`, `t = 0` | `err_raytocapsule_origin_in_cap_a` | ✅ |
| 32 | `c2RaytoCapsule` | ray origin inside end-cap circle `b` | `return 1`, `t = 0` | `err_raytocapsule_origin_in_cap_b` | ✅ |
| 33 | `c2RaytoCapsule` | `B.a == B.b` (degenerate capsule) → `c2Norm(0,0)` = `0 * (1/0)` = **NaN**, `M` all NaN, `yBb`/`yAp`/`yAd` NaN | no validation; `out->n` = `(NaN,NaN)`, then NaN-driven branches | `err_raytocapsule_degenerate_ab` | ✅ |
| 34 | `c2RaytoCapsule` | `B.r == 0` → `capsule_bb = {(0,0),(0,yBb.y)}`; `min(|..|,|..|) < 0` false | usually `return 0` | `err_raytocapsule_zero_radius` | ✅ |
| 35 | `c2RaytoCapsule` | `B.r < 0` (negative radius) | no validation; `capsule_bb.min.x = -r > 0 = max.x` (inverted) | `err_raytocapsule_negative_radius` | ✅ |
| 36 | `c2RaytoCapsule` | `d = yAe.x - yAp.x == 0` in the else-branch → **unguarded division** `(c - yAp.x)/d` → ±inf or NaN | propagates ±inf/NaN into `y`, then `y<=0` / `y>=yBb.y` decide | `err_raytocapsule_div_by_zero` | ✅ |
| 37 | `c2RaytoCapsule` | `y >= yBb.y` → delegates to `c2RaytoCircle(A, Cb, out)`, whose result may be `0` | `return` the inner `0`, `*out` left as the pre-written `n`/`t=0` | `err_raytocapsule_delegate_returns_zero` | ✅ |
| 38 | `c2CastRay` | `typeB` = `3` (one past the last enumerator) | `switch` has no `default`: control **falls off the end of a non-void function** (UB). GCC `-O0` emits `leave; ret` without touching `%eax`, so the returned value is the indeterminate incoming `%eax`. `*out` untouched. | `err_castray_out_of_range_enum` (asserts *`*out` untouched*; documents that the return value is UB and therefore not compared) | ✅ |
| 39 | `c2CastRay` | `typeB` = `-1`, `INT_MIN`, `INT_MAX`, `0x7fffffff` (arbitrary out-of-range enum ints across FFI) | same UB fall-through, `*out` untouched | `err_castray_out_of_range_enum` | ✅ |
| 40 | `c2CastRay` | `typeB` = `0/1/2` but `B == NULL` | unguarded deref → SIGSEGV in **both** libraries | `err_castray_null_shape` (documented; not executed in-process) | ✅ (documented) |
| 41 | any `c2Rayto*` / `spec_ray` | `out == NULL` **and** the function takes a hit path | unguarded write → SIGSEGV in **both** libraries. `c2RaytoCapsule` faults *unconditionally* (it writes `out` before any test); `c2RaytoCircle`/`c2RaytoAABB` fault only on the hit paths, i.e. a *miss* with `out==NULL` returns 0 safely. | `err_null_out_miss_paths_are_safe` (verifies the safe subset differentially in-process; the faulting subset is verified in a forked child) | ✅ |
| 42 | `c2Div` | `b == 0` → `1.0f/0.0f = +inf`, `a * inf` → `±inf` or `NaN` (for `a == 0`) | no rejection; inf/NaN propagate | `err_div_by_zero` | ✅ |
| 43 | `c2Div` | `b == -0.0` → `1.0f/-0.0f = -inf` | `-inf` propagates (sign matters) | `err_div_by_zero` | ✅ |
| 44 | `c2Norm` | `a == (0,0)` → `c2Len == 0` → `c2Div` by 0 → `(NaN, NaN)` | no rejection; `(NaN,NaN)` | `err_norm_zero_vector` | ✅ |
| 45 | `c2Norm` | `a` contains ±inf → `c2Len == inf`, `1/inf == 0`, `inf*0` = NaN | `(NaN, …)` | `err_norm_inf_vector` | ✅ |
| 46 | `c2Len` | `c2Dot(a,a)` overflows to `+inf` (e.g. `a.x = 1e30`) | `sqrtf(inf) = inf` | `err_len_overflow` | ✅ |
| 47 | `spec_ray` | `mp == ray.p` → `c2Sub` = `(0,0)` → `c2Norm` = `(NaN,NaN)` → `ray.t` NaN → `c2RaytoCircle` `disc` NaN | `return 0`, `*cast` untouched | `err_spec_ray_mp_equals_ray_origin` | ✅ |
| 48 | `spec_ray` | `c_r < 0` (negative radius) → `c = dot(m,m) - r*r` same as `+r` | no validation | `err_spec_ray_negative_radius` | ✅ |
| 49 | `spec_ray` | `c_r == 0` | tangent-only hit | `err_spec_ray_zero_radius` | ✅ |
| 50 | `spec_ray` | any float argument NaN / ±inf | no validation; NaN/inf propagate, generally `return 0` | `err_spec_ray_nan_inf_args` | ✅ |
| 51 | `spec_ray` | `ray.t` computed negative (mouse point *behind* the ray origin — impossible for `c2Norm`'d `d`, but reachable with inf/NaN inputs) | `t <= A.t` false ⇒ `return 0` | `err_spec_ray_nan_inf_args` | ✅ |

## Notes on rows 38–41 (UB / signal rows)

* Rows 38–39: the C code has **no** `default:` label and **no** trailing
  `return`, so for `typeB ∉ {0,1,2}` the C function's return value is
  *indeterminate* (it is whatever `%eax` happens to hold). The test therefore
  asserts the only well-defined, observable property — that neither library
  writes to `*out` — and does **not** compare the garbage return value, since
  the C library does not even agree with itself between `-O0` and `-O2` builds
  there. The Rust translation returns `0`.
* Rows 40–41: a `NULL` dereference cannot be compared in-process without
  killing the test harness. The *safe* subset (miss paths, where the C code
  never touches `out`) is compared differentially in-process; the faulting
  subset is exercised in a `fork()`ed child and both libraries are asserted to
  die with the same signal.
