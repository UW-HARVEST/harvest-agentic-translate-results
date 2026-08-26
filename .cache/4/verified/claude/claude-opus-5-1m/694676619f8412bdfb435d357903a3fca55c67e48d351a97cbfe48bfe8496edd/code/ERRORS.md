# ERRORS.md — error / rejection surface table (Phase A, gate for Phase C)

Derived mechanically from `c_src/src/lib.c` (645 lines, the only translation
unit). The library has **no error enum, no `errno`, no `assert`, no
`return NULL`, no `RETURN_ERROR` macro and no negative error codes**:

```
$ grep -cE 'assert|return -1|return NULL|RETURN_ERROR|errno' c_src/src/lib.c
0
```

Its rejection surface is therefore made up of *guards and sentinel results*:
null-pointer guards, `switch` fall-throughs / missing `default:` labels
(out-of-range enum and out-of-range `count`), explicit comparisons against
`0`, `FLT_EPSILON`, `-1.0e8f`, the hard-coded 20-iteration cap, and
division-by-zero producing `inf`/`NaN`. Every one of those is enumerated below,
one row per distinct branch, with the line number in `c_src/src/lib.c`.

Tests: `tests/phase_c_errors.rs`. Each row's test is named `e<NN>_...`.

| #  | function (line) | trigger (the exact invalid input / condition) | expected C result | test |
|----|-----------------|-----------------------------------------------|-------------------|------|
| 01 | `c2MakeProxy` (113) | `type` ∉ {0,1,2} — `switch` has **no `default:`** | returns without touching `*p`; caller's buffer is left byte-for-byte unchanged (`radius`, `count`, `verts` all keep their prior values) | `e01_makeproxy_out_of_range_enum` |
| 02 | `c2MakeProxy` (112) | `p == NULL` with a *valid* type | dereferences NULL → SIGSEGV. Documented, **not** differentially executed (both sides crash the process). | `e02_makeproxy_null_proxy_documented` |
| 03 | `c2GJKSimplexMetric` (161–163) | `s->count` ∉ {2,3} (`default:` falls through into `case 1:`) — incl. `0`, `1`, `4`, negative | returns `0.0f` (`+0.0`, bit pattern `0x00000000`) | `e03_simplexmetric_bad_count` |
| 04 | `c22` (190) | `v <= 0` (incl. `v == -0.0`, `v == NaN`→false) | collapse onto `a`: `a.u = 1`, `div = 1`, `count = 1`; `b` untouched | `e04_e05_e06_c22_all_branches` |
| 05 | `c22` (194) | `v > 0 && u <= 0` | collapse onto `b`: `a = b`, `a.u = 1`, `div = 1`, `count = 1` | `e04_e05_e06_c22_all_branches` |
| 06 | `c22` (199) | both `u > 0` and `v > 0` (also the NaN case: `NaN <= 0` is false) | `a.u=u`, `b.u=v`, `div=u+v`, `count=2` | `e04_e05_e06_c22_all_branches` |
| 07 | `c23` (221) | `vAB <= 0 && uCA <= 0` | vertex-`a` region: `a.u=1`, `div=1`, `count=1` | `e07_to_e14_c23_all_branches` |
| 08 | `c23` (225) | `uAB <= 0 && vBC <= 0` | vertex-`b` region: `a=b`, `a.u=1`, `div=1`, `count=1` | `e07_to_e14_c23_all_branches` |
| 09 | `c23` (230) | `uBC <= 0 && vCA <= 0` | vertex-`c` region: `a=c`, `a.u=1`, `div=1`, `count=1` | `e07_to_e14_c23_all_branches` |
| 10 | `c23` (235) | `uAB>0 && vAB>0 && wABC<=0` | edge AB, `count=2` | `e07_to_e14_c23_all_branches` |
| 11 | `c23` (240) | `uBC>0 && vBC>0 && uABC<=0` | edge BC (`a=b; b=c`), `count=2` | `e07_to_e14_c23_all_branches` |
| 12 | `c23` (247) | `uCA>0 && vCA>0 && vABC<=0` | edge CA (`b=a; a=c`), `count=2` | `e07_to_e14_c23_all_branches` |
| 13 | `c23` (254) | none of the above (interior, or every barycentric test defeated by NaN) | `count=3`, `div = uABC+vABC+wABC` (may be `0`, `NaN`, `±inf`) | `e07_to_e14_c23_all_branches` |
| 14 | `c23` (217–220) | degenerate triangle: `area == 0` (collinear / duplicated points) ⇒ `uABC = vABC = wABC = ±0` ⇒ all three `<= 0` tests true | falls into whichever earlier branch matches; if it reaches the `else`, `div = 0` | `e07_to_e14_c23_all_branches` |
| 15 | `c2D` (283) | `s->count == 1` | `-a.p` (note: `-0.0` for zero components) | `e15_c2D_count1` |
| 16 | `c2D` (287) | `count == 2`, `c2Det2(ab, -a.p) > 0` false (incl. `== 0` and `NaN`) | `c2CCW90(ab)` instead of `c2Skew(ab)` | `e16_c2D_count2_det_not_positive` |
| 17 | `c2D` (291–293) | `count` ∉ {1,2} (`case 3:` + `default:`) incl. `0`, `4`, negative | `(0,0)` | `e17_c2D_bad_count` |
| 18 | `c2Support` (300) | `count <= 0` (`0`, `-1`, `INT_MIN`) — loop never runs but `verts[0]` **is** dereferenced first | returns `0` | `e18_support_nonpositive_count` |
| 19 | `c2Support` (302) | all dots equal, or every dot is `NaN` (`dot > dmax` always false) | returns `0` (first index wins) | `e19_support_all_ties_or_nan`, `e19_boundary_support_first_vertex_never_beats_itself` |
| 20 | `c2Witness` (311) | `s->div == 0` ⇒ `den = 1/0 = +inf` (or `-0.0` ⇒ `-inf`) | outputs contain `±inf`/`NaN` (`inf*0 = NaN`) — bit-exact match required | `e20_witness_div_zero` |
| 21 | `c2Witness` (331–333) | `count` ∉ {1,2,3} (`default:`) incl. `0`, `4`, negative | `*a = *b = (0,0)` | `e21_witness_bad_count` |
| 22 | `c2Div` (338) | `b == 0` → `1.0f/0` = `±inf`; component `0 * inf` = `NaN` | `±inf` / `NaN` components, sign from the zero's sign | `e22_div_by_zero` |
| 23 | `c2Div` (338) | `b == NaN` / `b == ±inf` | `NaN` / `±0` components | `e23_div_nan_inf` |
| 24 | `c2Norm` (342) | zero-length vector `(0,0)` → `c2Len = 0` → `c2Div(a,0)` | `(NaN, NaN)` | `e24_norm_zero_vector` |
| 25 | `c2Norm` (342) | vector with a `NaN`/`inf` component | `NaN`/`±0` components | `e25_norm_nan_inf` |
| 26 | `c2L` (346) | `div == 0` ⇒ `den = inf` | `±inf`/`NaN` components | `e26_cL_div_zero` |
| 27 | `c2L` (353–354) | `count` ∉ {1,2} (`default:`, incl. `3`, `4`, `0`, negative) | `(0,0)` | `e27_cL_bad_count` |
| 28 | `c2Len` (152) | `c2Dot(a,a) < 0` — only reachable via `inf`/`NaN` mixtures ⇒ `sqrtf(NaN)` | `NaN` | `e28_len_nan` |
| 29 | `c2GJK` (367) | `ax_ptr == NULL` | uses `c2xIdentity()` for A — result identical to passing `&c2xIdentity()` | `e29_gjk_null_ax` |
| 30 | `c2GJK` (371) | `bx_ptr == NULL` | uses `c2xIdentity()` for B | `e30_gjk_null_bx` |
| 31 | `c2GJK` (383) | `cache != NULL` but `cache->count == 0` | `cache_was_good = 0` ⇒ warm start skipped, fresh 1-simplex built | `e31_gjk_cache_count_zero` |
| 32 | `c2GJK` (404) | `!(min_metric < max_metric*2 && metric < -1.0e8f)` — because `metric < -1.0e8f` is essentially never true this is **always** taken when `count != 0`, i.e. the cache is always accepted | `cache_was_read = 1` ⇒ the primed simplex is used verbatim, `s.count`/`s.div` come from the cache | `e32_gjk_cache_always_read` |
| 33 | `c2GJK` (385–397) | `cache->count` in 1..3 with `iA[i]`/`iB[i]` **outside `0..proxy->count`** (but still inside `verts[8]`) | `c2Proxy pA` is an *uninitialised* stack object (`c2GJK:375`) and `c2MakeProxy` only writes `verts[0..count)`, so this reads uninitialised stack memory. The C result is not a function of its inputs. Undefined behaviour; documented, **not** differentially executed. Indices inside `0..proxy->count` *are* exercised (rows 32, 47 of `CONFIGS.md`). | `e33_gjk_cache_index_out_of_shape_range_documented` |
| 34 | `c2GJK` (385) | `cache->count > 3` (e.g. 4, 5, `INT_MAX`) or `< 0` | reads past `cache->iA[3]` **and writes `verts[4]`**, i.e. past the end of `c2Simplex` → stack corruption. Undefined behaviour; documented, **not** differentially executed. | `e34_gjk_cache_count_gt3_documented` |
| 35 | `c2GJK` (424) | 20-iteration cap reached (`iter == 20`) | loop exits with whatever simplex it has; `*iterations == 20`. **Note:** the cap is *unreachable* — the proxies have at most 4 vertices, so the duplicate-support break (line 464) or the `d1 > d0` break always fires first. Verified by a 108 000-case randomized search (9 type pairs x 12 000, arbitrary bit-pattern coordinates incl. `NaN`/`inf`, denormals, `FLT_MAX`): the maximum `*iterations` ever observed is **5** (histogram printed by the test), so the cap is unreachable. The guard is therefore verified by asserting C and Rust return the **same `*iterations`** for every one of those cases. | `e35_gjk_iteration_cap` |
| 36 | `c2GJK` (440) | `s.count == 3` after `c23` | `hit = 1` ⇒ `a = b` and `dist = 0` (even when `use_radius == 0`) | `e36_gjk_hit` |
| 37 | `c2GJK` (446) | `d1 > d0` (no progress / numerical regress) | `break` out of the loop early | `e37_gjk_no_progress_break` |
| 38 | `c2GJK` (450) | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (degenerate search direction, e.g. identical shapes) | `break` | `e38_gjk_degenerate_direction_break` |
| 39 | `c2GJK` (464) | new support point duplicates a saved one (`iA==saveA[i] && iB==saveB[i]`) | `break` **before** `++s.count`, so the freshly written `verts[s.count]` is left dangling | `e39_gjk_duplicate_support_break` |
| 40 | `c2GJK` (481) | `use_radius == 0` | radius shrink skipped entirely — raw `dist` returned, `a`/`b` are the witness points | `e40_gjk_use_radius_zero` |
| 41 | `c2GJK` (481) | `use_radius` = any non-zero `int` (`2`, `-1`, `INT_MIN`, `INT_MAX`) | identical to `use_radius == 1` | `e41_gjk_use_radius_nonzero_variants` |
| 42 | `c2GJK` (484) | `use_radius != 0` and **not** (`dist > rA+rB && dist > FLT_EPSILON`) — i.e. shapes closer than the radius sum, or `dist` `NaN` | `a = b = midpoint(a,b)`, `dist = 0` | `e42_gjk_radius_else_midpoint` |
| 43 | `c2GJK` (484) | `dist > rA+rB` where `rA+rB` is `NaN` (radius `NaN`) ⇒ comparison false | midpoint branch | `e43_gjk_radius_nan` |
| 44 | `c2GJK` (490) | after the radius shrink `a.x==b.x && a.y==b.y` | `dist` forced to `0` although the subtraction produced non-zero | `e44_gjk_radius_shrink_collapses` |
| 45 | `c2GJK` (487) | radius branch where `c2Norm(b-a)` degenerates: `c2Dot` overflows so `c2Len` = `+inf` ⇒ `n = (±0,±0)` and `a`/`b` are left unmoved while `dist` stays `inf`. (The literal `b == a` case is *unreachable*: `a == b` implies `dist == 0`, which fails the `dist > FLT_EPSILON` guard at line 484, so the `else` branch is taken instead.) | `n` becomes `±0`, `a`/`b` unchanged, `dist == +inf` | `e45_gjk_radius_norm_overflow` |
| 46 | `c2GJK` (499) | `cache == NULL` | cache write-back skipped — caller's buffer untouched | `e46_gjk_null_cache_no_writeback` |
| 47 | `c2GJK` (509) | `outA == NULL` | `*outA` not written (caller's poison value preserved) | `e47_e48_e49_gjk_null_out_params` |
| 48 | `c2GJK` (511) | `outB == NULL` | `*outB` not written | `e47_e48_e49_gjk_null_out_params` |
| 49 | `c2GJK` (513) | `iterations == NULL` | `*iterations` not written | `e47_e48_e49_gjk_null_out_params` |
| 50 | `c2GJK` (377) | `typeA`/`typeB` ∉ {0,1,2} | `c2MakeProxy` leaves the **uninitialised stack** `c2Proxy` alone; `pA.count`/`pA.verts` are garbage ⇒ UB (`c2Support` may loop over a garbage count). Documented, **not** differentially executed. | `e50_gjk_bad_type_documented` |
| 51 | `c2AABBtoAABB` (519) | `B.max.x < A.min.x` (separating axis −X) | `0` | `e51_aabbaabb_sep_axes` |
| 52 | `c2AABBtoAABB` (520) | `A.max.x < B.min.x` (separating axis +X) | `0` | `e51_aabbaabb_sep_axes` |
| 53 | `c2AABBtoAABB` (521) | `B.max.y < A.min.y` (separating axis −Y) | `0` | `e51_aabbaabb_sep_axes` |
| 54 | `c2AABBtoAABB` (522) | `A.max.y < B.min.y` (separating axis +Y) | `0` | `e51_aabbaabb_sep_axes` |
| 55 | `c2AABBtoAABB` (519–523) | any coordinate `NaN` ⇒ all four `<` false | `1` ("collided") — a NaN box collides with everything | `e55_aabbaabb_nan_reports_hit` |
| 56 | `c2AABBtoCapsule` (527) | `c2GJK(...) != 0` (as a `float`→bool test; `NaN` counts as *true*) | `0` | `e56_aabbcapsule_reject` |
| 57 | `c2CapsuletoCapsule` (533) | `c2GJK(...) != 0` | `0` | `e57_capsulecapsule_reject` |
| 58 | `c2CircletoCircle` (543) | `d2 < r2` false — includes exact tangency `d2 == r2` (touching ⇒ **not** collided) and any `NaN` | `0` | `e58_circlecircle_reject_and_tangent` |
| 59 | `c2CircletoAABB` (551) | `d2 < r2` false — includes `r == 0` (`r2 == 0`, never `<`), tangency, inverted box, `NaN` | `0` | `e59_circleaabb_reject` |
| 60 | `c2CircletoCapsule` (559) | `da < 0` | distance measured to endpoint `a` | `e60_to_e63_circlecapsule_branches`, `e60_boundary_da_exactly_zero`, `e60_proof_da_zero_branches_coincide` |
| 61 | `c2CircletoCapsule` (563) | `da >= 0 && db < 0` — and `c2Dot(n,n) == 0` (degenerate capsule `a == b`) makes `da/0` = `±inf`/`NaN` | `d2` becomes `NaN`/`inf` ⇒ comparison false ⇒ `0` | `e60_to_e63_circlecapsule_branches` |
| 62 | `c2CircletoCapsule` (566) | `da >= 0 && db >= 0` | distance measured to endpoint `b` | `e60_to_e63_circlecapsule_branches` |
| 63 | `c2CircletoCapsule` (572) | `d2 < r*r` false | `0` | `e60_to_e63_circlecapsule_branches` |
| 64 | `c2Collided` (585) | `typeA == CIRCLE`, `typeB` ∉ {0,1,2} (`default:`) | `0` | `e64_collided_bad_typeB` |
| 65 | `c2Collided` (597) | `typeA == AABB`, `typeB` ∉ {0,1,2} | `0` | `e64_collided_bad_typeB` |
| 66 | `c2Collided` (609) | `typeA == CAPSULE`, `typeB` ∉ {0,1,2} | `0` | `e64_collided_bad_typeB` |
| 67 | `c2Collided` (613) | `typeA` ∉ {0,1,2} (outer `default:`) — `typeB` never even inspected | `0` | `e67_collided_bad_typeA` |
| 68 | `c2Collided` (575) | `A == NULL` / `B == NULL` with a *valid* type pair | NULL dereference → SIGSEGV. Documented, **not** differentially executed. | `e68_collided_null_shape_documented` |
| 69 | `c2Collided` (575) | `A == NULL` / `B == NULL` / both NULL, with an **invalid** `typeA` *or* an invalid `typeB` — every dereference lives inside a matched `case` arm, so a `default:` return happens before either pointer is touched | returns `0` safely | `e69_collided_null_with_bad_type` |
| 70 | `c2BBVerts` (105) | `bb->min`/`max` containing `NaN`/`inf`, or an inverted box | copies verbatim, no validation; `out[0..4)` written | `e70_bbverts_no_validation` |
| 71 | `aabb` (618) | `min_x..max_y` containing `NaN`, `±inf`, `FLT_MAX`, inverted box | still returns a 3-bit mask in `0..=7`; no rejection path exists | `e71_aabb_entry_extreme_inputs` |

## Generic FFI boundary cases (covered even though the C has no explicit check)

| # | case | covered by |
|---|------|------------|
| G1 | out-of-range enum across FFI (`3`, `4`, `7`, `100`, `INT_MAX`, `(unsigned)-1`) into `c2Collided` and `c2MakeProxy` | rows 01, 64–67, 69 |
| G2 | every nullable pointer parameter passed as `NULL` (`ax_ptr`, `bx_ptr`, `outA`, `outB`, `iterations`, `cache`) — all 64 combinations | rows 29, 30, 46–49 + `e_all_null_combinations` |
| G3 | zero length / count (`c2Support(count=0)`, `cache->count=0`, `simplex.count=0`) | rows 18, 31, 03, 17, 21, 27 |
| G4 | oversized / negative length (`c2Support(count<0)`, `simplex.count=4`, `count=INT_MIN`) | rows 18, 03, 17, 21, 27 |
| G5 | one step past a valid range (`simplex.count = 4`, `C2_TYPE = 3`) | rows 03, 17, 21, 27, 64–67 |
| G6 | `±0.0` distinguished from `0.0` in every float result (bit comparison) | all rows — `assert_same` compares `to_bits()` |
| G7 | `NaN` sign + payload preserved identically | rows 13, 20, 22–28, 43, 45, 55, 58, 61 |

## Deliberately *not* executed (undefined behaviour in the C itself)

Rows **02**, **33**, **34**, **50** and **68** describe inputs for which the C
reference performs a NULL dereference, an out-of-bounds stack write, or reads
uninitialised stack memory. There is no "correct" observable behaviour to
compare against (the C result is not a function of its inputs), so those rows
are asserted by *documentation + a compile-time-checked `#[ignore]`d test* rather
than by executing both libraries and diffing. Every other row is executed
against both `.so`s.

## Result

All **71 rows** plus the 7 generic FFI-boundary cases are covered by
`tests/phase_c_errors.rs` and **all pass**:

```
$ cargo build && cargo test --test phase_c_errors
test result: ok. 50 passed; 0 failed; 5 ignored
```

The 5 `ignored` tests are exactly rows **02, 33, 34, 50, 68** — the inputs for
which the C reference itself has undefined behaviour (NULL dereference,
out-of-bounds stack write, read of uninitialised stack memory). They are
compiled (so the calls stay type-checked and the reasoning stays visible) but not
executed, because the C's result there is not a function of its inputs. Every
other row is executed against both `.so`s and asserted bit-equal.

Branch-coverage diagnostics printed by the tests (`-- --nocapture`):

```
e04/05/06 c22 branch hits: [10486, 9240, 40274]
e07..e13  c23 branch hits: [11837, 11105, 10401, 48172, 43447, 39029, 36009]
e14       zero-area hits : [3083, 2373, 1700, 8944, 4536, 0, 0]
e16 c2D count=2: skew=11487 ccw90=28513
e32: warm start changed the outcome in 17094/24000 cases
e35: max iterations observed = 5; histogram = [49054, 31251, 22186, 5391, 111, 7, 0, ...]
e36: `hit` branch taken 16000 times
e37: iteration histogram = [9219, 14472, 3025, 284, 0, ...]
e56: rejected=15243 accepted=4757
e57: rejected=13579 accepted=6421
e60..e63 circle/capsule branch hits: [29117, 14680, 16203], rejects=52616
e60 proof: 316000 inputs with da == ±0 checked (228357 interior, 87643 endpoint b)
```

Every `c22` branch (3/3) and every `c23` branch (7/7) is hit thousands of times,
both `c2D` sub-branches are hit, the cache-read branch demonstrably changes the
outcome, and both accept and reject outcomes occur for every boolean wrapper —
so these are real branch exercises, not accidental single-path calls.

Test-strength evidence: `MUTATION_NOTES.md`.
