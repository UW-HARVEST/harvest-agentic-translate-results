# ERRORS.md — error / rejection surface of the C library

Derived mechanically from `c_src/src/lib.c` (grep for `return 0`, `return 1`,
`return NULL`, `assert`, every `if`, every ternary, every comparison against a
bound). Findings up front:

* There are **no** `assert`s, **no** `RETURN_ERROR`-style macros, **no** error
  enums, **no** `NULL` checks and **no** `errno` use anywhere in the library.
* The only rejection channel is the `int` return value of the predicate /
  raycast functions: **`0` = no hit / no overlap, `1` = hit / overlap**.
* Every pointer parameter (`c2Raycast *out`, `const void *B`) is dereferenced
  unconditionally on the paths that use it → a null/invalid pointer is
  undefined behaviour in C and must be UB-for-UB in Rust (SIGSEGV), not a
  graceful error.
* Out-of-range `C2_TYPE` in `c2CastRay` falls off the end of a non-`void`
  function (no `default:` label) → undefined return value.

`[x]` = a differential test constructs that exact condition, calls **both**
`.so`s and asserts the **same** return value *and* the same bytes in the
`c2Raycast` out-parameter (pre-filled with the sentinel
`t=0xDEADBEEF, n=(0xCAFEBABE, 0x12345678)` so that "did not write" is
distinguishable from "wrote something"), *and* asserts that the C really
produced the documented result so the row cannot silently rot.

| #  | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|----|----------|------------------------------------------|-------------------|------|-----|
| 1  | `c2RaytoCircle` (`lib.c:100-101`) | `disc = b*b - c < 0` — ray line misses the circle | `0`, `*out` untouched | `err01_circle_disc_negative` | [x] |
| 2  | `c2RaytoCircle` (`lib.c:103,109`) | `t = -b - sqrtf(disc) < 0` — circle behind the origin / origin inside the circle | `0`, `*out` untouched | `err02_circle_t_negative` | [x] |
| 3  | `c2RaytoCircle` (`lib.c:103,109`) | `t > A.t` (probed at `A.t = t`, `nextafter(t,-inf)`, `t/2`; the bound is inclusive) | `0` (and `1` exactly at `A.t == t`) | `err03_circle_t_beyond_ray_length` | [x] |
| 4  | `c2RaytoCircle` (`lib.c:100`) | `disc` is `NaN` (NaN in `A.p`/`A.d`/`B.p`/`B.r`): `disc < 0` false ⇒ no early-out, then `t >= 0` false | `0`, `*out` untouched | `err04_circle_disc_nan` | [x] |
| 5  | `c2RaytoCircle` (`lib.c:97`) | `B.r < 0` — `r*r` is positive, so it behaves exactly like `+|r|`; no validation | same as `+r` (asserted bit-for-bit) | `err05_circle_negative_radius_behaves_like_abs` | [x] |
| 6  | `c2RaytoCircle` (`lib.c:103`) | `A.t < 0` / `A.t == -0.0` ⇒ `t <= A.t` false for every `t >= 0` | `0` | `err06_circle_negative_ray_length` | [x] |
| 7  | `c2AABBtoAABB` (`lib.c:113,117`) | `d0`: `B.max.x < A.min.x` | `0` | `err07_10_aabbtoaabb_each_separating_axis` | [x] |
| 8  | `c2AABBtoAABB` (`lib.c:114,117`) | `d1`: `A.max.x < B.min.x` | `0` | `err07_10_aabbtoaabb_each_separating_axis` | [x] |
| 9  | `c2AABBtoAABB` (`lib.c:115,117`) | `d2`: `B.max.y < A.min.y` | `0` | `err07_10_aabbtoaabb_each_separating_axis` | [x] |
| 10 | `c2AABBtoAABB` (`lib.c:116,117`) | `d3`: `A.max.y < B.min.y` | `0` | `err07_10_aabbtoaabb_each_separating_axis` | [x] |
| 11 | `c2AABBtoAABB` (`lib.c:113-117`) | any `NaN` coordinate ⇒ all four `<` false | **`1`** (reports overlap), asserted | `err11_aabbtoaabb_nan_reports_overlap` | [x] |
| 12 | `c2RaytoAABB` (`lib.c:145-146`) | swept-ray bbox does not overlap `B` | `0`, `*out` untouched | `err12_raytoaabb_bbox_reject` | [x] |
| 13 | `c2RaytoAABB` (`lib.c:152-157`) | separating-axis test `d > 0` (thin diagonal just outside a corner) | `0`, `*out` untouched | `err13_raytoaabb_separating_axis_reject` | [x] |
| 14 | `c2RaytoAABB` (`lib.c:174,194-195`) | `hit == 0`, i.e. all of `t0..t3 > 1.0`; reachable with an all-`NaN` box (bbox test passes per row 11, `d` is NaN so `d > 0` is false, every ratio is NaN) | `0`, `*out` untouched | `err14_raytoaabb_no_plane_hit` | [x] |
| 15 | `c2RayToPlane_OneDimensional` (`lib.c:126-127`) | `da < 0` (origin on the inside of that plane) ⇒ contributes `t = 0` | not a rejection on its own | `err15_18_raytoplane_onedimensional_branches` (13168 hits) | [x] |
| 16 | `c2RayToPlane_OneDimensional` (`lib.c:128-129`) | `da*db > 0` (both endpoints on the same side, incl. the zero-length sweep) ⇒ `t = 1.0f`, still `hit` | not a rejection | `err15_18_…` (824 hits) | [x] |
| 17 | `c2RayToPlane_OneDimensional` (`lib.c:131-135`) | `d = da - db == 0`; **only** reachable when `da == db == 0`, i.e. the origin lies exactly on the plane and the ray is parallel to it (for `da == db != 0` the `da*db > 0` branch fires first) ⇒ `t = 0` | not a rejection | `err15_18_…` (1600 hits) | [x] |
| 18 | `c2RayToPlane_OneDimensional` (`lib.c:133`) | `da / d` not `<= 1.0` ⇒ that axis does not vote. For finite inputs `da >= 0 && da*db <= 0` implies `da - db >= da`, so the ratio is always `<= 1`: this branch is reachable **only through NaN** | that axis' `hitN = 0` | `err15_18_…` (800 hits, NaN plane) | [x] |
| 19 | `c2AABBtoPoint` (`lib.c:218,222`) | `B.x < A.min.x` | `0` | `err19_22_aabbtopoint_each_rejection` | [x] |
| 20 | `c2AABBtoPoint` (`lib.c:219,222`) | `B.y < A.min.y` | `0` | `err19_22_aabbtopoint_each_rejection` | [x] |
| 21 | `c2AABBtoPoint` (`lib.c:220,222`) | `B.x > A.max.x` | `0` | `err19_22_aabbtopoint_each_rejection` | [x] |
| 22 | `c2AABBtoPoint` (`lib.c:221,222`) | `B.y > A.max.y` | `0` | `err19_22_aabbtopoint_each_rejection` | [x] |
| 23 | `c2AABBtoPoint` (`lib.c:218-222`) | `NaN` in the point or the box ⇒ no comparison true | **`1`** (reports inside), asserted | `err23_aabbtopoint_nan_reports_inside` | [x] |
| 24 | `c2AABBtoPoint` (`lib.c:217-222`) | inverted box (`min > max`) | `0` for every finite point (measured: 0 of 1200 reported inside) | `err24_aabbtopoint_inverted_box` | [x] |
| 25 | `c2CircleToPoint` (`lib.c:228`) | `d2 >= A.r*A.r` — the comparison is **strict** `<`, so a point exactly on the rim is a miss (tested with dyadic coordinates so `d2 == r*r` bit-exactly) | `0` | `err25_circletopoint_on_rim_is_a_miss` | [x] |
| 26 | `c2CircleToPoint` (`lib.c:228`) | `A.r == 0` / `-0.0` ⇒ `d2 < 0` impossible | always `0` | `err26_circletopoint_zero_radius_never_hits` | [x] |
| 27 | `c2CircleToPoint` (`lib.c:228`) | `NaN` in the point / centre / radius ⇒ `NaN < r*r` false | `0` | `err27_circletopoint_nan_is_a_miss` | [x] |
| 28 | `c2RaytoCapsule` (`lib.c:260-264,291`) | neither `yAe.x*yAp.x < 0` nor `min(\|yAe.x\|,\|yAp.x\|) < B.r` (ray parallel to the axis, outside the slab) | `0` — but `*out` **has already been overwritten** at `lib.c:243-244` with `t = +0.0`, `n = c2Norm(b-a)` (both asserted) | `err28_capsule_outside_slab` | [x] |
| 29 | `c2RaytoCapsule` (`lib.c:233`) | degenerate capsule `B.a == B.b` ⇒ `c2Norm(0,0)` = `0 * (1/0)` = NaN, cascading into `M`, `yBb`, `yAp` | `1` with `out->n = (NaN,NaN)`, `out->t = +0.0` (asserted) | `err29_capsule_degenerate_a_equals_b` | [x] |
| 30 | `c2RaytoCapsule` (`lib.c:242,245`) | inverted `capsule_bb`. `yBb.y = dot(norm(b-a), b-a) = \|b-a\| >= 0` always (measured: 0 of 1200 negative), so the box can only be inverted on **x**, via `B.r < 0` (`min.x = -r > 0 = …`) | no rejection of its own; `c2AABBtoPoint` on an inverted box (row 24) | `err30_capsule_inverted_slab_box` | [x] |
| 31 | `c2RaytoCapsule` (`lib.c:278`) | `d = yAe.x - yAp.x` is `±inf` (`A.t = inf`) ⇒ `t = ∓0`, `y = NaN` ⇒ neither `y <= 0` nor `y >= yBb.y` ⇒ the flat-side branch with `out->t = NaN` | `1`, `out->t` is NaN (asserted) | `err31_capsule_infinite_denominator` | [x] |
| 32 | `c2RaytoCapsule` (`lib.c:270-274`) | `\|yAp.x\| < B.r` delegates to `c2RaytoCircle(Ca/Cb)`, which itself rejects | `0` with `*out` = the pre-written `(t=+0.0, n=norm(b-a))` (asserted) | `err32_capsule_near_axis_delegation_misses` | [x] |
| 33 | `c2RaytoCapsule` (`lib.c:280-283`) | delegation via `y <= 0` / `y >= yBb.y` to `c2RaytoCircle` misses | `0`, `*out` = pre-written values (asserted) | `err33_capsule_cross_delegation_misses` | [x] |
| 34 | `c2CastRay` (`lib.c:294-304`) | `typeB` outside `{0,1,2}` — tested with `3,4,5,255,256,-1,-2,-1000,INT_MAX,INT_MIN`. No `default:` label, so control **falls off the end of a non-`void` function**. | **undefined return value**: gcc `-O0` emits `leave; ret`, so the caller sees whatever was left in `EAX` at the call site (measured: 610/610 calls returned caller-dependent garbage, identical for every invalid value). Well-defined part: nothing is dereferenced, `*out` stays untouched, no crash. The Rust returns the source's dead `return 0;`. The test asserts the well-defined part (both leave `*out` untouched, neither crashes) and records the C's garbage; it deliberately does **not** assert equality of an undefined value. | `err34_castray_out_of_range_enum_values` | [x] |
| 35 | `c2CastRay` (`lib.c:297-301`) | `B` pointing at a *differently shaped* object — the same 32-byte buffer is dispatched as circle (12 B), AABB (16 B) and capsule (20 B) | both implementations must interpret the identical bytes identically | `err35_castray_shape_reinterpretation` | [x] |
| 36 | `c2RaytoCircle` / `c2RaytoAABB` / `c2RaytoCapsule` / `c2CastRay` / `spec_ray` | `out == NULL` | UB. `c2RaytoCircle`/`c2RaytoAABB` store only on the **hit** path ⇒ a *miss* exits cleanly (`code 0`), a *hit* faults; `c2RaytoCapsule` stores unconditionally ⇒ always faults. Measured **SIGSEGV (11)** in C **and** Rust for every faulting case, clean exit for every non-faulting case, in a re-exec'd child process. | `phase_c_null::rows36_37_43_null_pointer_behaviour_matches` (cases `circle_out_null_{miss,hit}`, `aabb_out_null_{miss,hit}`, `capsule_out_null`, `castray_out_null_{miss,hit}`) | [x] |
| 37 | `c2CastRay` | `B == NULL` with a valid `typeB` (unconditional load) — and `B == out == NULL` with an *invalid* `typeB`, which dereferences nothing | SIGSEGV (11) in both; clean exit in both for the invalid-`typeB` variant | same test (cases `castray_b_null`, `castray_b_and_out_null_invalid_type`) | [x] |
| 38 | `c2Div` / `c2Norm` (`lib.c:66,70`) | `b == ±0.0` ⇒ `1.0f/0 = ±inf`, `0*inf = NaN`; `b = ±inf` ⇒ `1/inf = ±0`; `b` denormal ⇒ `1/b = inf`; zero-length vector into `c2Norm` | `(NaN,NaN)` / `(±inf,±inf)` / `(±0,±0)`; no error. `c2Norm(0,0)` is NaN (asserted) | `err38_div_and_norm_by_zero` | [x] |
| 39 | `c2Len` (`lib.c:44`) | `c2Dot(a,a)` overflows to `+inf` ⇒ `sqrtf(+inf)` | `+inf` (asserted) | `err39_41_len_overflow_and_nan` | [x] |
| 40 | `c2Len` (`lib.c:44`) | `NaN` component ⇒ glibc's `sqrtf` wrapper takes `isless(x,0) == false` ⇒ `SQRTSS` ⇒ the NaN quieted | bit-exact NaN match required | `err39_41_len_overflow_and_nan`, `row05_c2Len_huge_inf_nan` | [x] |
| 41 | `sqrtf` domain | a **negative finite** radicand can never reach `sqrtf`: `c2Len` passes `dot(a,a) >= 0` and `c2RaytoCircle` guards with `disc < 0` (asserted empirically over 2400 adversarial vectors: `min c2Dot(a,a) >= 0`) | `-NaN` / `errno` path unreachable | `err39_41_len_overflow_and_nan` | [x] |
| 42 | `spec_ray` (`lib.c:316`) | `mp == ray.p` ⇒ `c2Norm(0,0)` = `(NaN,NaN)`, `ray.t = NaN` ⇒ `disc = NaN` | `0`, `*cast` untouched (asserted) | `err42_spec_ray_degenerate_direction` | [x] |
| 43 | `spec_ray` | `cast == NULL` and the ray misses ⇒ no store happens ⇒ no fault; and `cast == NULL` with a hit ⇒ fault | clean exit / SIGSEGV, identical in both | `phase_c_null::…` (cases `spec_ray_null_miss`, `spec_ray_null_hit`) | [x] |
| 44 | `spec_ray` | `c_r == 0 / -0.0 / < 0` — no validation; a ray line exactly through the centre still yields `disc == 0` and therefore a hit (measured 2280 hits / 2400) | `1` or `0` exactly as computed, no error | `err44_spec_ray_zero_radius_can_still_hit` | [x] |

## Generic boundaries covered in addition

* `±0.0` / `-0.0` operands for every arithmetic helper (the sign of the zero must
  match: `c2Absv(-0.0) == -0.0`, unlike `fabsf`).
* `f32::MIN_POSITIVE`, denormals (`1e-45`, `0x007FFFFF`), `f32::MAX`, `±inf`,
  quiet `NaN` (`0x7FC00000`), signalling `NaN` (`0x7FA00000`), negative `NaN`
  (`0xFFC00000`, `0xFFA00000`), and 20 000+ completely random 32-bit patterns.
* One `next_after` step past every documented comparison boundary: `disc == 0`
  (tangent ±1 ulp), `t == 0`, `t == A.t`, `d == 0`, `tN == 1.0f`, `d2 == r*r`,
  `|yAp.x| == B.r`, `y == 0`, `y == yBb.y`.
* Out-of-range enum values across the FFI boundary (row 34) — C enums accept any
  `int`, and all ten probed values are handled without a crash by both.
* Null pointers (rows 36, 37, 43) compared by **exit status *and* fatal signal
  number** in a re-exec'd child process.

## Note on `rustc`'s UB checks (why `[profile.dev] debug-assertions = false`)

With `debug-assertions` on, `rustc` injects a *null-pointer-dereference* check
that turns the row-36/37/43 stores into a non-unwinding panic → **SIGABRT (6)**,
whereas the C library faults with **SIGSEGV (11)**. `Cargo.toml` therefore
disables `debug-assertions`/`overflow-checks` for the dev profile so that the
dev `.so` is behaviourally identical both to the C `.so` and to the release
`.so`. (There is no integer arithmetic in the library that could overflow.)
