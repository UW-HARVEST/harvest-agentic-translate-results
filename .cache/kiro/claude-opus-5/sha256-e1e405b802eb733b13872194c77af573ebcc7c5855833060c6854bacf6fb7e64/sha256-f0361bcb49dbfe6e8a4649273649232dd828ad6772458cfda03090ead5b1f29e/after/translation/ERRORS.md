# ERRORS.md — error / rejection surface table

`c_src/src/lib.c` contains **no** `assert`, no `RETURN_ERROR`-style macro, no
`return -1`, no `return NULL`, and no error enum. Mechanical greps
(`grep -n 'assert\|RETURN_ERROR\|return -1\|return NULL' lib.c`) come back
empty. Its entire rejection surface consists of:

* `switch` statements with a `default:` arm (lines 161, 292, 331, 353, 585, 597,
  609, 613) — an out-of-range enum/`count` value is *silently* mapped to a
  fallback result rather than an error code,
* one `switch` (`c2MakeProxy`, line 106) with **no** `default:` arm — an
  out-of-range `C2_TYPE` leaves the output object completely untouched,
* explicit null-pointer checks in `c2GJK` (lines 367, 371, and the
  `if (cache)` / `if (outA)` / `if (outB)` / `if (iterations)` guards),
* `return 0;` rejections in `c2Collided` (lines 586, 598, 610, 614),
* loop-termination / degeneracy guards inside `c2GJK` (iteration cap, duplicate
  support point, non-decreasing distance, epsilon-small search direction),
* implicit divisions with no zero guard (`c2Div`, `c2Norm`,
  `1.0f / s->div` in `c2Witness` / `c2L`) whose "error result" is an IEEE
  ±Inf / NaN that must propagate bit-identically.

One row per distinct rejection branch. Every row has a differential test in
`translation/tests/phase_c_errors.rs`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `c2Collided` | `typeA` outside `{0,1,2}` (e.g. `3`, `-1`, `INT_MAX`, `INT_MIN`) — outer `default:` (line 613) | returns `0`, neither shape dereferenced | `err01_collided_bad_typeA` |
| 2 | `c2Collided` | `typeA == C2_TYPE_CIRCLE`, `typeB` outside `{0,1,2}` — inner `default:` (line 586) | returns `0` | `err02_collided_circle_bad_typeB` |
| 3 | `c2Collided` | `typeA == C2_TYPE_AABB`, `typeB` outside `{0,1,2}` — inner `default:` (line 598) | returns `0` | `err03_collided_aabb_bad_typeB` |
| 4 | `c2Collided` | `typeA == C2_TYPE_CAPSULE`, `typeB` outside `{0,1,2}` — inner `default:` (line 610) | returns `0` | `err04_collided_capsule_bad_typeB` |
| 5 | `c2MakeProxy` | `type` outside `{0,1,2}`; `switch` has **no** `default:` | `*p` left byte-for-byte unmodified (caller's prior contents preserved, incl. `radius`, `count`, all 8 `verts`) | `err05_makeproxy_bad_type` |
| 6 | `c2GJK` | `ax_ptr == NULL` (line 367) | substitutes `c2xIdentity()`; result equals passing an explicit identity `c2x` | `err06_gjk_null_ax` |
| 7 | `c2GJK` | `bx_ptr == NULL` (line 371) | substitutes `c2xIdentity()`; result equals passing an explicit identity `c2x` | `err07_gjk_null_bx` |
| 8 | `c2GJK` | `outA == NULL` | no store through `outA`; return value and other outputs unchanged | `err08_gjk_null_outA` |
| 9 | `c2GJK` | `outB == NULL` | no store through `outB`; return value and other outputs unchanged | `err09_gjk_null_outB` |
| 10 | `c2GJK` | `iterations == NULL` | no store through `iterations` | `err10_gjk_null_iterations` |
| 11 | `c2GJK` | `cache == NULL` | no cache read, no cache write-back | `err11_gjk_null_cache` |
| 12 | `c2GJK` | `cache != NULL` but `cache->count == 0` (`cache_was_good` false) | cache not read; fresh 1-vertex simplex built; cache still written back on exit | `err12_gjk_zero_count_cache` |
| 13 | `c2GJK` | `cache != NULL`, `cache->count != 0`, arbitrary `metric`/`div` — the `!(min_metric < max_metric*2.0f && metric < -1.0e8f)` guard (line 404) is true for all finite metrics, so `cache_was_read = 1` unconditionally | warm-started simplex from the cached indices, with `u = 0` for every cached vertex | `err13_gjk_warm_cache` |
| 14 | `c2GJK` | `cache->metric = NaN` (makes both `min_metric`/`max_metric` comparisons false) | still `cache_was_read = 1` (the `&&` short-circuits identically) | `err14_gjk_nan_cache_metric` |
| 15 | `c2GJK` | `cache->metric = -1e30f` (satisfies `metric_old < ...`), combined with a real simplex metric | guard still true → `cache_was_read = 1` | `err15_gjk_huge_negative_cache_metric` |
| 16 | `c2Witness` | `s->count` outside `{1,2,3}` (`0`, `4`, `-1`, huge) — `default:` (line 331) | `*a = *b = c2V(0,0)` | `err16_witness_bad_count` |
| 17 | `c2Witness` | `s->div == 0` → `den = 1/0 = +Inf` | Inf/NaN components produced by `Inf * u`; must match bit-for-bit | `err17_witness_zero_div` |
| 18 | `c2GJKSimplexMetric` | `s->count` outside `{2,3}` (`default:` falls into `case 1:`, line 161–163) | returns `0.0f` | `err18_metric_bad_count` |
| 19 | `c2L` | `s->count` outside `{1,2}` (`default:`, line 353) — includes `count == 3` | returns `c2V(0,0)` | `err19_c2L_bad_count` |
| 20 | `c2L` | `s->count == 2` with `s->div == 0` → `den = +Inf` | Inf/NaN result, bit-identical | `err20_c2L_zero_div` |
| 21 | `c2D` | `s->count == 3` or any other value (`case 3: default:`, line 292) | returns `c2V(0,0)` | `err21_c2D_bad_count` |
| 22 | `c2Support` | `count <= 0` (`0`, `-1`, `INT_MIN`) — loop body never runs but `verts[0]` **is** read unconditionally | returns `0` | `err22_support_nonpositive_count` |
| 23 | `c2Support` | all dots equal / `d == (0,0)` — `dot > dmax` never true | returns `0` (first index wins ties) | `err23_support_tie` |
| 24 | `c2Support` | `d` or `verts` containing NaN, so every `dot > dmax` is false | returns `0` | `err24_support_nan` |
| 25 | `c2Div` | `b == 0.0f` (and `-0.0f`) | `c2Mulvs(a, 1/0)` → ±Inf / NaN (`0 * Inf`) components | `err25_div_by_zero` |
| 26 | `c2Norm` | zero-length vector → `c2Len == 0` → division by zero | `NaN`/`Inf` components | `err26_norm_zero_vector` |
| 27 | `c2Len` | negative `c2Dot(a,a)` is impossible, but `Inf`/`NaN` inputs give `sqrtf(NaN)`/`sqrtf(Inf)` | `NaN` / `+Inf` | `err27_len_nonfinite` |
| 28 | `c2Maxv` / `c2Minv` / `c2Clampv` | NaN operand: the C uses a ternary, **not** `fmaxf`/`fminf`, so a false comparison yields the *second* operand | `c2Maxv(a,b)` → `b.x` when `a.x > b.x` is false (incl. NaN) | `err28_minmax_nan` |
| 29 | `c2Clampv` | inverted range (`lo > hi`) | `max(lo, min(a,hi))` = `lo`, no rejection | `err29_clampv_inverted` |
| 30 | `c2CircletoAABB` | inverted AABB (`min > max`) | computed anyway via `c2Clampv`; result is whatever the clamp yields | `err30_circle_aabb_inverted` |
| 31 | `c2AABBtoAABB` | inverted AABB (`min > max`) on either side | pure comparison, no validation; `!(d0|d1|d2|d3)` | `err31_aabb_aabb_inverted` |
| 32 | `c2CircletoCircle` | negative radii (`r < 0`), so `r2 = (rA+rB)^2 >= 0` — sign is lost | `d2 < r2` computed on the squared sum | `err32_circle_circle_negative_radius` |
| 33 | `c2CircletoCapsule` | degenerate capsule `a == b` → `n = (0,0)`, `da = 0` (not `< 0`), `db = 0` (not `< 0`) → falls to the `bp` branch, **no** division by `c2Dot(n,n)` | `d2 = |A.p - B.b|^2` | `err33_circle_capsule_degenerate` |
| 34 | `c2CircletoCapsule` | `da >= 0 && db < 0` with `c2Dot(n,n)` denormal/near-zero → `da / dot` overflows | Inf/NaN `d2`, comparison `d2 < r*r` false | `err34_circle_capsule_tiny_axis` |
| 35 | `c2GJK` | iteration cap: `while (iter < 20)` (line 425) | `*iterations <= 20`; loop exits without any error signal. **Measured**: over 86 400 differential calls spanning all 9 type pairs × transforms × cold/zeroed/warm caches × both `use_radius` values, the maximum observed count is **3** — the three shape kinds yield proxies of at most 4 vertices, so `iter == 20` is unreachable through the public API. C and Rust agree on the count for every input | `err35_gjk_iteration_cap` |
| 36 | `c2GJK` | duplicate support point (`iA == saveA[i] && iB == saveB[i]`) | `break` before `++s.count`, so the just-written `verts[s.count]` is *not* counted | `err36_gjk_duplicate_support` |
| 37 | `c2GJK` | non-decreasing distance `d1 > d0` | `break` | `err37_gjk_d1_gt_d0` |
| 38 | `c2GJK` | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (epsilon-small search direction) | `break` | `err38_gjk_tiny_direction` |
| 39 | `c2GJK` | `s.count == 3` (`hit`) | `a = b`, `dist = 0` regardless of `use_radius` | `err39_gjk_hit_path` |
| 40 | `c2GJK` | `use_radius != 0` and `dist <= rA + rB` (overlap) | `a = b = midpoint`, `dist = 0` | `err40_gjk_radius_overlap` |
| 41 | `c2GJK` | `use_radius != 0` and `dist <= FLT_EPSILON` | same midpoint/`dist = 0` branch | `err41_gjk_radius_epsilon` |
| 42 | `c2GJK` | `use_radius != 0`, separated, but `a == b` after shrinking | `dist` forced to `0` while `a`/`b` keep their shrunk values | `err42_gjk_radius_ab_equal` |
| 43 | `c2GJK` | `use_radius == 0` | no radius adjustment at all — raw witness distance returned | `err43_gjk_no_radius` |
| 44 | `c2GJK` | `use_radius` a non-`0`/`1` int (`2`, `-1`, `INT_MIN`) — C truthiness | treated as true (radius path taken) | `err44_gjk_use_radius_nonbool` |
| 45 | `c2GJK` | shape containing NaN / ±Inf coordinates | NaN/Inf propagates through the simplex; return value and outputs must match bit-for-bit (incl. NaN payload) | `err45_gjk_nonfinite_shapes` |
| 46 | `c2AABBtoCapsule` / `c2CapsuletoCapsule` | `if (float)` truthiness of the `c2GJK` return. Both wrappers hard-code `use_radius = 1`, and on that path a NaN `dist` fails `dist > rA + rB` and is rewritten to `+0.0` by the midpoint branch — so a NaN can never escape them. The reachable classes are `+0.0` (falsy) and positive-finite / `+Inf` (truthy) | `+0.0` → returns `1`; positive or `+Inf` → returns `0`; NaN observed 0 times in 6000 adversarial inputs. The NaN-is-truthy behaviour IS reachable through `c2GJK` itself with `use_radius = 0`, which the same test also covers (1506 NaN distances, both libraries agreeing) | `err46_bool_wrappers_float_truthiness` |
| 47 | `aabb` | `NaN` / `±Inf` / inverted `min`/`max` arguments | the three `c2Collided` results packed as `b0 | b1<<1 | b2<<2` with no validation | `err47_aabb_entry_nonfinite` |
| 48 | `c2BBVerts` | inverted / NaN AABB | writes 4 verts unconditionally, no validation | `err48_bbverts_inverted` |
| 49 | `c22` | `v <= 0` (incl. `v == -0.0f`, `v == 0.0f`) | collapse to `count = 1`, `div = 1`, `a.u = 1` | `err49_c22_v_le_zero` |
| 50 | `c22` | `u <= 0` (and `v > 0`) | `a = b`, `count = 1` | `err50_c22_u_le_zero` |
| 51 | `c22` | NaN barycentric `u`/`v` (all comparisons false) | falls through to the `else` branch: `count = 2`, `div = NaN` | `err51_c22_nan` |
| 52 | `c23` | each of the 7 branches, incl. the NaN-driven fall-through to `count = 3` with `div = NaN` | branch-exact simplex mutation | `err52_c23_all_branches` |

## Notes on unreachable-by-construction UB

Two further C paths are reachable only through inputs whose C behaviour is
*undefined* (it reads indeterminate or out-of-bounds memory), so no
byte-identical expectation can exist and they are excluded from the table:

* `c2GJK` with an invalid `C2_TYPE`: `c2MakeProxy` leaves the `c2Proxy` stack
  object uninitialised, and the subsequent `pA.verts[0]` read is indeterminate.
* `c2GJK` with `cache->count > 3` or `cache->iA[i] >= 8`: the C indexes past
  `int iA[3]` / `c2v verts[8]`.

Row 5 covers the *defined* half of the first case (`c2MakeProxy` called directly
with a caller-initialised proxy), which is the observable contract.
