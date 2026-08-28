# ERRORS.md — Phase C error / rejection surface table

Mechanically derived by grepping **every** `return 0`, `return 1`, `return
!(...)`, comparison-guard, and fall-off-the-end path in `c_src/src/lib.c`.
There are no `assert`s, no error enums, no `errno` use, no `NULL` checks and no
min/max constants in the C source — the library rejects input purely through
`int` return codes (`0` = no hit / false, `1` = hit / true) and through
predicate results. Each distinct rejection branch gets its own row.

`grep -n 'return' c_src/src/lib.c` was used as the seed; the ternary-expanded
comparison guards were added by reading each `if`.

Legend for "expected C result": `ret` = value returned by the function;
`*out` = whether the `c2Raycast` out-parameter is written.

| #  | function | trigger (exact invalid input / condition) | expected C result | test (`tests/phase_c_errors.rs`) | [x] |
|----|----------|-------------------------------------------|-------------------|----------------------------------|-----|
|  1 | `c2RaytoCircle` | `disc = b*b - c < 0` (ray line misses circle) | `ret 0`, `*out` untouched | `err_01_circle_disc_negative` | [x] |
|  2 | `c2RaytoCircle` | `disc` is `NaN` (e.g. `A.p` or `B.p` `NaN`, or `inf-inf`) → `disc < 0` is **false**, so *not* rejected here; then `t = NaN` fails `t >= 0` | `ret 0`, `*out` untouched | `err_02_circle_disc_nan` | [x] |
|  3 | `c2RaytoCircle` | `t = -b - sqrt(disc) < 0` (circle entirely behind ray origin) | `ret 0`, `*out` untouched | `err_03_circle_t_negative` | [x] |
|  4 | `c2RaytoCircle` | `t > A.t` (hit beyond ray length; incl. `A.t < 0`, `A.t = NaN`) | `ret 0`, `*out` untouched | `err_04_circle_t_past_end` | [x] |
|  5 | `c2RaytoCircle` | `A.t = NaN` with an otherwise valid hit → `t <= A.t` false | `ret 0`, `*out` untouched | `err_04_circle_t_past_end` | [x] |
|  6 | `c2AABBtoAABB` | `B.max.x < A.min.x` (`d0`) → separated on -x | `ret 0` | `err_06_09_aabb_aabb_four_axes` | [x] |
|  7 | `c2AABBtoAABB` | `A.max.x < B.min.x` (`d1`) → separated on +x | `ret 0` | `err_06_09_aabb_aabb_four_axes` | [x] |
|  8 | `c2AABBtoAABB` | `B.max.y < A.min.y` (`d2`) → separated on -y | `ret 0` | `err_06_09_aabb_aabb_four_axes` | [x] |
|  9 | `c2AABBtoAABB` | `A.max.y < B.min.y` (`d3`) → separated on +y | `ret 0` | `err_06_09_aabb_aabb_four_axes` | [x] |
| 10 | `c2AABBtoAABB` | any coordinate `NaN` → every `<` is false → `d0..d3 == 0` → **accepts** | `ret 1` | `err_10_aabb_aabb_nan_accepts` | [x] |
| 11 | `c2AABBtoAABB` | inverted box (`min > max`) — no validation, result follows the raw comparisons | `ret` per formula | `err_11_aabb_aabb_inverted` | [x] |
| 12 | `c2RaytoAABB` | swept-ray bounding box misses `B` (`!c2AABBtoAABB(a_box,B)`) | `ret 0`, `*out` untouched | `err_12_aabb_broadphase_reject` | [x] |
| 13 | `c2RaytoAABB` | `d = |dot(n,p0-c)| - dot(abs_n,he) > 0` (SAT reject on ray axis) | `ret 0`, `*out` untouched | `err_13_aabb_sat_reject` | [x] |
| 14 | `c2RaytoAABB` | `hit0|hit1|hit2|hit3 == 0`, i.e. **all four** `t_i > 1.0f` | `ret 0`, `*out` untouched | `err_14_aabb_all_t_gt_one` | [x] |
| 15 | `c2RaytoAABB` | any `t_i` is `NaN` → `t_i <= 1.0f` false → that `hit_i = 0` | per formula | `err_15_aabb_nan_t` | [x] |
| 16 | `c2RaytoAABB` | `A.t = 0` (degenerate zero-length ray) — `p1 == p0`, `n == (0,0)` | per formula | `err_16_aabb_zero_length_ray` | [x] |
| 17 | `c2RayToPlane_OneDimensional` (via 12–14) | `da < 0` → returns `0` (plane behind) | contributes `t_i = 0` | `err_17_19_ray_plane_branches` | [x] |
| 18 | `c2RayToPlane_OneDimensional` (via 12–14) | `da*db > 0` → returns `1.0f` (no crossing) | contributes `t_i = 1` | `err_17_19_ray_plane_branches` | [x] |
| 19 | `c2RayToPlane_OneDimensional` (via 12–14) | `d = da-db == 0` → division rejected, returns `0` | contributes `t_i = 0` | `err_17_19_ray_plane_branches` | [x] |
| 20 | `c2AABBtoPoint` | `B.x < A.min.x` (`d0`) | `ret 0` | `err_20_23_aabb_point_four_axes` | [x] |
| 21 | `c2AABBtoPoint` | `B.y < A.min.y` (`d1`) | `ret 0` | `err_20_23_aabb_point_four_axes` | [x] |
| 22 | `c2AABBtoPoint` | `B.x > A.max.x` (`d2`) | `ret 0` | `err_20_23_aabb_point_four_axes` | [x] |
| 23 | `c2AABBtoPoint` | `B.y > A.max.y` (`d3`) | `ret 0` | `err_20_23_aabb_point_four_axes` | [x] |
| 24 | `c2AABBtoPoint` | `B` has a `NaN` component → all four false → **accepts** | `ret 1` | `err_24_aabb_point_nan_accepts` | [x] |
| 25 | `c2CircleToPoint` | `d2 >= A.r*A.r` (point outside/on circle) | `ret 0` | `err_25_circle_point_outside` | [x] |
| 26 | `c2CircleToPoint` | `A.r < 0` (negative radius) → `r*r > 0`, still a valid test | `ret` per formula | `err_26_circle_point_negative_r` | [x] |
| 27 | `c2CircleToPoint` | `A.r = 0` → `d2 < 0` impossible → always rejects | `ret 0` | `err_27_circle_point_zero_r` | [x] |
| 28 | `c2CircleToPoint` | `NaN` coordinate → `d2` `NaN` → `r*r > NaN` false | `ret 0` | `err_28_circle_point_nan` | [x] |
| 29 | `c2RaytoCapsule` | final `return 0`: `yAe.x*yAp.x >= 0` **and** `min(|yAe.x|,|yAp.x|) >= B.r` (ray stays outside the slab, never crosses) | `ret 0`, `*out` **written** (`n = norm(b-a)`, `t = 0`) | `err_29_capsule_outside_slab` | [x] |
| 30 | `c2RaytoCapsule` | degenerate capsule `B.a == B.b` → `norm((0,0))` = `0 * (1/0)` = `NaN` axis → all comparisons unordered | `ret 0` (or per formula), `*out` = `(NaN,NaN)`/0 | `err_30_capsule_degenerate_axis` | [x] |
| 31 | `c2RaytoCapsule` | `B.r = 0` (zero radius) | per formula | `err_31_capsule_zero_radius` | [x] |
| 32 | `c2RaytoCapsule` | `B.r < 0` (negative radius) → `capsule_bb.min.x = -r > 0 = max.x` inverted bb | per formula | `err_32_capsule_negative_radius` | [x] |
| 33 | `c2RaytoCapsule` | delegated rejection: `c2RaytoCircle(A,Ca,out)` returns `0` (end-cap A miss) | `ret 0`, `*out` = the pre-set `norm/0` values | `err_33_34_capsule_delegate_cap_miss` | [x] |
| 34 | `c2RaytoCapsule` | delegated rejection: `c2RaytoCircle(A,Cb,out)` returns `0` (end-cap B miss) | `ret 0`, `*out` = the pre-set `norm/0` values | `err_33_34_capsule_delegate_cap_miss` | [x] |
| 35 | `c2RaytoCapsule` | `d = yAe.x - yAp.x == 0` → `t = ±inf`/`NaN` divide, **no guard** (unlike `c2RayToPlane_OneDimensional`) | per formula, no rejection | `err_35_capsule_zero_denominator` | [x] |
| 36 | `c2Div` / `c2Norm` / `c2Len` | `b == 0` → `1.0f/0.0f = inf`; `c2Norm((0,0))` → `0*inf = NaN` | `(NaN, NaN)` | `err_36_div_norm_by_zero` | [x] |
| 37 | `c2Div` | `b == -0.0` → `1.0f/-0.0f = -inf` | `(-inf*x, ...)` | `err_37_div_negative_zero` | [x] |
| 38 | `c2CastRay` | `typeB` **out of range** (`3`, `4`, `-1`, `INT_MAX`, `INT_MIN`, `0x7fffffff`) → falls off the end of the `switch` with **no `default:` and no trailing `return`** (C UB) | at `-O0` gcc leaves `%eax` untouched → return value is the caller's leftover `eax`; `*out` untouched | `err_38_castray_out_of_range_type` | [x] |
| 39 | `c2CastRay` | `typeB == C2_TYPE_CAPSULE` has an unreachable `return 0;` after the `return c2RaytoCapsule(...)` — dead code, must not change behaviour | `ret` = `c2RaytoCapsule` result | `err_39_castray_capsule_dead_return` | [x] |
| 40 | `gen_ray` | `mp == ray.p` → `c2Norm((0,0))` = `(NaN,NaN)` → `ray.d`/`ray.t` `NaN`; everything downstream unordered | per formula | `err_40_gen_ray_degenerate_ray` | [x] |
| 41 | `gen_ray` | `hit` accumulator: all three sub-casts hit → `1 + 2 + 4 = 7`; none → `0` (full 0..7 bitmask range) | `ret` in `0..=7` | `err_41_gen_ray_hit_bitmask_range` | [x] |
| 42 | all `*out` writers | `out` aliasing: passing the **same** `c2Raycast*` as `cast1`/`cast2`/`cast3` to `gen_ray` | last writer wins, identically | `err_42_out_aliasing` | [x] |
| 43 | `c2Len` | `sqrtf` argument overflow: `a = (3e38, 3e38)` → `dot = inf` → `sqrt(inf) = inf` | `inf` | `err_43_len_overflow` | [x] |
| 44 | `c2Len` | `a` contains `NaN` → `dot` `NaN` → `sqrtf(NaN)` = quiet `NaN` (glibc `sqrtf` does *not* take the `isless(x,0)` error path) | quiet `NaN` | `err_44_len_nan` | [x] |

## Notes on absent error machinery

`grep -nE 'RETURN_ERROR|assert|errno|NULL|-1' c_src/src/lib.c` → no matches for
any of `RETURN_ERROR`, `assert`, `errno`, `NULL`. The only `-1` occurrences are
the literal normals `c2V(-1,0)` and `c2V(0,-1)` in `c2RaytoAABB`, and `-B.r` in
`c2RaytoCapsule`. There is therefore **no** null-pointer validation anywhere:
the C code dereferences `out` unconditionally in `c2RaytoCapsule` (before any
`return`) and on the hit path in `c2RaytoCircle` / `c2RaytoAABB`. Passing a
null `out` is genuine UB in both implementations and is therefore *not* tested
by dereference; instead rows 1–4 assert that `*out` is left untouched on the
reject paths, which is the observable half of that contract.
