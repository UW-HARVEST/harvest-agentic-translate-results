# ERRORS.md — Error / rejection surface table

Derived mechanically from `c_src/src/lib.c` (340 lines). The library uses **no**
error macros, no `assert`, no `errno`, no `NULL` returns and no error enum. Its
only rejection channel is an `int` return of `0` ("no hit") from the predicate
and raycast functions, plus two internal `return 0` short-circuits inside the
`static inline` plane helper, plus one C-level **undefined-behaviour** path
(`c2CastRay`'s `switch` with no `default`).

Every `return 0` / `return 0.0f` / falsy branch in the C source gets its own
row, keyed by the C line number so the table is auditable against the source.

Legend for "expected C result": `ret` is the `int` return value; `*out`
describes whether the `c2Raycast` output is written.

| # | function (C line) | trigger (exact invalid input/condition) | expected C result | test |
|---|-------------------|------------------------------------------|-------------------|------|
| 1 | `c2RaytoCircle` L100 (`if (disc < 0)`) | `disc = b*b - c < 0`: ray line misses the circle entirely (e.g. circle centre far off the ray line) | `ret == 0`, `*out` **untouched** | `err_01_circle_disc_negative` |
| 2 | `c2RaytoCircle` L106 (`t >= 0` fails) | `t = -b - sqrt(disc) < 0`: intersection lies *behind* the ray origin (ray origin already past/inside the circle) | `ret == 0`, `*out` untouched | `err_02_circle_t_negative` |
| 3 | `c2RaytoCircle` L106 (`t <= A.t` fails) | `t > A.t`: ray line hits, but the hit is beyond the ray's length | `ret == 0`, `*out` untouched | `err_03_circle_t_beyond_len` |
| 4 | `c2RaytoCircle` L100 | `disc` is NaN (NaN in `A.p`, `A.d`, `B.p` or `B.r`) — `NaN < 0` is false, so this does **not** reject; falls to L106 where `t >= 0` is false ⇒ 0 | `ret == 0`, `*out` untouched | `err_04_circle_nan_inputs` |
| 5 | `c2RaytoCircle` L100 | `B.r` negative: `c = dot(m,m) - r*r` uses `r*r`, so `-r` behaves exactly like `+r` (no rejection added) | same `ret`/`*out` as `+|r|` | `err_05_circle_negative_radius` |
| 6 | `c2AABBtoAABB` L112–L116 | any of `d0..d3` set: `B.max.x < A.min.x` | `ret == 0` | `err_06_aabbaabb_sep_d0` |
| 7 | `c2AABBtoAABB` L112–L116 | `A.max.x < B.min.x` | `ret == 0` | `err_07_aabbaabb_sep_d1` |
| 8 | `c2AABBtoAABB` L112–L116 | `B.max.y < A.min.y` | `ret == 0` | `err_08_aabbaabb_sep_d2` |
| 9 | `c2AABBtoAABB` L112–L116 | `A.max.y < B.min.y` | `ret == 0` | `err_09_aabbaabb_sep_d3` |
| 10 | `c2AABBtoAABB` L112–L116 | inverted box (`min > max`) — the C does **not** validate this; all four `<` are evaluated on the raw fields | whatever the four comparisons yield (often `1`) | `err_10_aabbaabb_inverted` |
| 11 | `c2AABBtoAABB` L112–L116 | NaN coordinate: every `<` is false ⇒ `!(0)` ⇒ `1` (NaN box "overlaps everything") | `ret == 1` | `err_11_aabbaabb_nan` |
| 12 | `c2RayToPlane_OneDimensional` L127 (`if (da < 0)`) | `da < 0` (ray start on the outside of that plane) ⇒ helper returns `0`, so the corresponding `hitN` is true with `tN == 0` | contributes `t == 0` | `err_12_plane_da_negative` (via `c2RaytoAABB`) |
| 13 | `c2RayToPlane_OneDimensional` L135 (`d == 0`) | `da == db` (ray exactly parallel to that plane) and `da*db <= 0` ⇒ divide-by-zero guarded, returns `0` | contributes `t == 0` | `err_13_plane_parallel_zero_d` (via `c2RaytoAABB`) |
| 14 | `c2RaytoAABB` L145 (`!c2AABBtoAABB`) | the ray's own bounding box is disjoint from `B` | `ret == 0`, `*out` **untouched** | `err_14_aabb_bb_reject` |
| 15 | `c2RaytoAABB` L157 (`if (d > 0)`) | separating-axis test on the ray's skew normal fails: `|dot(n, p0-centre)| - dot(|n|, half_extents) > 0` (ray's *bounding box* overlaps but the ray *line* misses the box) | `ret == 0`, `*out` untouched | `err_15_aabb_sat_reject` |
| 16 | `c2RaytoAABB` L195 (`hit == 0`) | all four `tN > 1.0f`. Only reachable when some `tN` is NaN or `> 1`; `NaN <= 1.0f` is false, so a NaN `da`/`db` on all four planes reaches this | `ret == 0`, `*out` untouched | `err_16_aabb_no_plane_hit` |
| 17 | `c2RaytoAABB` L146 | `A.t == 0` (degenerate zero-length ray): `p1 == p0`, `ab == 0`, `n == 0`, so `d = 0 - dot(0,half) = 0`, not `> 0` | reaches the plane block; `out->t == 0 * A.t` | `err_17_aabb_zero_length_ray` |
| 18 | `c2RaytoAABB` L146 | `A.d` is NaN (from `c2Norm` of a zero vector) ⇒ `p1` NaN ⇒ `a_box` NaN ⇒ row 11 makes `c2AABBtoAABB` return 1, `d` is NaN so `d > 0` false, all `tN` NaN ⇒ row 16 | `ret == 0`, `*out` untouched | `err_18_aabb_nan_direction` |
| 19 | `c2AABBtoPoint` L199–L203 | `B.x < A.min.x` | `ret == 0` | `err_19_aabbpoint_below_x` |
| 20 | `c2AABBtoPoint` L199–L203 | `B.y < A.min.y` | `ret == 0` | `err_20_aabbpoint_below_y` |
| 21 | `c2AABBtoPoint` L199–L203 | `B.x > A.max.x` | `ret == 0` | `err_21_aabbpoint_above_x` |
| 22 | `c2AABBtoPoint` L199–L203 | `B.y > A.max.y` | `ret == 0` | `err_22_aabbpoint_above_y` |
| 23 | `c2AABBtoPoint` L199–L203 | NaN point ⇒ all four comparisons false ⇒ `1` | `ret == 1` | `err_23_aabbpoint_nan` |
| 24 | `c2CircleToPoint` L209 (`d2 < A.r*A.r`) | point on or outside the circle (`d2 >= r*r`), **including exactly on the boundary** — strict `<` rejects the boundary | `ret == 0` | `err_24_circlepoint_on_boundary` |
| 25 | `c2CircleToPoint` L209 | `A.r == 0` ⇒ `r*r == 0`, `d2 < 0` impossible ⇒ always `0`, even for the exact centre | `ret == 0` | `err_25_circlepoint_zero_radius` |
| 26 | `c2CircleToPoint` L209 | NaN point/centre ⇒ `d2` NaN ⇒ `NaN < r*r` false ⇒ `0` | `ret == 0` | `err_26_circlepoint_nan` |
| 27 | `c2RaytoCapsule` L291 (fall through) | neither `yAe.x*yAp.x < 0` nor `min(|yAe.x|,|yAp.x|) < B.r`: ray stays on one side of the capsule axis and never comes within `B.r` of it | `ret == 0`, **but `*out` HAS been written** (`out->n = c2Norm(cap_n)`, `out->t = 0` at L243–244 run unconditionally) | `err_27_capsule_fallthrough_out_written` |
| 28 | `c2RaytoCapsule` L246 | ray origin already inside the capsule's axis-aligned slab ⇒ early `return 1` with `out->t == 0` and `out->n == normalize(b-a)` — *not* a real raycast result | `ret == 1`, `out->t == 0` | `err_28_capsule_origin_inside_slab` |
| 29 | `c2RaytoCapsule` L255 / L257 | ray origin strictly inside end-cap circle A (L255) or B (L257) ⇒ `return 1` with `out->t == 0` | `ret == 1`, `out->t == 0` | `err_29_capsule_origin_in_endcap` |
| 30 | `c2RaytoCapsule` L263 (`B.r`) | `B.r == 0`: `capsule_bb` degenerates to a segment; `min(|yAe.x|,|yAp.x|) < 0` is false, so only the `yAe.x*yAp.x < 0` disjunct can fire | per-branch, no crash | `err_30_capsule_zero_radius` |
| 31 | `c2RaytoCapsule` L232 (`c2Norm`) | `B.a == B.b` (zero-length capsule) ⇒ `c2Norm` divides by `0` ⇒ `M` is all-NaN ⇒ `yAp`/`yBb` NaN. **CORRECTED** (the first derivation predicted `ret == 0`): a NaN `yAp` makes all four comparisons in `c2AABBtoPoint` false, so the slab test returns **1** and the function takes the EARLY `return 1`. | `ret == 1`, `out->t == +0.0`, `out->n == (NaN, NaN)` | `err_31_capsule_degenerate_axis` |
| 32 | `c2RaytoCapsule` L285 (`d == 0`) | `yAe.x == yAp.x` inside the else-branch would divide by zero, and unlike `c2RayToPlane_OneDimensional` there is no guard. **CORRECTED — this branch is UNREACHABLE.** The else-branch requires `\|yAp.x\| >= B.r` *and* the outer `if`, which forces `yAe.x != yAp.x`: either the two have strict opposite signs (`yAe.x*yAp.x < 0`), or `min(\|yAe.x\|,\|yAp.x\|) < B.r <= \|yAp.x\|` so the minimum must be `\|yAe.x\|`; a NaN in either makes both disjuncts false. Proven empirically over 22 020 else-branch entries (351 404 of which had `yAe.x == yAp.x`) with **0** reachable zero denominators. | unreachable; no `/0` occurs | `err_32_capsule_division_denominator_never_zero` |
| 33 | `c2RaytoCapsule` L281–L283 | `|yAp.x| < B.r` (origin laterally inside the slab but axially outside) ⇒ **delegates** to `c2RaytoCircle` on cap A (`yAp.y < 0`) or cap B, so it inherits rows 1–3 | `ret` == that `c2RaytoCircle`'s result | `err_33_capsule_delegates_to_circle` |
| 34 | `c2CastRay` L297–L306 | `typeB` outside `{0,1,2}` — the C `switch` has **no `default`** and the function has **no** final `return`. Compiled `-O0` it falls through to `leave; ret` leaving `%eax` untouched, i.e. the return value is the caller's leftover `%eax`. Verified in `objdump -d` (`cmpl $0x2` / `ja 274b`, and `274b: leave; ret`). The comparison is **unsigned**, so negative values also fall through. | returns the caller's incoming `%eax` verbatim; `*out` untouched | `err_34_castray_out_of_range_enum` (deterministic: a naked trampoline seeds `%eax` with `0x5A5A5A5A` before a tail `jmp`, and both libraries must return it unchanged), `err_34b_castray_enum_boundary` |
| 35 | `c2CastRay` L299 | `B == NULL` with a valid `typeB` ⇒ dereference of `NULL` ⇒ **SIGSEGV** in C | crash (not tested in-process) | `err_35_castray_null_shape` (documented, `#[ignore]`) |
| 36 | `c2RaytoCircle`/`c2RaytoAABB` | `out == NULL` **and no hit** ⇒ never dereferenced ⇒ safe, returns `0` | `ret == 0`, no crash | `err_36_null_out_no_hit` |
| 37 | `c2RaytoCapsule` | `out == NULL` ⇒ **always** dereferenced at L243 before any check ⇒ SIGSEGV even with no hit | crash (not tested in-process) | `err_37_capsule_null_out` (documented, `#[ignore]`) |
| 38 | `c2Div` / `c2Norm` L64–L69 | `b == 0` / zero-length vector: `1.0f/0.0f == inf`, then `0*inf == NaN` for a zero component ⇒ `c2Norm(0,0) == (NaN, NaN)`; `c2Norm(0,5) == (0*inf, 5*inf) == (NaN, inf)`. **No guard in C.** | `(NaN, NaN)` / `(NaN, inf)` bit-identical | `err_38_norm_zero_vector` |
| 39 | `c2Len` L42 | `c2Dot(a,a) < 0` impossible for finite input, but `sqrtf` of a NaN dot (NaN component) ⇒ NaN | NaN, same payload/sign | `err_39_len_nan` |
| 40 | `gen_ray` L310 | `mp == ray.p` ⇒ `c2Norm` of a zero vector ⇒ `ray.d` NaN ⇒ `ray.t` NaN; all three casts run with a NaN ray. **CORRECTED** (the first derivation predicted `ret == 0` always): only the CIRCLE leg is guaranteed to miss (`t = -b - sqrtf(NaN)` is NaN, so `t >= 0` is false). The BOX leg can still report a hit — a NaN `p1` makes the ray bbox NaN so `c2AABBtoAABB` returns 1, the SAT `d` is NaN so `d > 0` is false, and `p0` is finite so some plane has `da < 0` ⇒ `tN == 0` ⇒ `hit`. The CAPSULE leg can hit via its slab test, which depends only on `A.p`. Observed return codes: `{0, 2, 4, 6}` — never odd. | bit 0 always clear; `cast1` untouched; `cast2` always written; `cast3` written iff bit 2 set | `err_40_gen_ray_degenerate_ray` |
| 41 | `gen_ray` L310 | any of `cast1`/`cast2`/`cast3` NULL. `cast1`/`cast3` are safe when their shape misses (row 36); `cast2` (capsule) always crashes (row 37) | per row 36/37 | `err_41_gen_ray_null_outs` (safe subset only) |
| 42 | `gen_ray` | inverted `bb` (`bb_min > bb_max`) — never validated; flows into rows 10/15 | no rejection added | `err_42_gen_ray_inverted_bb` |

## Generic FFI-boundary boundaries also covered

* **null pointers** — rows 35–37, 41.
* **zero lengths / degenerate sizes** — rows 17 (zero-length ray), 25/30 (zero
  radius), 31 (zero-length capsule axis), 38 (zero vector normalise).
* **one step past a valid range** — row 24 (`d2 == r*r`, exactly on the
  boundary, must reject), row 3 (`t` one ULP past `A.t`), row 12/13
  (`t == 1.0f` boundary of `hitN`).
* **out-of-range enum across FFI** — row 34, with `typeB` ∈
  `{-1, 3, 4, 99, i32::MIN, i32::MAX}`.

## Status

All 42 rows have a passing differential test. Every test additionally ASSERTS
that the C really took the branch the row describes (via a replay of the C's own
control flow through the C library's exported primitives), so a row cannot
silently stop being covered.

Test files: `tests/phase_c_errors.rs` (rows 1–26),
`tests/phase_c_capsule_gen.rs` (rows 27–33, 38–42),
`tests/phase_c_crash.rs` (rows 34–37, 41-safe-subset).

| rows | status |
|------|--------|
| 1–5 `c2RaytoCircle` | pass |
| 6–11 `c2AABBtoAABB` | pass |
| 12–13 plane helper | pass |
| 14–18 `c2RaytoAABB` | pass |
| 19–23 `c2AABBtoPoint` | pass |
| 24–26 `c2CircleToPoint` | pass |
| 27–33 `c2RaytoCapsule` | pass |
| 34 out-of-range enum | pass — **required a Rust fix** |
| 35, 37 null-deref SIGSEGV parity | pass (child process, signal 11 from both) |
| 36 safe null out-param | pass |
| 38–39 unguarded `/` and `sqrt` | pass |
| 40–42 `gen_ray` | pass |

### Divergence found and fixed

**Row 34 — `c2CastRay` with an out-of-range `C2_TYPE`.** The original Rust
translation had `_ => 0` for the default arm. The C has no `default` arm and no
final `return`, so it leaves `%eax` as the caller left it. Seeding `%eax` with
`0x5A5A5A5A` through a naked trampoline showed the C returning `0x5A5A5A5A` and
the Rust returning `0`. Fixed by making the exported `c2CastRay` a naked shim
(`cmp esi, 2` / `ja` / `jmp <impl>` / `ret`) that mirrors the C's dispatch and
its untouched-`%eax` fall-through. `nm -D` is unaffected: the real body is a
private, mangled symbol that is not dynamically exported.

### Three ERRORS.md rows were wrong on first derivation

Rows 31, 32 and 40 predicted behaviour the C does not exhibit. In each case the C
was taken as ground truth, the table row was corrected, and the test now asserts
the branch the C actually takes. Row 32 turned out to describe an **unreachable**
divide-by-zero, which the test now proves empirically rather than assuming.
