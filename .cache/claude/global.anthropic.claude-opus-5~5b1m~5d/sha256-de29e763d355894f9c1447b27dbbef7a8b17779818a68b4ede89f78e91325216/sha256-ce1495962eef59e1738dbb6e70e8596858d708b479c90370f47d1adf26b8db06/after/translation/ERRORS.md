# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. The library has **no error enum,
no `RETURN_ERROR` macro, no `assert`, and no `errno`**: every rejection is
expressed as an early `return`, a `switch` `default:` label, a missing `switch`
label, a null-pointer guard, or a saturating/short-circuit numeric branch.
The grep basis is recorded per row.

`E-` rows are the enumerated rejections. `B-` rows are the generic FFI
boundaries required by Phase C (nulls, zero/oversized lengths, one-past-range
enum values).

| # | function | trigger (exact invalid input / condition) | expected C result |
|---|----------|-------------------------------------------|-------------------|
| E1 | `c2MakeProxy` | `type` is not 0/1/2 (e.g. `3`, `-1`, `999`, `INT_MIN`, `INT_MAX`) — `switch` has **no `default:`** | function writes **nothing**; `*p` left exactly as the caller had it (radius/count/verts unchanged) |
| E2 | `c2GJKSimplexMetric` | `s->count == 0` — hits `default:` which **falls through into `case 1:`** | returns `0.0f` |
| E3 | `c2GJKSimplexMetric` | `s->count == 1` | returns `0.0f` |
| E4 | `c2GJKSimplexMetric` | `s->count` ∉ {1,2,3} (4, -1, INT_MAX, INT_MIN) — `default:` | returns `0.0f` |
| E5 | `c2D` | `s->count == 3` (`case 3:` shares body with `default:`) | returns `c2V(0,0)` |
| E6 | `c2D` | `s->count` ∉ {1,2,3} (0, 4, -7, INT_MAX) — `default:` | returns `c2V(0,0)` |
| E7 | `c2L` | `s->count` ∉ {1,2} (0, 3, 4, -1, INT_MAX) — `default:` | returns `c2V(0,0)` (note: `den = 1/div` is still computed first, so a `div==0` cannot trap) |
| E8 | `c2Witness` | `s->count` ∉ {1,2,3} (0, 4, -1, INT_MAX) — `default:` | writes `*a = c2V(0,0)` and `*b = c2V(0,0)` |
| E9 | `c2Witness` / `c2L` | `s->div == 0.0f` → `den = 1.0f/0.0f` | `den = +inf`; results are `±inf`/`NaN` per IEEE-754, **no trap, no error return** |
| E10 | `c2Witness` / `c2L` | `s->div == -0.0f` | `den = -inf` |
| E11 | `c2Div` | `b == 0.0f` | `c2Mulvs(a, +inf)`; components become `±inf` or `NaN` (`0*inf`) |
| E12 | `c2Div` | `b == NaN` | both components `NaN` |
| E13 | `c2Norm` | `a == (0,0)` → `c2Len(a) == 0` → division by zero | `c2V(NaN, NaN)` (`0 * inf`) |
| E14 | `c2Len` | `c2Dot(a,a)` overflows to `+inf` (e.g. `a = (1e30, 1e30)`) | returns `+inf` |
| E15 | `c2Len` | any component `NaN` | returns `NaN` (`sqrtf` of NaN) |
| E16 | `c2Support` | `count <= 0` (`0`, `-1`, `INT_MIN`) — the `for` guard `i < count` never runs, but `verts[0]` **is still dereferenced** before it | returns `0` (must not be treated as an error; must not panic) |
| E17 | `c2Support` | `count == 1` | returns `0` |
| E18 | `c2Support` | all dots are `NaN` (`dot > dmax` always false) | returns `0` |
| E19 | `c2GJK` | `ax_ptr == NULL` | uses `c2xIdentity()` instead of dereferencing — no crash |
| E20 | `c2GJK` | `bx_ptr == NULL` | uses `c2xIdentity()` |
| E21 | `c2GJK` | `outA == NULL` | skips `*outA = a` |
| E22 | `c2GJK` | `outB == NULL` | skips `*outB = b` |
| E23 | `c2GJK` | `iterations == NULL` | skips `*iterations = iter` |
| E24 | `c2GJK` | `cache == NULL` | skips both the cache read **and** the cache write-back |
| E25 | `c2GJK` | `cache != NULL` **and** `cache->count == 0` → `cache_was_good == 0` | cache is **not** read; simplex is re-seeded from vertex 0; cache is still written back on exit |
| E26 | `c2GJK` | `cache != NULL`, `cache->count != 0`, and `!(min_metric < max_metric*2 && metric < -1e8f)` — true for essentially every finite metric | `cache_was_read = 1`, the stale cached simplex is used verbatim (this inverted/dead test is reproduced as-is) |
| E27 | `c2GJK` | cache-seeded simplex reaches `s.count == 3` on the very first `c23` call | `hit = 1`, loop breaks with `iter == 0`, `dist = 0`, `a = b` |
| E28 | `c2GJK` | `d1 > d0` (no progress / numeric regression) | `break` out of the loop early, `iter` < 20 |
| E29 | `c2GJK` | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (degenerate search direction) | `break` |
| E30 | `c2GJK` | new support point duplicates a saved one (`dup == 1`) | `break` **before** `++s.count`, so the freshly written `verts[s.count]` is left outside the simplex |
| E31 | `c2GJK` | 20 iterations elapse without termination | loop exits on `iter < 20`, `*iterations == 20` |
| E32 | `c2GJK` | `use_radius == 0` (and `hit == 0`) | **no** radius shrink: raw witness points and raw `dist` are returned, even when `dist < rA+rB` |
| E33 | `c2GJK` | `use_radius != 0`, `hit == 0`, and `dist <= rA + rB` (overlap within radii) | `a = b = midpoint(a,b)`, `dist = 0` |
| E34 | `c2GJK` | `use_radius != 0`, `hit == 0`, and `dist <= FLT_EPSILON` (coincident witnesses) | same midpoint branch, `dist = 0` (this is what stops `c2Norm` dividing by zero) |
| E35 | `c2GJK` | `use_radius != 0`, `dist > rA+rB`, but after shifting `a.x==b.x && a.y==b.y` | `dist` forced back to `0` |
| E36 | `c2GJK` | `hit != 0` | `a = b`, `dist = 0`, and the `use_radius` block is **skipped entirely** |
| E37 | `c2GJK` | negative radius shapes (`r < 0`) so that `rA + rB < 0` | no check exists: `dist -= rA+rB` **grows** the distance; must be reproduced |
| E38 | `c2Collided` | `typeA` ∉ {0,1,2} (`3`, `-1`, `INT_MAX`, `INT_MIN`) — outer `switch` `default:` | returns `0` **without dereferencing `A` or `B`** |
| E39 | `c2Collided` | `typeA == C2_TYPE_CIRCLE`, `typeB` ∉ {0,1,2} — inner `default:` | returns `0` |
| E40 | `c2Collided` | `typeA == C2_TYPE_AABB`, `typeB` ∉ {0,1,2} — inner `default:` | returns `0` |
| E41 | `c2Collided` | `typeA == C2_TYPE_CAPSULE`, `typeB` ∉ {0,1,2} — inner `default:` | returns `0` |
| E42 | `c2CircletoCircle` | `A.r + B.r` negative → `r2 = (A.r+B.r)^2` positive again | comparison uses the **squared** sum, so a negative radius behaves like its absolute value |
| E43 | `c2CircletoAABB` | `B.min > B.max` (inverted AABB) — `c2Clampv` = `max(lo, min(a,hi))` with no validation | clamp yields `lo`; result is whatever that produces (no rejection) |
| E44 | `c2CircletoCapsule` | `B.a == B.b` (zero-length capsule) → `c2Dot(n,n) == 0` | `da == 0` so `da < 0` is false; `db == 0` so `db < 0` is false → the `bp` branch is taken; the `da/dot(n,n)` division is **not** reached |
| E45 | `c2CircletoCapsule` | `A.r + B.r` negative | `d2 < r*r` compares against the square, negative radius behaves as its magnitude |
| E46 | `c2AABBtoCapsule` / `c2CapsuletoCapsule` | `c2GJK` returns a non-zero `dist` (including `NaN`: `if (NaN) → false`) | non-zero → `0`; `NaN` is **falsy** in C so `NaN` returns `1` |
| E47 | `capsule` | `min_*`/`max_*`/`r` are `NaN` or `±inf` | no validation whatsoever; the three `c2Collided` results are combined as `b0 + (b1<<1) + (b2<<2)` |
| B1 | `c2BBVerts` | `out`/`bb` non-null but aliasing / `bb->min > bb->max` | no validation; the four corner writes happen unconditionally |
| B2 | `c2Support` | `count` larger than the real array (oversized length) | reads past the array; **undefined behaviour in C** — not differentially testable, so the test only exercises `count` ≤ the allocated length (documented, not asserted) |
| B3 | `c2GJK` | `cache->count > 3` | indexes `cache->iA[3]`, `saveA[3]` and `verts[3]` out of bounds → **UB in C** (writes past a 3-element stack array). Not differentially testable; the tests confine `cache->count` to `0..=3` |
| B4 | `c2GJK` | `cache->count < 0` | the seeding `for` loop body never executes, then `s.count = cache->count` (negative) → `c2GJKSimplexMetric` `default:` → 0, `c2Witness` `default:` → `(0,0)`; the main loop's copy loop also does nothing |
| B5 | `c2MakeProxy` / `c2Collided` / `c2GJK` | `shape`/`A`/`B` `NULL` **with a valid type** | dereferences NULL → **SIGSEGV in C** (no guard). Not differentially testable; only the guarded paths (E38–E41, where the pointer is never read) are exercised with NULL |
| B6 | all `c2*` taking a `c2Simplex*`/`c2v*` | NULL pointer | dereferenced unguarded → SIGSEGV in C; documented, not exercised |

## Rejection-mechanism inventory (grep basis)

```
switch (…) { … }  with NO default:   -> c2MakeProxy                        (E1)
default: labels                      -> c2GJKSimplexMetric, c2D, c2L,
                                        c2Witness, c2Collided x4           (E2..E8, E38..E41)
if (!ptr) / if (ptr)  null guards    -> c2GJK ax_ptr, bx_ptr, outA, outB,
                                        iterations, cache                  (E19..E25)
early `break` in the GJK loop        -> count==3, d1>d0, tiny d, dup, iter (E27..E31)
`if (use_radius)` / else midpoint    -> c2GJK radius handling              (E32..E36)
implicit "truthiness" rejection      -> `if (c2GJK(...)) return 0;`        (E46)
division with no zero check          -> c2Div, c2Norm, c2Witness, c2L,
                                        c2CircletoCapsule                  (E9..E13, E44)
min/max constants                    -> FLT_MAX seeds for d0/d1,
                                        FLT_EPSILON, FLT_EPSILON^2, -1e8f  (E14, E29, E26, E34)
```

There are **no** `assert`, `return -1`, `return NULL`, `abort`, or error-enum
statements anywhere in `c_src/src/lib.c`.

---

## Phase C status — every row has a passing differential test

Test binary: `tests/phase_c_errors.rs` (plus `tests/e31_search.rs` for E31).
Run with `cargo test`; all rows verified in the dev **and** release profile.

| row | test function | [x] |
|-----|---------------|-----|
| E1 | `e1_makeproxy_invalid_type_writes_nothing` | [x] |
| E2 | `e2_e3_e4_simplex_metric_bad_count` | [x] |
| E3 | `e2_e3_e4_simplex_metric_bad_count` | [x] |
| E4 | `e2_e3_e4_simplex_metric_bad_count` | [x] |
| E5 | `e5_e6_c2d_bad_count` | [x] |
| E6 | `e5_e6_c2d_bad_count` | [x] |
| E7 | `e7_e9_e10_c2l_bad_count_and_zero_div` | [x] |
| E8 | `e8_e9_e10_witness_bad_count_and_zero_div` | [x] |
| E9 | `e7_e9_e10_c2l_bad_count_and_zero_div`, `e8_e9_e10_witness_bad_count_and_zero_div` | [x] |
| E10 | same as E9 (`div == -0.0f`) | [x] |
| E11 | `e11_to_e15_div_norm_len_edges` | [x] |
| E12 | `e11_to_e15_div_norm_len_edges` | [x] |
| E13 | `e11_to_e15_div_norm_len_edges` (asserts NaN, not just equality) | [x] |
| E14 | `e11_to_e15_div_norm_len_edges` (asserts `+inf`) | [x] |
| E15 | `e11_to_e15_div_norm_len_edges` (asserts NaN; incl. a signalling NaN input) | [x] |
| E16 | `e16_e17_e18_support_bad_count` (asserts the sentinel `0`) | [x] |
| E17 | `e16_e17_e18_support_bad_count` | [x] |
| E18 | `e16_e17_e18_support_bad_count` | [x] |
| E19 | `e19_to_e25_gjk_null_guards` (NULL == explicit identity) | [x] |
| E20 | `e19_to_e25_gjk_null_guards` | [x] |
| E21 | `e19_to_e25_gjk_null_guards` (also asserts the NULL target is untouched) | [x] |
| E22 | `e19_to_e25_gjk_null_guards` | [x] |
| E23 | `e19_to_e25_gjk_null_guards` (also asserts the NULL target is untouched) | [x] |
| E24 | `e19_to_e25_gjk_null_guards` | [x] |
| E25 | `e19_to_e25_gjk_null_guards` (asserts the cache *is* written back) | [x] |
| E26 | `e26_e27_stale_cache_is_accepted` (incl. `metric = -1e9`) | [x] |
| E27 | `e26_e27_stale_cache_is_accepted` (asserts the immediate `iter==0, dist==0` hit was observed) | [x] |
| E28 | `e28_to_e31_loop_termination` (iteration histogram proves >=3 exit routes) | [x] |
| E29 | `e29_degenerate_direction` (coincident / 1-ULP-apart shapes) | [x] |
| E30 | `e28_to_e31_loop_termination` (the dominant exit for 1/2-vertex proxies) | [x] |
| E31 | `e31_iteration_limit_search` — 400 000 randomized configurations across all shape kinds, transforms, caches and `use_radius`; C and Rust agree on `*iterations` for every one, and neither ever exceeds the bound. **Observed maximum is 4**: with proxies of at most 4 vertices and a strictly decreasing `d0`, `iter == 20` is unreachable, so the saturation value itself cannot be constructed. The bound is verified (`0 <= iter <= 20`, identical in both libraries) rather than saturated. | [x] |
| E32 | `e32_to_e37_radius_branches` (`use_radius = 0` raw output compared separately) | [x] |
| E33 | `e32_to_e37_radius_branches` (midpoint branch counter > 0) | [x] |
| E34 | `e32_to_e37_radius_branches`, `e29_degenerate_direction` | [x] |
| E35 | `e35_shrink_collapses_to_zero` — 3 696 observed collapses at huge magnitudes | [x] |
| E36 | `e32_to_e37_radius_branches` (hit counter > 0) | [x] |
| E37 | `e32_to_e37_radius_branches` (both radii negative) | [x] |
| E38 | `e38_to_e41_collided_bad_enums` (13 out-of-range enum values incl. `INT_MIN`/`INT_MAX`, with NULL shapes) | [x] |
| E39 | `e38_to_e41_collided_bad_enums` | [x] |
| E40 | `e38_to_e41_collided_bad_enums` | [x] |
| E41 | `e38_to_e41_collided_bad_enums` | [x] |
| E42 | `e42_e45_negative_radii` | [x] |
| E43 | `e43_inverted_aabb` | [x] |
| E44 | `e44_zero_length_capsule` | [x] |
| E45 | `e42_e45_negative_radii` | [x] |
| E46 | `e46_gjk_truthiness_rejection` — reproduces the C rule `if (dist) return 0; else return 1;` exactly for 40 000 wild inputs and asserts BOTH outcomes occur (33 610 x `return 1`, 6 390 x `return 0`). **Correction to the row text:** a NaN distance is produced 29 599 times with `use_radius = 0`, but with `use_radius = 1` (which is what the two wrappers pass) the `dist > rA+rB` test is false for NaN, so the midpoint branch always clamps it to `0`; the wrappers therefore never observe NaN. The test asserts this invariant (`nan_ur1 == 0`) so the claim cannot silently rot. | [x] |
| E47 | `e47_capsule_entry_no_validation` (17^3 x 3 special-value argument tuples) | [x] |
| B1 | `b1_bbverts_no_validation` | [x] |
| B4 | `b4_negative_cache_count` | [x] |
| B2 | Not exercised: `count` beyond the array is out-of-bounds reads, i.e. **undefined behaviour in C**, so there is no defined C result to be differential against. Documented only. | n/a |
| B3 | Not exercised: `cache->count > 3` writes past `saveA[3]`/`verts[3]`/`iA[3]` — **UB in C**. Tests confine `cache->count` to `-N..=3`. Documented only. | n/a |
| B5 | Not exercised with a valid type: NULL shape + valid type is a NULL dereference (SIGSEGV) in C. The *guarded* NULL paths (invalid type, where the pointer is never read) ARE exercised in `e1_...` and `e38_to_e41_...`. | n/a |
| B6 | Not exercised: NULL `c2Simplex*`/`c2v*` is an unguarded NULL dereference in C. Documented only. | n/a |
