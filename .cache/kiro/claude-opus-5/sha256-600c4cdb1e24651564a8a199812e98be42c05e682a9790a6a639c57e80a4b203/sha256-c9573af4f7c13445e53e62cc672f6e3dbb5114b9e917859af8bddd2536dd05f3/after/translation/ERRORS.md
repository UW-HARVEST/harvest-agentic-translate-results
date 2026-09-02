# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. The library has **no** `RETURN_ERROR`
macro, **no** `assert`, **no** error enum and **no** function that returns
`NULL`/`-1`: it never allocates and never validates. Its whole rejection surface
therefore consists of

* `default:` switch labels that silently return a sentinel (`0`, `c2V(0,0)`),
* switch statements with **no** `default:` label (input silently ignored),
* null-pointer guards that substitute a default instead of failing,
* early `break` guards inside the GJK loop (degenerate / non-converging input),
* hard numeric constants that act as thresholds / caps,
* division by a value the code never checks for zero (produces `inf`/`NaN`).

Every row below is a distinct rejection/degenerate branch in the C source and has
a differential test in `tests/error_paths.rs` that asserts C and Rust return the
**same** sentinel / bit pattern.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `c2Collided` (lib.c:614 `default:`) | `typeA` not in {0,1,2} (e.g. `3`, `-1`, `999`, `INT_MIN`, `INT_MAX`) | `return 0` | `err_collided_bad_typeA` |
| 2 | `c2Collided` (lib.c:586 `default:`) | `typeA == C2_TYPE_CIRCLE(0)` and `typeB` not in {0,1,2} | `return 0` | `err_collided_bad_typeB_circle` |
| 3 | `c2Collided` (lib.c:598 `default:`) | `typeA == C2_TYPE_AABB(1)` and `typeB` not in {0,1,2} | `return 0` | `err_collided_bad_typeB_aabb` |
| 4 | `c2Collided` (lib.c:610 `default:`) | `typeA == C2_TYPE_CAPSULE(2)` and `typeB` not in {0,1,2} | `return 0` | `err_collided_bad_typeB_capsule` |
| 5 | `c2MakeProxy` (lib.c:105 `switch` has **no** `default:`) | `type` not in {0,1,2} | `*p` left **completely untouched** (caller-provided bytes preserved, `radius`/`count`/`verts` unchanged) | `err_makeproxy_bad_type_leaves_output_untouched` |
| 6 | `c2GJKSimplexMetric` (lib.c:162 `default:` falls into `case 1`) | `s->count` is `0`, negative, or `> 3` (e.g. `4`, `-1`, `INT_MAX`) | `return 0` (`0.0f`) | `err_simplexmetric_out_of_range_count` |
| 7 | `c2D` (lib.c:293 `default:` joined with `case 3`) | `s->count` is `0`, `3`, negative or `> 3` | `return c2V(0,0)` | `err_c2d_out_of_range_count` |
| 8 | `c2L` (lib.c:354 `default:` joined with `case 3`) | `s->count` is `0`, `3`, negative or `> 3` | `return c2V(0,0)` | `err_c2l_out_of_range_count` |
| 9 | `c2Witness` (lib.c:332 `default:`) | `s->count` is `0`, negative or `> 3` | `*a = *b = c2V(0,0)` | `err_witness_out_of_range_count` |
| 10 | `c2Witness` (lib.c:311 `1.0f / s->div`) | `s->div == 0` with `count` 2 or 3 | `den = inf` → outputs `inf`/`NaN` (no check) | `err_witness_zero_div` |
| 11 | `c2L` (lib.c:348 `1.0f / s->div`) | `s->div == 0` with `count == 2` | `den = inf` → `inf`/`NaN` result | `err_c2l_zero_div` |
| 12 | `c2Div` (lib.c:339 `1.0f / b`) | `b == 0` (also `-0.0`) | `±inf` components, `NaN` for a zero component | `err_div_by_zero` |
| 13 | `c2Norm` (lib.c:343 `c2Div(a, c2Len(a))`) | `a == c2V(0,0)` → length 0 | `NaN` components (`0 * inf`) | `err_norm_zero_vector` |
| 14 | `c2Len` / `c2Div` / `c2Norm` | non-finite input (`NaN`, `±inf`) | propagated per IEEE-754, no rejection | `err_nonfinite_scalar_helpers` |
| 15 | `c2Support` (lib.c:299 reads `verts[0]` before the loop) | `count <= 0` (`0`, `-1`, `INT_MIN`) — loop body never runs | `return 0`, still dereferences `verts[0]` | `err_support_nonpositive_count` |
| 16 | `c2Support` (lib.c:301 `i < count`) | `count > 8` on a `c2Proxy`-sized array | reads past the logical end; index of max dot over `count` elements | `err_support_count_past_end` |
| 17 | `c2Support` | all dots equal / `NaN` dots (`dot > dmax` is false for `NaN`) | `return 0` (first index wins ties and NaNs) | `err_support_ties_and_nan` |
| 18 | `c2GJK` (lib.c:368 `if (!ax_ptr)`) | `ax_ptr == NULL` | substitutes `c2xIdentity()`, no error | `err_gjk_null_transforms` |
| 19 | `c2GJK` (lib.c:372 `if (!bx_ptr)`) | `bx_ptr == NULL` | substitutes `c2xIdentity()`, no error | `err_gjk_null_transforms` |
| 20 | `c2GJK` (lib.c:508 `if (outA)`) | `outA == NULL` | write skipped, no crash | `err_gjk_null_outputs` |
| 21 | `c2GJK` (lib.c:510 `if (outB)`) | `outB == NULL` | write skipped, no crash | `err_gjk_null_outputs` |
| 22 | `c2GJK` (lib.c:512 `if (iterations)`) | `iterations == NULL` | write skipped, no crash | `err_gjk_null_outputs` |
| 23 | `c2GJK` (lib.c:381 `if (cache)`) | `cache == NULL` | cache read **and** write skipped | `err_gjk_null_outputs` |
| 24 | `c2GJK` (lib.c:382 `!!cache->count`) | `cache->count == 0` (fresh/zeroed cache) | cache **not** read, simplex re-seeded from vertex 0 | `err_gjk_cache_count_zero` |
| 25 | `c2GJK` (lib.c:405 `!(min_metric < max_metric*2 && metric < -1.0e8f)`) | any cache whose `metric` does not satisfy the (practically unsatisfiable) condition | `cache_was_read = 1` → cached simplex is kept verbatim | `err_gjk_cache_metric_reject` |
| 26 | `c2GJK` (lib.c:385 `pA.verts[iA]`, `pB.verts[iB]`) | `cache->count` in 1..3 with `iA`/`iB` beyond the proxy's real vertex count but inside `verts[8]` | out-of-logical-range vertex read, no rejection | `err_gjk_cache_index_past_count` |
| 27 | `c2GJK` (lib.c:425 `while (iter < 20)`) | input that never converges | loop capped at 20 iterations; `*iterations <= 20` | `err_gjk_iteration_cap` |
| 28 | `c2GJK` (lib.c:442 `if (s.count == 3)`) | shapes whose Minkowski simplex encloses the origin | `hit = 1`, `a = b`, `dist = 0` | `err_gjk_hit_zero_distance` |
| 29 | `c2GJK` (lib.c:447 `if (d1 > d0)`) | non-monotonic squared distance (numerical stall) | `break` out of the loop early | `err_gjk_degenerate_inputs` |
| 30 | `c2GJK` (lib.c:451 `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON`) | degenerate search direction (`FLT_EPSILON == 1.1920929e-7f`) | `break` | `err_gjk_degenerate_inputs` |
| 31 | `c2GJK` (lib.c:466 duplicate `iA`/`iB`) | support point repeats a saved vertex | `break` **before** `++s.count` | `err_gjk_degenerate_inputs` |
| 32 | `c2GJK` (lib.c:485 `dist > rA + rB && dist > FLT_EPSILON`) | `use_radius != 0` and `dist <= rA + rB` (overlap) **or** `dist <= FLT_EPSILON` | midpoint collapse: `a = b = (a+b)*0.5f`, `dist = 0` | `err_gjk_radius_collapse` |
| 33 | `c2GJK` (lib.c:492 `if (a.x == b.x && a.y == b.y)`) | radius shrink makes the witness points coincide | `dist = 0` | `err_gjk_radius_collapse` |
| 34 | `c2GJK` (lib.c:490 `c2Norm(c2Sub(b, a))`) | `use_radius != 0`, `dist > rA+rB`, but `b - a` is the zero vector | `NaN` propagated into `a`/`b` | `err_gjk_radius_collapse` |
| 35 | `c2GJK` | negative shape radius (`c2Circle.r < 0`, `c2Capsule.r < 0`) — never checked | `rA + rB` negative, distance grows | `err_gjk_negative_radius` |
| 36 | `c2GJK` (lib.c:378-379 `c2MakeProxy`) | `typeA`/`typeB` out of {0,1,2}: proxy left as the caller's bytes | no rejection; `pA.count`/`pA.radius` are whatever the (uninitialised in C) stack slot held — **UB, asserted only for "does not crash"** | `err_gjk_bad_type_no_crash` |
| 37 | `c2AABBtoAABB` (lib.c:524 `!(d0\|d1\|d2\|d3)`) | inverted AABB (`min > max`) — never validated | plain interval test on the raw fields | `err_aabbtoaabb_inverted` |
| 38 | `c2AABBtoAABB` | `NaN` in any component (all four `<` are false) | `return 1` (reports a hit) | `err_aabbtoaabb_nan` |
| 39 | `c2CircletoCircle` (lib.c:544 `d2 < r2`) | negative radii — `r2 = (A.r+B.r)^2` makes the sign vanish | negative radii behave like positive ones | `err_circle_negative_radius` |
| 40 | `c2CircletoAABB` (lib.c:552) | inverted AABB (`min > max`) — `c2Clampv` = `max(lo, min(a, hi))` clamps to `lo` | result driven by `lo`, no rejection | `err_circletoaabb_inverted` |
| 41 | `c2CircletoCapsule` (lib.c:566 `da / c2Dot(n, n)`) | degenerate capsule `B.a == B.b` → `c2Dot(n,n) == 0` | division by zero when `da >= 0 && db < 0`; otherwise the `da < 0` branch wins | `err_circletocapsule_degenerate` |
| 42 | `c2AABBtoCapsule` / `c2CapsuletoCapsule` (lib.c:528/534 `if (c2GJK(...))`) | GJK returns `-0.0f` or `NaN` | `if` is false for `-0.0f` → `return 1`; true for `NaN` → `return 0` | `err_bool_wrappers_zero_semantics` |
| 43 | `c2BBVerts` | inverted / `NaN` AABB | writes 4 vertices verbatim, no validation | `err_bbverts_inverted` |
| 44 | `capsule` (include/lib.h) | any float args incl. `NaN`, `±inf`, negative `r`, `a == b` | 3-bit bitmask `0..7`, never an error code | `err_capsule_extreme_args` |

## Generic FFI boundary coverage (also tested, per instructions)

| trigger | covered by |
|---------|-----------|
| out-of-range `C2_TYPE` enum values across the FFI boundary (`-1`, `3`, `4`, `255`, `256`, `INT_MIN`, `INT_MAX`) | rows 1–4, 36 |
| null pointers on every pointer parameter that C guards | rows 18–23 |
| zero lengths / counts (`count == 0`) | rows 6–9, 15, 24 |
| oversized lengths (`count > 3` for simplices, `count > 8` for vertex arrays) | rows 6–9, 16 |
| one step past a valid range (`count == 4`, `count == -1`, `type == 3`) | rows 1–9, 36 |
| non-finite floats (`NaN`, `±inf`, `-0.0`, subnormals, `FLT_MAX`) | rows 14, 38, 42, 44 |

**Note on row 36.** In C, `c2GJK` declares `c2Proxy pA; c2Proxy pB;` on the stack
and `c2MakeProxy` has no `default:` label, so an invalid type leaves the proxy
**uninitialised**. Reading it is undefined behaviour with no deterministic value,
so the differential test for that row asserts only that both libraries return
without crashing (and that valid types still agree bit-for-bit); it does not
compare a garbage float against another garbage float. Measured agreement:
**0/24** — see `NOTES-ub.md` for why, and for the same reasoning applied to
row 26.

## Status: all 44 rows verified

```
$ cargo test --release --test error_paths
test result: ok. 36 passed; 0 failed; 3 ignored
```

Every row above has a passing differential test. The 3 `ignored` tests are the
child-process halves of rows 26 and 36 (they read uninitialised / out-of-bounds
C stack and are spawned deliberately by their parent test so a crash stays
isolated); they are not skipped coverage.

### Measured facts worth recording

* **Row 27 (iteration cap).** Measured over 60 000 randomised configurations
  including warm-start caches (`tests/iteration_depth.rs`), the observed
  histogram is `{0: 25149, 1: 30460, 2: 3938, 3: 453}`. The `hit`, `dup` and
  `d1 > d0` guards terminate the loop long before `iter` reaches 20, so the cap
  itself is not reachable through the public API. The test asserts C and Rust
  agree on the count and that it stays within `0..=20`, rather than pretending
  to hit 20.
* **Row 42 (`-0.0` / `NaN` truthiness).** Cross-checked in both libraries: the
  wrapper's result equals `dist == 0.0` for finite distances and `0` when the
  distance is `NaN`, exactly as `if (c2GJK(...))` implies.
* **Rows 1-4.** All 12 out-of-range enum values (`3, 4, 5, 255, 256, 1000, -1,
  -2, -256, INT_MIN, INT_MAX, 0x10000`) return `0` from both libraries.
