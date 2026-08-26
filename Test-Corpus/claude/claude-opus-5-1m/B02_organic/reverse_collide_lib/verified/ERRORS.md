# ERRORS.md — error / rejection surface of `c_src/src/lib.c`

Derived mechanically from the C source. The library has **no** `errno`, no
`RETURN_ERROR` macro, no `assert()`, no `return -1` / `return NULL`, and no
allocation. Its entire rejection surface consists of

* `switch` statements whose `default:` label silently returns a **sentinel**
  (`0`, `(0,0)`, or "leave the out-parameter untouched"),
* explicit **NULL-pointer checks** that substitute a default or skip a write,
* one **cache-invalidation guard** that discards caller-supplied state,
* unchecked divisions / `sqrtf` domains that produce IEEE sentinels
  (`inf`, `-inf`, `NaN`) instead of an error code.

Grep provenance for every row is given in the `C site` column.

| #  | function | C site | trigger (exact invalid input / condition) | expected C result | differential test | [x] |
|----|----------|--------|-------------------------------------------|-------------------|-------------------|-----|
| 1  | `c2MakeProxy` | `switch(type)` L114, no `default:` | `type` not in {0,1,2} (e.g. `3`, `-1`, `0x7fffffff`) | function is a **no-op**: `*p` is left exactly as the caller had it (radius/count/verts unmodified) | `c01_makeproxy_invalid_type_is_a_noop`, `g1_out_of_range_enum_values` | [x] |
| 2  | `c2GJKSimplexMetric` | `default: case 1:` L162-164 | `s->count` not in {2,3} (0, 1, 4, −1, huge) | returns `0.0f` | `c02_gjksimplexmetric_default_returns_zero` | [x] |
| 3  | `c2D` | `case 3: default:` L292-294 | `s->count` not in {1,2} (0, 3, 4, −1) | returns `c2v{0,0}` | `c03_cD_default_returns_zero_vector` | [x] |
| 4  | `c2Witness` | `default:` L332-334 | `s->count` not in {1,2,3} (0, 4, −1) | writes `*a = {0,0}`, `*b = {0,0}` (div/u ignored) | `c04_witness_default_writes_zero_vectors` | [x] |
| 5  | `c2L` | `default:` L354-355 | `s->count` not in {1,2} (0, 3, 4, −1) | returns `c2v{0,0}` (the `1.0f/div` is still computed, unused) | `c05_cL_default_returns_zero_vector` | [x] |
| 6  | `c2GJK` | `if (!ax_ptr)` L368 | `ax_ptr == NULL` | substitutes `c2xIdentity()` (`p={0,0}`, `r={1,0}`) instead of faulting | `c06_c07_null_transforms_equal_explicit_identity` | [x] |
| 7  | `c2GJK` | `if (!bx_ptr)` L372 | `bx_ptr == NULL` | substitutes `c2xIdentity()` | `c06_c07_null_transforms_equal_explicit_identity` | [x] |
| 8  | `c2GJK` | `if (cache)` L383 | `cache == NULL` | cache read *and* write-back are both skipped; simplex is cold-started | `c08_c12_c13_c14_null_out_params_are_skipped`, `b62_null_optional_pointer_matrix` | [x] |
| 9  | `c2GJK` | `int cache_was_good = !!cache->count;` L384 | `cache != NULL` but `cache->count == 0` | cache is rejected as "not good"; simplex cold-started from vertex 0 | `c09_cache_count_zero_is_rejected`, `b57_cold_cache` | [x] |
| 10 | `c2GJK` | `if (!(min_metric < max_metric*2.0f && metric < -1.0e8f))` L405 | cached simplex whose recomputed `metric` is `< -1.0e8f` **and** `min_metric < 2*max_metric` (e.g. `count=3` cache over huge coords, `cache->metric = 0`) | cache is **rejected** (`cache_was_read` stays 0) → simplex cold-started, cache overwritten on exit | `c10_cache_metric_guard_rejects_the_cache` | [x] |
| 11 | `c2GJK` | L386 loop bound is `cache->count`, unchecked | `cache->count < 0` | `for` body never runs; `s.count` becomes the negative value; every downstream `switch` takes its `default:` → returns `dist == 0`, `*outA = *outB = {0,0}`, `*iterations == 0`, and writes back `cache->count = <negative>`, `cache->metric = 0` | `c11_negative_cache_count` | [x] |
| 12 | `c2GJK` | `if (outA)` L510 | `outA == NULL` | write skipped, no fault | `c08_c12_c13_c14_null_out_params_are_skipped` | [x] |
| 13 | `c2GJK` | `if (outB)` L512 | `outB == NULL` | write skipped, no fault | `c08_c12_c13_c14_null_out_params_are_skipped` | [x] |
| 14 | `c2GJK` | `if (iterations)` L514 | `iterations == NULL` | write skipped, no fault | `c08_c12_c13_c14_null_out_params_are_skipped` | [x] |
| 15 | `c2GJK` | `while (iter < 20)` L425 | simplex that never terminates by duplicate/epsilon | hard iteration cap: `*iterations <= 20`, loop exits with whatever simplex it has | `c15_iteration_cap` | [x] |
| 16 | `c2GJK` | `if (d1 > d0) break;` L447 | non-monotonic distance progress (numerical noise) | early `break`, distance from the previous simplex is used | `c16_non_monotonic_progress_break`, `b64_huge_coordinates` | [x] |
| 17 | `c2GJK` | `if (c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON) break;` L451 | degenerate search direction (`d ≈ 0`), e.g. two identical shapes | early `break`; falls through to witness/radius handling | `c17_degenerate_direction_break`, `b56_identical_shapes` | [x] |
| 18 | `c2GJK` | `if (dup) break;` L471 | support point duplicates a saved index pair (always happens for a 1-vertex proxy, i.e. circle-vs-circle) | early `break` | `c18_duplicate_support_break` | [x] |
| 19 | `c2GJK` | `else` of L485 (`dist <= rA+rB \|\| dist <= FLT_EPSILON`) with `use_radius != 0` | overlapping / touching shapes | `dist` forced to `0`, `*outA = *outB = midpoint((a+b)*0.5f)` | `c19_radius_midpoint_collapse` | [x] |
| 20 | `c2GJK` | `if (a.x==b.x && a.y==b.y) dist = 0;` L491 | radius shrink collapses the two witness points | `dist` forced to `0` although the raw distance was `> rA+rB` | `c20_radius_shrink_collapse` | [x] |
| 21 | `c2GJK` | `if (hit)` L479 | `s.count == 3` reached (origin enclosed) | `a = b`, `dist = 0`, radii ignored entirely even when `use_radius != 0` | `c21_hit_ignores_radii` | [x] |
| 22 | `c2Collided` | `default: return 0;` L586-587 | `typeA == C2_TYPE_CIRCLE` and `typeB ∉ {0,1,2}` | returns `0` | `c22_c23_c24_c25_collided_invalid_types_return_zero` | [x] |
| 23 | `c2Collided` | `default: return 0;` L598-599 | `typeA == C2_TYPE_AABB` and `typeB ∉ {0,1,2}` | returns `0` | `c22_c23_c24_c25_collided_invalid_types_return_zero` | [x] |
| 24 | `c2Collided` | `default: return 0;` L610-611 | `typeA == C2_TYPE_CAPSULE` and `typeB ∉ {0,1,2}` | returns `0` | `c22_c23_c24_c25_collided_invalid_types_return_zero` | [x] |
| 25 | `c2Collided` | `default: return 0;` L614-615 | `typeA ∉ {0,1,2}` (any `typeB`, incl. invalid) | returns `0` — `B` is never dereferenced | `c22_c23_c24_c25_collided_invalid_types_return_zero` | [x] |
| 26 | `c2Support` | `for (i=1; i<count; ...)`, `verts[0]` read unconditionally L299-306 | `count <= 0` | still reads `verts[0]`; loop body never runs; returns `0` | `c26_support_nonpositive_count_returns_zero` | [x] |
| 27 | `c2Support` | `if (dot > dmax)` strict `>` L303 | tie between two vertices with equal projection | returns the **lowest** index (first maximum wins) | `c27_support_tie_picks_lowest_index` | [x] |
| 28 | `c2Div` | `c2Mulvs(a, 1.0f/b)` L339, no zero check | `b == 0.0f` | `1/0 = +inf` → components become `±inf` or `NaN` (for a `0` component); `b == -0.0f` → `-inf` | `c28_div_by_zero` | [x] |
| 29 | `c2Norm` | `c2Div(a, c2Len(a))` L343, no zero check | `a == {0,0}` | `c2Len == 0` → `{NaN, NaN}` (`0 * inf`) | `c29_norm_of_zero_vector` | [x] |
| 30 | `c2Len` | `sqrtf(c2Dot(a,a))` L153 | component overflow (e.g. `1e30`) → `c2Dot` overflows to `+inf` | returns `+inf`; `NaN` input propagates `NaN` | `c30_len_overflow_and_nan` | [x] |
| 31 | `c2Witness` | `float den = 1.0f / s->div;` L312, no zero check | `s->div == 0` and `count ∈ {2,3}` | `den = ±inf` → witness points become `±inf`/`NaN` | `c31_c32_zero_div_in_witness_and_L` | [x] |
| 32 | `c2L` | `float den = 1.0f / s->div;` L347, no zero check | `s->div == 0` and `count == 2` | returns `±inf`/`NaN` components | `c31_c32_zero_div_in_witness_and_L` | [x] |
| 33 | `c2CircletoCapsule` | `da / c2Dot(n,n)` L565, no zero check | degenerate capsule `B.a == B.b`, so `n == (0,0)` and `c2Dot(n,n) == 0` | the division is **unreachable**: `da == 0` is not `< 0` and `db == 0` is not `< 0`, so the C skips the perpendicular branch and falls into the `bp` branch. A degenerate capsule therefore behaves exactly like a circle centred on `B.b` (verified against `c2CircletoCircle` in `c33_degenerate_capsule_takes_the_bp_branch`). Same for a *near*-degenerate capsule whose `c2Dot(n,n)` underflows to `0`: `da` and `db` are then bounded by that same underflowing magnitude and cannot straddle zero. | `c33_degenerate_capsule_takes_the_bp_branch` | [x] |
| 34 | `c2CircletoCircle` / `c2CircletoAABB` / `c2CircletoCapsule` | strict `<` L544, L552, L573 | exactly-touching shapes (`d2 == r2`) | returns `0` (touching is *not* a collision) | `c34_exact_touch_is_not_a_collision` | [x] |
| 35 | `c2AABBtoAABB` | `B.max.x < A.min.x` etc. L520-523 | exactly-touching or `NaN` coordinates | `NaN` comparisons are all false → `d0..d3 == 0` → returns `1` (**inverted AABBs and NaN boxes report a hit**) | `c35_aabb_nan_and_inverted_report_hit` | [x] |
| 36 | `c2AABBtoCapsule` / `c2CapsuletoCapsule` | `if (c2GJK(...)) return 0; return 1;` L528-530, L534-536 | `c2GJK` returns a non-zero float, including `NaN` (`NaN != 0` is true in C's implicit `!= 0`) | returns `0` (no collision) — a `NaN` distance is reported as *no* hit | `c36_nan_distance_reports_no_collision` | [x] |
| 37 | `c2Clampv` | `c2Maxv(lo, c2Minv(a, hi))` L73 | inverted box (`lo > hi`) | no rejection: silently returns `lo` | `c37_clampv_inverted_box` | [x] |
| 38 | `reverse_collide` | L619-646, no validation | `NaN` / `inf` / negative `r` | never errors; returns a 3-bit mask in `0..=7` (negative `r` squares to positive, so hits are still possible) | `c38_reverse_collide_never_errors`, `b73_reverse_collide_boundaries` | [x] |

## Generic FFI boundary cases also covered by the Phase C tests

| #  | case | rows | differential test | [x] |
|----|------|------|-------------------|-----|
| G1 | out-of-range `C2_TYPE` values passed as `int`: `3`, `4`, `-1`, `255`, `1<<30`, `INT_MIN`, `INT_MAX`, plus 64 random non-variant ints | 1, 22-25 | `g1_out_of_range_enum_values`, `c01_...`, `c22_c23_c24_c25_...` | [x] |
| G2 | `NULL` for every nullable pointer parameter of `c2GJK` (`ax_ptr`, `bx_ptr`, `outA`, `outB`, `iterations`, `cache`) in all 64 combinations | 6-8, 12-14 | `b62_null_optional_pointer_matrix`, `c08_c12_c13_c14_...` | [x] |
| G3 | `use_radius` values other than 0/1 (`2`, `-1`, `7`, `INT_MIN`, `INT_MAX`) must behave like 1 | 19 | `c19_radius_midpoint_collapse` | [x] |
| G4 | `s->count` one step past every valid range (`-1`, `-2`, `0`, `4`, `5`, `100`, `INT_MIN`, `INT_MAX`) for `c2D`, `c2L`, `c2Witness`, `c2GJKSimplexMetric` | 2-5 | `c02_...`, `c03_...`, `c04_...`, `c05_...` | [x] |
| G5 | `count = 0`, `-1`, `-2`, `-100`, `INT_MIN` for `c2Support`; oversized `count = 8` with 8 slots | 26-27 | `c26_...`, `c27_...`, `b09_support` | [x] |
| G6 | the full `f32` boundary grid (`0`, `-0`, `FLT_MIN`, smallest denormal, `FLT_EPSILON`, `±FLT_MAX`, `±inf`, `NaN`) through **every** scalar/vector export, as a 3-deep cross-product | 28-35, 38 | `g6_boundary_grid_through_every_value_entry_point` | [x] |
| G7 | every distinct NaN class (sign x quiet/signalling x payload) cross-multiplied through every arithmetic export, to pin SSE NaN-payload propagation | 28-33 | `bnan1_nan_payload_matrix_leaf_functions`, `bnan2_nan_payload_matrix_composites` | [x] |
| G8 | every one of the 38 exported symbols is proven to be invoked differentially at least once | all | `g7_every_exported_symbol_is_exercised_differentially` | [x] |

**All 38 rows + all 8 generic rows have a passing differential test.**

## Deliberately NOT tested — undefined behaviour in the C

"Identical behaviour" is not defined for these, because the C itself has no
defined behaviour. Each is listed with the reason.

| case | why it is UB, and the evidence |
|------|--------------------------------|
| `NULL` for the **non**-nullable parameters — `c2MakeProxy(shape=NULL)` with a *valid* type, `c2Collided(A=NULL)` with a valid `typeA`, `c2Support(verts=NULL)`, `c2GJK(A=NULL)`, `c2Witness(s=NULL)`, `c2BBVerts(out=NULL)` | the C dereferences them unconditionally; there is no null check to match. (`c2Collided` with an **invalid** `typeA` never dereferences `B`, so `B=NULL` *is* tested — see row 25.) |
| `c2GJK` with a `C2_TYPE` outside {0,1,2} | `c2MakeProxy` is then a no-op (row 1), so `c2Proxy pA;` stays **uninitialised** and the C reads indeterminate stack memory for `pA.count` / `pA.verts[0]`. Measured (`tests/phase_c_ub_probe.rs`, run with `--ignored`): the same call returns `dist = 0, outA = (NaN, NaN)` on a clean stack and `dist = inf, outA = (1e20, -2e20)` — leaking a previous call's shape data — after an unrelated `c2GJK` call dirtied the stack. The answer is not a function of the inputs, so no translation can reproduce it. Rust's zero-initialised `c2Proxy::default()` is the closest defined behaviour. |
| `cache->count > 3` | the write-back loop `cache->iA[i] = ...` runs past the end of the 3-element `iA` array, and `cache->iB[3]` reads the `div` field as an `int` which is then used as an unchecked `pB.verts[iB]` index. |
| cache indices `iA[i] / iB[i]` outside `[0, proxy.count)` | `pA.verts[iA]` then reads a slot `c2MakeProxy` never wrote, i.e. uninitialised stack in the C versus zeroes in Rust — the same indeterminate-value problem as above. All warm-cache tests therefore draw indices from `[0, proxy.count)`. |

Everything outside this table is covered.
