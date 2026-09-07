# ERRORS.md — error / rejection surface table

The C library (`c_src/src/lib.c`) has **no** error enum, no `assert`, no
`RETURN_ERROR` macro and no `return NULL`. Every "rejection" is one of:

* a `switch` arm that falls to `default:` and returns a **sentinel** (`0`,
  `c2V(0,0)`, `0.0f`),
* a **null-pointer guard** (`if (!ptr)` → substitute identity / skip the store),
* a **loop/degeneracy guard** inside `c2GJK` that terminates iteration,
* an **unguarded** divide / `sqrtf` that yields `inf`/`nan` instead of erroring.

Rows below are grepped mechanically from the source (line numbers are
`c_src/src/lib.c`). Each row has a differential test in
`tests/error_paths.rs`.

| # | function | trigger (exact invalid input / condition) | expected C result | test |
|---|----------|-------------------------------------------|-------------------|------|
| 1 | `c2MakeProxy` (L114) | `type` not in {0,1,2} (e.g. 3, 7, 0xFFFFFFFF, `(unsigned)-1`) | `switch` has no `default:` → **`*p` left completely untouched** (caller-visible: proxy keeps its prior contents) | `err_makeproxy_bad_type_leaves_proxy_untouched` |
| 2 | `c2Collided` (L586) | valid `typeA == C2_TYPE_CIRCLE`, `typeB` out of range | `return 0` | `err_collided_bad_typeB` |
| 3 | `c2Collided` (L598) | `typeA == C2_TYPE_AABB`, `typeB` out of range | `return 0` | `err_collided_bad_typeB` |
| 4 | `c2Collided` (L610) | `typeA == C2_TYPE_CAPSULE`, `typeB` out of range | `return 0` | `err_collided_bad_typeB` |
| 5 | `c2Collided` (L614) | `typeA` out of range (any `typeB`, incl. out of range) | `return 0` (outer `default:`) | `err_collided_bad_typeA` |
| 6 | `c2GJK` (L368) | `ax_ptr == NULL` | no crash: `ax = c2xIdentity()` | `err_gjk_null_transforms` |
| 7 | `c2GJK` (L373) | `bx_ptr == NULL` | no crash: `bx = c2xIdentity()` | `err_gjk_null_transforms` |
| 8 | `c2GJK` (L510) | `outA == NULL` | store skipped, distance still returned | `err_gjk_null_outputs` |
| 9 | `c2GJK` (L512) | `outB == NULL` | store skipped | `err_gjk_null_outputs` |
| 10 | `c2GJK` (L514) | `iterations == NULL` | store skipped | `err_gjk_null_outputs` |
| 11 | `c2GJK` (L383) | `cache == NULL` | whole cache read **and** write-back skipped | `err_gjk_null_outputs` |
| 12 | `c2GJK` (L384) | `cache != NULL` but `cache->count == 0` | `cache_was_good == 0` → cache not read, simplex reset to count 1 | `err_gjk_cache_count_zero` |
| 13 | `c2GJK` (L386) | `cache->count < 0` | read loop body never runs, but `s.count = cache->count` (negative) is still installed → `c22/c23` switch takes no arm, `c2L`/`c2D`/`c2Witness` hit their `default:` sentinels | `err_gjk_cache_count_negative` |
| 14 | `c2GJK` (L405) | cached metric such that `min_metric < max_metric*2 && metric < -1e8f` | `cache_was_read` stays 0 → cache **discarded**, simplex re-seeded | `err_gjk_cache_rejected_by_metric` |
| 15 | `c2GJK` (L425) | shapes/config that never converge | `while (iter < 20)` caps iterations at 20 | `err_gjk_iteration_cap` |
| 16 | `c2GJK` (L447) | `d1 > d0` (distance stopped decreasing) | `break` out of the loop | `err_gjk_termination_guards` |
| 17 | `c2GJK` (L451) | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (degenerate search direction, e.g. identical shapes) | `break` | `err_gjk_termination_guards` |
| 18 | `c2GJK` (L466) | new support point duplicates a saved index pair | `dup = 1` → `break` | `err_gjk_termination_guards` |
| 19 | `c2GJK` (L479) | `hit != 0` (simplex reached count 3, i.e. origin enclosed) | `a = b; dist = 0` regardless of `use_radius` | `err_gjk_hit_zero_distance` |
| 20 | `c2GJK` (L485) | `use_radius != 0` and `dist <= rA+rB` **or** `dist <= FLT_EPSILON` | midpoint collapse: `a = b = (a+b)*0.5f`, `dist = 0` | `err_gjk_radius_collapse` |
| 21 | `c2GJK` (L491) | after radius shrink `a == b` exactly | `dist = 0` | `err_gjk_radius_collapse` |
| 22 | `c2GJK` (L482) | `use_radius == 0` with positive-radius shapes | radius **not** subtracted (raw core distance returned) | `err_gjk_use_radius_zero` |
| 23 | `c2GJKSimplexMetric` (L162) | `s->count` not 2 and not 3 (0, 1, 4, negative, huge) | `default:` falls into `case 1:` → `return 0` | `err_simplexmetric_bad_count` |
| 24 | `c2D` (L292) | `s->count == 3` or any other value ≠ 1,2 | `return c2V(0,0)` | `err_c2d_bad_count` |
| 25 | `c2L` (L354) | `s->count` not 1 and not 2 (0, 3, 4, negative) | `default:` → `return c2V(0,0)` | `err_c2l_bad_count` |
| 26 | `c2Witness` (L332) | `s->count` not 1,2,3 (0, 4, negative) | `default:` → `*a = *b = c2V(0,0)` | `err_witness_bad_count` |
| 27 | `c2Witness` (L312) | `s->div == 0` | `den = 1/0 = +inf` → outputs `inf`/`nan` (no guard) | `err_witness_zero_div` |
| 28 | `c2L` (L347) | `s->div == 0` | `den = inf` → `inf`/`nan` components for count 2 | `err_c2l_zero_div` |
| 29 | `c2Div` (L338) | `b == 0` | `c2Mulvs(a, inf)` → `±inf` / `nan` (0*inf) | `err_div_by_zero` |
| 30 | `c2Norm` (L342) | `a == (0,0)` → `c2Len(a) == 0` | `c2Div(a, 0)` → `(nan, nan)` | `err_norm_zero_vector` |
| 31 | `c2Len` (L152) | component `nan`/`inf`, or negative-producing `c2Dot` overflow | `sqrtf` of the dot: `nan` propagates, no domain check | `err_len_nonfinite` |
| 32 | `c2Support` (L298) | `count <= 0` | `verts[0]` is **still dereferenced** before the loop; loop skipped → `return 0` | `err_support_count_le_zero` |
| 33 | `c2Support` (L301) | `count == 1` | loop never runs → `return 0` | `err_support_count_le_zero` |
| 34 | `c2Support` (L303) | all dots equal / `nan` dots (`dot > dmax` always false) | keeps `imax = 0` | `err_support_nan_dots` |
| 35 | `c2CircletoCapsule` (L565) | degenerate capsule `B.a == B.b` → `c2Dot(n,n) == 0` | `da == 0` so `da < 0` false; `db == 0` so `db < 0` false → `bp` branch, **no** division by zero | `err_circletocapsule_degenerate` |
| 36 | `c2CircletoCircle` / `c2CircletoAABB` / `c2CircletoCapsule` | negative radius | comparison uses `r*r` (sign lost) → behaves like `|r|`; `d2 < r2` strict, so touching == no hit | `err_negative_radius` |
| 37 | `c2AABBtoAABB` (L519) | inverted AABB (`min > max`) | pure `<` comparisons, returns whatever the four `<` say (no validation) | `err_inverted_aabb` |
| 38 | `c2CircletoAABB` (L548) | inverted AABB → `c2Clampv(a, min, max)` = `max(min, min(a, max))` | clamp yields `min` when inverted; no validation | `err_inverted_aabb` |
| 39 | all float entry points | `nan` / `+inf` / `-inf` / `-0.0` / subnormal inputs | IEEE-754 propagation, no checks anywhere in the file | `err_nonfinite_inputs_propagate` |
| 40 | `reverse_collide` (L619) | any `x`,`y`,`r` incl. `nan`/`inf`/negative `r` | result is the 3-bit OR of three `c2Collided` calls; nan radius ⇒ all comparisons false ⇒ `0` | `err_reverse_collide_nonfinite` |

## Explicitly NOT tested (undefined behaviour in C, not a specifiable contract)

| trigger | why excluded |
|---------|--------------|
| `c2GJK(A=NULL, ...)` with a valid type | `c2MakeProxy` dereferences `shape` unconditionally → segfault in both libs |
| `c2GJK` with out-of-range `typeA`/`typeB` | `c2Proxy pA;` is an **uninitialised stack object**; the untranslatable value of `pA.count`/`pA.verts` then drives the whole algorithm. Not reproducible by construction. |
| `cache->count > 3` | reads past `int iA[3]` / `int iB[3]` — out-of-bounds |
| `cache->iA[i] >= pA.count` (e.g. index 1 with a CIRCLE proxy, whose `count` is 1) | `c2GJK` declares `c2Proxy pA;` **uninitialised on the stack**, and `c2MakeProxy` only writes `verts[0 .. count-1]`. Reading `pA.verts[i]` for `i >= count` therefore reads stack garbage in C and drives the whole GJK loop from it. Confirmed experimentally: this is the one construction that makes C and Rust disagree, and it is unspecifiable rather than a translation bug. All cache-bearing tests keep every cached index below the proxy's initialised vertex count (`proxy_vert_count` in `tests/stress.rs`). |
| `c2Support(verts=NULL, ...)` | unconditional `verts[0]` load |
| `c2BBVerts(out)` with fewer than 4 slots | unconditional 4 stores |
