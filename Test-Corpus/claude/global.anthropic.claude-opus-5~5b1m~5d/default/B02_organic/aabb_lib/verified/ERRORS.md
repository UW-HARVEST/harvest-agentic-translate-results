# ERRORS.md — error / rejection surface table (Phase C)

Derived **mechanically** from `c_src/src/lib.c` by grepping every
`return <sentinel>`, every `default:` label, every null-pointer test, every
`switch` fall-through with no matching `case`, every comparison that rejects a
candidate (`<= 0`, `< 0`, `> 0`, `> d0`, `< eps*eps`), every early `break`, and
every hard bound (`iter < 20`, `verts[8]`, `iA[3]`/`iB[3]`).

The library has **no** error enum, no `errno`, no `assert`, and no
`RETURN_ERROR` macro. Its rejection vocabulary is:

* `int` 0/1 sentinels (`c2Collided`, `c2*to*`),
* "do nothing" (unmatched `switch` case in `c2MakeProxy`, null out-pointers in
  `c2GJK`),
* substitute-a-default-value (`return 0`, `return c2V(0,0)` in the simplex
  helpers' `default:` arms; `c2xIdentity()` for a null transform),
* loop abandonment (the six `break`s in the GJK main loop + the `iter < 20` cap),
* IEEE-754 non-finite results (division by a zero / non-finite denominator).

Row status: `[x]` = differential test written **and passing** against both `.so`s.
Test file: `tests/phase_c_errors.rs` (test-function name in the last column).

| # | function | trigger (exact invalid input / condition) | expected C result | status | test |
|---|----------|-------------------------------------------|-------------------|--------|------|
| 1 | `c2Collided` | `typeA` not in {0,1,2} (e.g. `3`, `-1`, `INT_MIN`, `INT_MAX`), any `typeB` | outer `default:` → returns `0`; shapes never dereferenced | [x] | `err_collided_bad_typeA` |
| 2 | `c2Collided` | `typeA == C2_TYPE_CIRCLE` (0), `typeB` not in {0,1,2} | inner `default:` → returns `0` | [x] | `err_collided_bad_typeB_circle` |
| 3 | `c2Collided` | `typeA == C2_TYPE_AABB` (1), `typeB` not in {0,1,2} | inner `default:` → returns `0` | [x] | `err_collided_bad_typeB_aabb` |
| 4 | `c2Collided` | `typeA == C2_TYPE_CAPSULE` (2), `typeB` not in {0,1,2} | inner `default:` → returns `0` | [x] | `err_collided_bad_typeB_capsule` |
| 5 | `c2MakeProxy` | `type` not in {0,1,2} — the `switch` has **no** `default:` | `*p` is left **completely unmodified** (no radius/count/verts write) | [x] | `err_makeproxy_bad_type_leaves_proxy_untouched` |
| 6 | `c2GJK` | `ax_ptr == NULL` | `ax = c2xIdentity()` is substituted (`if (!ax_ptr)`) | [x] | `err_gjk_null_ax` |
| 7 | `c2GJK` | `bx_ptr == NULL` | `bx = c2xIdentity()` is substituted (`if (!bx_ptr)`) | [x] | `err_gjk_null_bx` |
| 8 | `c2GJK` | `outA == NULL` | the `if (outA)` guard skips the store; return value unaffected | [x] | `err_gjk_null_outputs` |
| 9 | `c2GJK` | `outB == NULL` | the `if (outB)` guard skips the store | [x] | `err_gjk_null_outputs` |
| 10 | `c2GJK` | `iterations == NULL` | the `if (iterations)` guard skips the store | [x] | `err_gjk_null_outputs` |
| 11 | `c2GJK` | `cache == NULL` | both `if (cache)` blocks skipped: no read, no write-back | [x] | `err_gjk_null_outputs` |
| 12 | `c2GJK` | `cache != NULL` but `cache->count == 0` | `cache_was_good = !!0 = 0` → cache **rejected**, simplex re-seeded from vertex 0; cache is still written back on exit | [x] | `err_gjk_cache_count_zero_rejected` |
| 13 | `c2GJK` | cache accepted but stale: `!(min_metric < max_metric*2 && metric < -1.0e8f)` — because of the (quirky, always-false for finite metrics) `metric < -1.0e8f` conjunct this is **true for every finite metric**, so a non-empty cache is *always* "read" | `cache_was_read = 1`; simplex restored from cache, **not** re-seeded | [x] | `err_gjk_cache_reuse_always_accepted` |
| 14 | `c2GJK` | cache with `metric = NaN` (and/or `±inf`) | `min_metric`/`max_metric` pick via `?:` with NaN ⇒ both take the *else* operand; `metric < -1.0e8f` still false ⇒ `cache_was_read = 1` | [x] | `err_gjk_cache_nonfinite_metric` |
| 15 | `c2GJK` | `use_radius == 0` and shapes are radius-bearing (circle/capsule) | radius shrink block skipped entirely: raw core distance returned, `outA`/`outB` un-adjusted | [x] | `err_gjk_use_radius_zero` |
| 16 | `c2GJK` | `use_radius != 0` (any non-zero int, incl. negative / `2` / `INT_MIN`) | truthiness test only — identical to `use_radius == 1` | [x] | `err_gjk_use_radius_truthy_values` |
| 17 | `c2GJK` | `use_radius != 0` and `!(dist > rA+rB && dist > FLT_EPSILON)` (deeply overlapping or radius-swallowed shapes) | else-branch: `a = b = (a+b)*0.5f`, `dist = 0` | [x] | `err_gjk_radius_else_branch_midpoint` |
| 18 | `c2GJK` | `use_radius != 0`, shrink applied, and the shrunk points collide exactly (`a.x==b.x && a.y==b.y`) | `dist` forced to `0` even though `dist -= rA+rB` produced a non-zero value | [x] | `err_gjk_radius_shrink_exact_equal` |
| 19 | `c2GJK` | `hit` (simplex reached `count == 3`, i.e. origin enclosed) | `a = b`, `dist = 0`, and the `use_radius` block is **not** entered (`else if`) | [x] | `err_gjk_hit_overrides_radius` |
| 20 | `c2GJK` | main-loop guard `iter < 20` reached without converging | loop abandoned, `*iterations == 20`, whatever simplex exists is used | [x] | `err_gjk_iteration_cap` |
| 21 | `c2GJK` | loop `break` on `d1 > d0` (non-monotone progress) | loop abandoned **after** `s.count` was already reduced by `c22`/`c23` | [x] | `err_gjk_break_paths_reachable` |
| 22 | `c2GJK` | loop `break` on `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (degenerate search direction, e.g. `count==1` at the origin, or `count==2` collinear with origin) | loop abandoned; distance comes from the current simplex | [x] | `err_gjk_break_degenerate_direction` |
| 23 | `c2GJK` | loop `break` on duplicate support point (`iA==saveA[i] && iB==saveB[i]`) | loop abandoned **without** `++s.count`, so the freshly written `verts[s.count]` is discarded | [x] | `err_gjk_break_paths_reachable` |
| 24 | `c2GJK` | shapes are two coincident points (circle+circle, r=0, same centre) ⇒ simplex vertex at the origin, `div = 1`, `c2D` returns `(0,0)` | epsilon `break` on iteration 0, `dist = 0`, `*iterations == 0` | [x] | `err_gjk_coincident_points` |
| 25 | `c2GJK` | non-finite shape data (`NaN`, `±inf` coords / radii) | every `<`/`>` test with a NaN operand is false, so control flows through the `else`/fall-through arms; `dist` becomes `NaN`/`inf` | [x] | `err_gjk_nonfinite_shapes` |
| 26 | `c2GJKSimplexMetric` | `s->count` not in {2,3} (`0`, `1`, `4`, `-1`, `INT_MAX`) — `default:` falls through into `case 1:` | returns `0.0f` | [x] | `err_metric_out_of_range_count` |
| 27 | `c2D` | `s->count` not in {1,2} (`case 3:` + `default:`) | returns `c2V(0,0)` | [x] | `err_c2d_out_of_range_count` |
| 28 | `c2D` | `s->count == 2` and `c2Det2(ab, -a) > 0` is **false** (incl. `== 0` and NaN) | takes `c2CCW90(ab)` instead of `c2Skew(ab)` | [x] | `err_c2d_det_not_positive` |
| 29 | `c2L` | `s->count` not in {1,2} (`case 3` hits `default:`) | returns `c2V(0,0)` — note `den` is still computed, so `div == 0` is harmless here | [x] | `err_c2l_out_of_range_count` |
| 30 | `c2L` | `s->div == 0` with `count` 1 or 2 | `den = 1/0 = +inf`; `count==1` ignores it, `count==2` yields `±inf`/`NaN` components | [x] | `err_c2l_zero_div` |
| 31 | `c2Witness` | `s->count` not in {1,2,3} (`0`, `4`, negative) → `default:` | `*a = *b = c2V(0,0)` | [x] | `err_witness_out_of_range_count` |
| 32 | `c2Witness` | `s->div == 0` (or `±0.0`, `NaN`) with `count` 2 or 3 | `den = 1/div` is `±inf`/`NaN`; results propagate `inf`/`NaN` bit patterns | [x] | `err_witness_zero_div` |
| 33 | `c22` | `v <= 0` (origin beyond `a`) | collapse to 1-simplex keeping `a`; `div = 1`, `count = 1` | [x] | `err_c22_all_branches` |
| 34 | `c22` | `u <= 0` (origin beyond `b`) | collapse to 1-simplex, `s->a = s->b` first | [x] | `err_c22_all_branches` |
| 35 | `c22` | degenerate segment `a == b` ⇒ `u == v == 0` ⇒ first test `v <= 0` wins | collapse keeping `a` (not the `u<=0` arm) | [x] | `err_c22_degenerate_equal_points` |
| 36 | `c23` | `vAB <= 0 && uCA <= 0` | collapse to vertex `a` | [x] | `err_c23_all_branches` |
| 37 | `c23` | `uAB <= 0 && vBC <= 0` | collapse to vertex `b` (`s->a = s->b`) | [x] | `err_c23_all_branches` |
| 38 | `c23` | `uBC <= 0 && vCA <= 0` | collapse to vertex `c` (`s->a = s->c`) | [x] | `err_c23_all_branches` |
| 39 | `c23` | `uAB > 0 && vAB > 0 && wABC <= 0` | collapse to edge `ab` | [x] | `err_c23_all_branches` |
| 40 | `c23` | `uBC > 0 && vBC > 0 && uABC <= 0` | collapse to edge `bc` (`a=b; b=c`) | [x] | `err_c23_all_branches` |
| 41 | `c23` | `uCA > 0 && vCA > 0 && vABC <= 0` | collapse to edge `ca` (`b=a; a=c`) | [x] | `err_c23_all_branches` |
| 42 | `c23` | degenerate triangle (`area == 0`: collinear or repeated points) ⇒ `uABC = vABC = wABC = 0`, so the `<= 0` conjuncts are satisfied by whichever earlier arm matches first | first matching arm wins in source order (never the final `else`, unless all three edge tests fail) | [x] | `err_c23_degenerate_triangle` |
| 43 | `c23` | all six edge tests fail (origin strictly inside) ⇒ final `else` | `count = 3`, `div = uABC+vABC+wABC` (which may be `0` for a degenerate triangle) | [x] | `err_c23_all_branches` |
| 44 | `c2Support` | `count <= 0` (`0`, `-1`, `INT_MIN`) — the C code unconditionally reads `verts[0]` before the loop | returns `0`; `verts[0]` **is** dereferenced (caller must supply ≥1 vertex) | [x] | `err_support_nonpositive_count` |
| 45 | `c2Support` | tie: `dot == dmax` (not `>`) | keeps the **earlier** index | [x] | `err_support_ties_keep_first` |
| 46 | `c2Support` | `d == (0,0)` ⇒ all dots are `0` (or `±0`) | returns `0` | [x] | `err_support_zero_direction` |
| 47 | `c2Support` | some vertex/direction component is `NaN` ⇒ `dot > dmax` always false | index is the first non-NaN maximum encountered before any NaN, else `0` | [x] | `err_support_nan` |
| 48 | `c2Norm` / `c2Div` | zero-length vector ⇒ `c2Len == 0` ⇒ `1.0f/0.0f == +inf` | `(0*inf, 0*inf) = (NaN, NaN)`; sign of `NaN` follows IEEE | [x] | `err_norm_zero_vector` |
| 49 | `c2Div` | `b == 0.0f`, `b == -0.0f`, `b == NaN`, `b == ±inf` | `1/b` then component multiply; `±inf`/`NaN`/`±0` propagate | [x] | `err_div_degenerate_denominators` |
| 50 | `c2Norm` | non-finite input (`NaN`, `inf` components) | `c2Len` = `NaN`/`inf`; `1/NaN = NaN`, `1/inf = 0` | [x] | `err_norm_nonfinite` |
| 51 | `c2Len` | negative dot (impossible for `c2Dot(a,a)` unless non-finite) / `NaN` input | `sqrtf(NaN) = NaN`; `sqrtf(-0.0) = -0.0` (sign preserved) | [x] | `err_len_edge_values` |
| 52 | `c2Maxv` / `c2Minv` / `c2Clampv` | `NaN` operand — C uses `?:`, so `a>b`/`a<b` is false and the **`b` operand** is returned | NaN-asymmetric result (differs from `fmaxf`/`fminf` and from Rust's `f32::max`) | [x] | `err_minmax_nan_asymmetry` |
| 53 | `c2Maxv` / `c2Minv` | `+0.0` vs `-0.0` — neither `>` nor `<` holds | returns the **second** operand (`b`), so the sign of zero is `b`'s | [x] | `err_minmax_signed_zero` |
| 54 | `c2Clampv` | inverted box (`lo > hi`) | no rejection — `c2Maxv(lo, c2Minv(a,hi))` silently returns `lo` | [x] | `err_clampv_inverted_box` |
| 55 | `c2CircletoAABB` | inverted / zero-extent AABB (`min > max`), zero-radius circle | no rejection: `d2 < r2` with `r2 = 0` is false ⇒ `0` | [x] | `err_circle_aabb_degenerate` |
| 56 | `c2CircletoCircle` | negative radii (`A.r + B.r < 0`) | `r2 = (A.r+B.r)²` is positive again ⇒ negative radii behave like positive ones | [x] | `err_circle_circle_negative_radii` |
| 57 | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` ⇒ `n = (0,0)`, `da = 0` ⇒ `da < 0` false, `db = 0` ⇒ `db < 0` false ⇒ uses the `bp` branch | distance to `B.b`; the `da / c2Dot(n,n)` division-by-zero branch is **not** taken | [x] | `err_circle_capsule_degenerate` |
| 58 | `c2CircletoCapsule` | `da >= 0 && db < 0` with `c2Dot(n,n) == 0` — unreachable for finite input, reachable with `inf`/`NaN` capsule ends | `da/0` ⇒ `±inf`/`NaN` propagates into `d2` | [x] | `err_circle_capsule_nonfinite` |
| 59 | `c2AABBtoAABB` | inverted boxes (`min > max`) — the four `<` tests | pure comparison result, no rejection; NaN coordinate ⇒ all `d*` are 0 ⇒ returns `1` | [x] | `err_aabb_aabb_degenerate` |
| 60 | `c2AABBtoCapsule` / `c2CapsuletoCapsule` | `c2GJK(...) != 0.0f` — note `NaN != 0` is **true**, so a NaN distance reports "no collision" (`0`) | `return 0` when the float is non-zero (incl. NaN), `1` only for exactly `±0.0f` | [x] | `err_gjk_wrappers_nan_distance` |
| 61 | `c2BBVerts` | `bb->min > bb->max` (inverted) | no validation: writes 4 verts in the fixed order `min, (max.x,min.y), max, (min.x,max.y)` | [x] | `err_bbverts_inverted` |
| 62 | `aabb` | any input, incl. inverted box / NaN / inf | 3-bit bitmask; NaN box ⇒ `c2CircletoAABB` gives 0, `c2AABBtoAABB` gives 1<<1, capsule path via GJK | [x] | `err_aabb_entry_degenerate` |
| 63 | `c2Mulvs` / `c2Add` / `c2Sub` / `c2Dot` / `c2Det2` / `c2Mulrv` / `c2MulrvT` / `c2Mulxv` / `c2Neg` / `c2Skew` / `c2CCW90` / `c2V` | non-finite operands (`NaN` payload/sign, `±inf`, `±0`) | plain IEEE-754 f32 arithmetic in the exact source order; sign of `NaN` and of `0` must match bit-for-bit | [x] | `err_scalar_ops_nonfinite` |
| 63a | every float/vector-returning primitive (`c2Dot`, `c2Det2`, `c2Len`, `c2Div`, `c2Norm`, `c2Add`, `c2Sub`, `c2Mulvs`, `c2Mulrv`, `c2MulrvT`, `c2Mulxv`, `c2Maxv`, `c2Minv`, `c2Clampv`) | **two different NaN payloads meeting in one SSE instruction** — exhaustive 12³ sweep over {+/-qNaN(0), 0x7fc01234, 0xffc04321, 0x7fd00001, 0xffdbeef0, +/-sNaN, +/-inf, +0, 1.0}, both argument orders | x86 resolves the NaN destination-operand-first and quietens it; the destination is whichever register gcc `-O0` happened to pick, so the result payload/sign must match bit-for-bit | [x] | `err_nan_payload_scalar_matrix` |
| 63b | `c22`, `c23`, `c2L`, `c2D`, `c2Witness`, `c2GJKSimplexMetric` | every simplex field (`sA`, `sB`, `p`, `u`, `div`) drawn from the same distinct-payload pool, `count` 1..3 | the composed `c2Dot`/`c2Det2`/`addss`/`mulss`/`divss` chains must produce the same NaN payload in `div`, `u` and every output vector | [x] | `err_nan_payload_simplex_matrix` |
| 63c | `c2GJK` (whole pipeline) | shapes, transforms and cache all built from the distinct-payload pool, `use_radius` 0 and 1, all 9 type pairs | `dist`, `outA`, `outB`, `*iterations` and the written-back cache must match bit-for-bit | [x] | `err_nan_payload_gjk_matrix` |
| 64 | `c2GJK` | `cache->count < 0` (`-1`, `INT_MIN`, …). `!!count` is **true**, but both `for (i = 0; i < cache->count; ++i)` loops run zero times | simplex keeps `count < 0` ⇒ `c2L`/`c2D`/`c2Witness`/`c2GJKSimplexMetric` all take `default:`; epsilon `break` on iteration 0; `dist = 0`, `outA = outB = (0,0)`, `*iterations = 0`, `cache->count`/`div` copied through verbatim, **no** index written back, `cache->metric = 0` | [x] | `err_gjk_cache_negative_count` |
| 65 | `c2GJK` | `cache->count == 4` with every aliased index kept in `[0, 8)` (see U3) | the 4-vertex simplex takes every `default:` arm ⇒ `dist = 0`, `outA = outB = (0,0)`, `cache->count` stays `4` | [x] | `err_gjk_cache_count_four` |
| 63d | `c2Len`, `c2Dot`, `c2Det2`, `c2Div`, `c2Norm`, `c2Mulrv`, `c2MulrvT` | 200 000 unconstrained random **32-bit patterns** per argument (every NaN/subnormal/infinity encoding). `c2Len` matters most: the C makes a real PLT call to glibc `sqrtf` while the Rust emits `sqrtss` | identical bit patterns. `c2Dot(a,a)` can only ever be `>= +0.0`, `+inf` or a quiet NaN — never negative — so glibc's `errno`-setting negative-argument path in `sqrtf` is unreachable | [x] | `err_bitpattern_fuzz` |
| 66 | `c2BBVerts` | `out` buffer **overlaps** `*bb` (legal C, and reachable because `c2MakeProxy` passes `p->verts` as `out`). Each `bb->` load happens *after* the previous `out[...]` store, so `out[3]`'s `bb->min.x` sees the value `out[1]` just wrote | cascading, partially-updated result — **not** the result of a copy-then-write implementation. Every `c2v`-slot offset 0..3 of a shared buffer is swept | [x] | `err_bbverts_output_aliases_input` |
| 67 | `c2Witness` | `a` and/or `b` point **into** `*s` (legal C, e.g. `&s->a.sB`). `*a` is stored *before* the `*b` expression is evaluated | the `*b` computation observes the already-stored `*a` | [x] | `err_witness_output_aliases_simplex` |

## Documented-UB rows (deliberately NOT asserted equal)

These are inputs for which the C code performs an out-of-bounds access or reads
an uninitialised object. The C result is not a value the C source defines — it is
whatever happens to be on the C stack — so a byte-identical Rust result is not
achievable *or* meaningful. They are listed for completeness and are exercised
only for "does not crash" where safe.

| # | function | trigger | why untestable |
|---|----------|---------|----------------|
| U1 | `c2GJK` | `typeA`/`typeB` not in {0,1,2} | `c2MakeProxy` writes nothing (row 5), so `c2Proxy pA;` stays **uninitialised**; `pA.count` is stack garbage which then drives `c2Support`'s loop. Rust zero-initialises instead. Recorded as the `#[ignore]`d test `err_gjk_bad_type_is_ub`. |
| U2 | `c2GJK` | `cache->count > 4` | `for (i = 0; i < cache->count; ++i)` writes `verts[i]` past the end of `c2Simplex s` (a genuine stack smash) and reads `cache->iA[i]` past `iA[3]`. The Rust clamps the loop to the 3 slots the C struct actually has and therefore stays memory-safe. |
| U3 | `c2GJK` | `cache->count == 4` with an unconstrained `div` | the C reads `cache->iA[3]` (which aliases `iB[0]`) and `cache->iB[3]` (which aliases the **float** `div` reinterpreted as an `int`) and uses both as proxy-vertex indices. An ordinary `div` such as `1.0f` becomes the index `1065353216` ⇒ **SIGSEGV**. Row 65 asserts the sub-case where every aliased index lands in `[0, 8)` (`div` bit pattern `0..8`); anything else is unmatchable. It also writes `saveA[3]`/`saveB[3]` one past the end of `int saveA[3], saveB[3]`. |
| U4 | `c2GJK` | `cache->iA[i]` / `cache->iB[i]` outside `[0, proxy.count)` | `pA.verts[iA]` reads an uninitialised (`proxy.count <= iA < 8`) or out-of-bounds (`iA >= 8`, `iA < 0`) `c2v` from the C stack. Confirmed experimentally: a circle proxy (`count == 1`) replayed with `iA[1] == 1` makes the C return stack garbage such as `(-4.68e-17, 1.53e-41)`. Not reproducible ⇒ not asserted. The Rust reads its zero-initialised slot for `0 <= i < 8` and returns `(0,0)` beyond that (`proxy_vert`), so it never faults. Every index the library itself stores into a cache is `< proxy.count`, so no cache produced by `c2GJK` can reach this. |
| U5 | `c2Support` | `verts == NULL`, or `count` larger than the caller's array | unconditional `verts[0]` load / OOB loop. The Rust mirrors the C pointer arithmetic exactly, so the caller's contract is identical. |
| U6 | any pointer-taking function (`c22`, `c23`, `c2D`, `c2L`, `c2Witness`, `c2GJKSimplexMetric`, `c2BBVerts`, `c2MakeProxy`) | `NULL` pointer argument | the C code dereferences without a guard → `SIGSEGV`. Both builds fault; nothing to compare. |

## Coverage summary

`tests/phase_c_errors.rs`: **61 tests, 60 executed + 1 `#[ignore]`d UB record, all
passing** against both `.so`s, in `dev` and `release`, under every feature
combination, and re-run with 3 extra random seeds per combination
(`C2_DIFF_SEED`). Rows 1–67 (incl. 63a/63b/63c/63d): **71/71 checked**.
