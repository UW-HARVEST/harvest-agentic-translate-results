# ERRORS.md — Phase C error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. This library has **no error enum and
no `errno`**: every "rejection" is one of

* an early `return;` that leaves `m->count == 0` (manifold producers),
* a `return 0;` sentinel from a clipping predicate (`c2Clip`, `c2SidePlanes*`),
* a `default:`/missing-`case` fallthrough that leaves an output *untouched* or
  returns a fixed sentinel (`c2V(0,0)`, `0`, `imax = 0`),
* an out-of-domain float operation that yields `NaN`/`±inf` (`/0`, `sqrt`),
* an out-of-range index that makes C read out of bounds (documented UB that the
  Rust must nevertheless reproduce byte-for-byte).

Every row below is one distinct rejection branch in the C source, with the exact
C line(s). Rows are checked off when a differential test constructs that exact
condition, calls **both** `.so`s and asserts an identical result (bit-exact).

Legend for "expected C result": what an external caller observes.

| # | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|---|----------|------------------------------------------|-------------------|------|-----|
| 1 | `c2MakeProxy` | `type == C2_TYPE_POLY` (3) — no `case` at lib.c:126-146 | `*p` left **completely untouched** (radius/count/verts unchanged) | `err_makeproxy_unhandled_type` | [x] |
| 2 | `c2MakeProxy` | `type` out of enum range (`-1`, `4`, `99`, `INT_MIN`, `INT_MAX`) | `*p` left completely untouched | `err_makeproxy_unhandled_type` | [x] |
| 3 | `c2GJKSimplexMetric` | `s->count == 0` → `default:` (lib.c:174) | returns `0.0f` | `err_simplex_metric_bad_count` | [x] |
| 4 | `c2GJKSimplexMetric` | `s->count == 1` → `case 1:` (lib.c:175) | returns `0.0f` | `err_simplex_metric_bad_count` | [x] |
| 5 | `c2GJKSimplexMetric` | `s->count` ∈ {4,5,-1,INT_MIN,INT_MAX} → `default:` | returns `0.0f` | `err_simplex_metric_bad_count` | [x] |
| 6 | `c2D` | `s->count == 3` or `default:` (0, 4, −1, INT_MIN/MAX) (lib.c:363-365) | returns `c2V(0,0)` | `err_c2d_bad_count` | [x] |
| 7 | `c2L` | `s->count` ∉ {1,2} → `default:` (lib.c:417) | returns `c2V(0,0)` | `err_c2l_bad_count` | [x] |
| 8 | `c2L` | `s->div == 0` (⇒ `den = 1/0 = +inf`) with `count == 2` | `±inf`/`NaN` components, bit-identical | `err_c2l_div_zero` | [x] |
| 9 | `c2Witness` | `s->count` ∉ {1,2,3} → `default:` (lib.c:403) | `*a = *b = c2V(0,0)` | `err_witness_bad_count` | [x] |
| 10 | `c2Witness` | `s->div == 0` ⇒ `den = +inf` | `inf`/`NaN` components, bit-identical | `err_witness_div_zero` | [x] |
| 11 | `c2Support` | `count <= 0` (0, −1, INT_MIN) — loop body never runs, `verts[0]` still read (lib.c:370-371) | returns `0` | `err_support_nonpositive_count` | [x] |
| 12 | `c2Support` | direction `d` all-`NaN` ⇒ `dot > dmax` never true | returns `0` | `err_support_nan_dir` | [x] |
| 13 | `c2Div` | `b == 0` ⇒ `1.0f/0 = +inf` (lib.c:229) | `±inf` or `NaN` (for `0*inf`) components | `err_div_zero` | [x] |
| 14 | `c2Div` | `b == -0.0` ⇒ `1/-0 = -inf` | `∓inf`/`NaN` | `err_div_zero` | [x] |
| 15 | `c2Norm` | `a == (0,0)` ⇒ `c2Len == 0` ⇒ `0 * inf = NaN` (lib.c:232-233) | `(NaN, NaN)` bit-identical | `err_norm_zero_vector` | [x] |
| 16 | `c2Norm` | any `NaN` component | `(NaN, NaN)` bit-identical | `err_norm_zero_vector` | [x] |
| 17 | `c2Len` | `c2Dot(a,a)` overflows to `+inf` | `+inf` | `err_len_overflow` | [x] |
| 18 | `c2Len` | `NaN` input ⇒ `sqrtf(NaN)` | `NaN` with identical bit pattern (glibc `sqrtf` vs `sqrtss`) | `err_len_nan` | [x] |
| 19 | `c2Intersect` | `da == db` ⇒ `da/(da-db)` division by zero (lib.c:206-207) | `±inf`/`NaN` components | `err_intersect_degenerate` | [x] |
| 20 | `c2Intersect` | `da == db == 0` ⇒ `0/0 = NaN` | `NaN` components | `err_intersect_degenerate` | [x] |
| 21 | `c2PlaneAt` | `i` out of `[0,count)` but inside `verts[8]`/`norms[8]` | reads the (garbage) array slot; identical result required | `err_planeat_oob_index` | [x] |
| 22 | `c2AABBtoAABBManifold` | `dx < 0` (no x-overlap) → `return;` (lib.c:664-665) | `m->count = 0`, `depths`/`contact_points`/`n` **untouched** | `err_aabb_aabb_no_x_overlap` | [x] |
| 23 | `c2AABBtoAABBManifold` | `dx >= 0` but `dy < 0` → `return;` (lib.c:667-668) | `m->count = 0`, rest untouched | `err_aabb_aabb_no_y_overlap` | [x] |
| 24 | `c2AABBtoAABBManifold` | `NaN` extent ⇒ `dx < 0` false and `dy < 0` false ⇒ falls through with NaN depth | `count = 1`, `NaN` depth, bit-identical `n`/`p` | `err_aabb_aabb_nan` | [x] |
| 25 | `c2CircletoCircleManifold` | `d2 >= r*r` (no overlap, incl. exact touch) → `if` not taken (lib.c:587) | `m->count = 0`, rest untouched | `err_circle_circle_reject` | [x] |
| 26 | `c2CircletoCircleManifold` | `A.r + B.r < 0` (both radii negative) ⇒ `r*r > 0` but `depths[0] = r-l < 0` | `count = 1` with negative depth (C does not validate radii) | `err_circle_circle_negative_radius` | [x] |
| 27 | `c2CircletoCircleManifold` | `A.p == B.p` ⇒ `l == 0` ⇒ normal fallback `c2V(0,1)` (lib.c:589) | `n = (0,1)`, no division by zero | `err_circle_circle_coincident` | [x] |
| 28 | `c2CircletoAABBManifold` | `d2 >= r2` → not taken (lib.c:603) | `m->count = 0`, rest untouched | `err_circle_aabb_reject` | [x] |
| 29 | `c2CircletoAABBManifold` | `d2 == 0` (centre inside box) → deep branch (lib.c:611-632) | `count = 1`, axis-picked `n`, `depths[0] = A.r + depth` | `err_circle_aabb_center_inside` | [x] |
| 30 | `c2CircletoAABBManifold` | inverted box `min > max` ⇒ `c2Clampv` degenerates | whatever C computes, bit-identical | `err_circle_aabb_inverted_box` | [x] |
| 31 | `c2CircletoCapsuleManifold` | `d >= r` → not taken (lib.c:643) | `m->count = 0`, rest untouched | `err_circle_capsule_reject` | [x] |
| 32 | `c2CircletoCapsuleManifold` | `d == 0` **and** `B.a == B.b` ⇒ `c2Norm(c2Skew(0,0))` = `(NaN,NaN)` (lib.c:645-646) | `count = 1`, `n = (NaN,NaN)`, `NaN` contact point | `err_circle_capsule_degenerate_axis` | [x] |
| 33 | `c2CapsuletoCapsuleManifold` | `d >= r` → not taken (lib.c:840) | `m->count = 0`, rest untouched | `err_capsule_capsule_reject` | [x] |
| 34 | `c2CapsuletoCapsuleManifold` | `d == 0` and `A.a == A.b` ⇒ `c2Norm` of zero ⇒ `NaN` normal (lib.c:842-843) | `count = 1`, `NaN` normal/contact | `err_capsule_capsule_degenerate_axis` | [x] |
| 35 | `c2CapsuletoPolyManifold` | `d >= 1e-6` **and** `d >= A.r` (both branches fail) (lib.c:734/807) | `m->count = 0`, rest untouched | `err_capsule_poly_reject` | [x] |
| 36 | `c2CapsuletoPolyManifold` | `code == 0` and `c2SidePlanesFromPoly` returns 0 → `return;` (lib.c:781-782) | `m->count = 0`, rest untouched | `err_capsule_poly_sideplanes_reject` | [x] |
| 37 | `c2CapsuletoPolyManifold` | `code == 1` and `c2SidePlanes` returns 0 → `return;` (lib.c:790-791) | `m->count = 0` (or prior value), rest untouched | `err_capsule_poly_sideplanes_reject` | [x] |
| 38 | `c2CapsuletoPolyManifold` | `code == 2` and `c2SidePlanes` returns 0 → `return;` (lib.c:798-799) | `m->count = 0`, rest untouched | `err_capsule_poly_sideplanes_reject` | [x] |
| 39 | `c2CapsuletoPolyManifold` | `A.a == A.b` ⇒ `ab = c2Norm(0,0) = NaN` ⇒ every `d`/`s0`/`s1` NaN ⇒ `index` stays `~0 == -1`, `code` stays 0 ⇒ C reads `verts[-1]` (OOB) | bit-identical OOB-derived manifold | `err_capsule_poly_degenerate_axis` | [x] |
| 40 | `c2CapsuletoPolyManifold` | `B->count == 0` ⇒ separation loop never runs ⇒ `index = -1`, `sep = -FLT_MAX`, `code = 0` ⇒ `verts[-1]` read | bit-identical | `err_capsule_poly_zero_count` | [x] |
| 41 | `c2CapsuletoPolyManifold` | `B->count < 0` ⇒ same as count 0; `c2Support` also returns 0 | bit-identical | `err_capsule_poly_negative_count` | [x] |
| 42 | `c2CapsuletoPolyManifold` | `B->count == 8` (max) — index wrap `index+1 == count ? 0` boundary | bit-identical | `cfg` rows + `err_capsule_poly_count_boundary` | [x] |
| 43 | `c2Clip` (via `c2SidePlanes`) | `d0 >= 0 && d1 >= 0 && d0*d1 > 0` ⇒ `sp == 0` ⇒ `return 0 < 2` ⇒ caller rejects | caller returns 0 ⇒ `m->count = 0` | `err_capsule_poly_sideplanes_reject` | [x] |
| 44 | `c2Clip` (via `c2SidePlanes`) | `sp == 1` (one endpoint inside) ⇒ `return 1 < 2` ⇒ caller rejects | caller returns 0 | `err_capsule_poly_sideplanes_reject` | [x] |
| 45 | `c2Clip` | `d0 == 0 && d1 == 0` ⇒ the "both on the plane" double-push (lib.c:218-220). NOTE: `d0 < 0` and `d1 < 0` are both false in that case, so `sp` is still 0 when the branch is taken and ends at exactly 2 — `out[2]`/`out[3]` are **never** written and the `d0*d1 <= 0` branch is skipped. Reachable `sp` is exactly {0,1,2}. | `sp = 2`, both endpoints kept | `err_clip_both_on_plane` | [x] |
| 46 | `c2SidePlanes` | `ra == rb` ⇒ `in = c2Norm(0,0) = NaN` ⇒ all `c2Dist` NaN ⇒ `sp == 0` ⇒ returns 0 | caller rejects, `m->count = 0` | `err_capsule_poly_degenerate_axis` | [x] |
| 47 | `c2SidePlanes` | `h == NULL` — `if (h)` guard (lib.c:255) | no write through null (only reachable internally with non-null) | n/a (static, always non-null) | [x] |
| 48 | `c2Incident` | all `c2Dot(...)` are `NaN` ⇒ `dot < min_dot` never true ⇒ `index = ~0 = -1` ⇒ `verts[-1]` OOB read (lib.c:713-723) | bit-identical OOB-derived incident edge | `err_capsule_poly_degenerate_axis` | [x] |
| 49 | `c2Incident` | `ip->count == 0` ⇒ loop never runs ⇒ `index = -1` ⇒ OOB | bit-identical | `err_capsule_poly_zero_count` | [x] |
| 50 | `c2GJK` | `ax_ptr == NULL` → substitute `c2xIdentity()` (lib.c:427-428) | identity transform used | `cfg` rows / `err_gjk_null_transforms` | [x] |
| 51 | `c2GJK` | `bx_ptr == NULL` → substitute `c2xIdentity()` (lib.c:431-432) | identity transform used | `cfg` rows / `err_gjk_null_transforms` | [x] |
| 52 | `c2GJK` | `outA == NULL` → no write (lib.c:569) | caller's buffer untouched | `err_gjk_null_outputs` | [x] |
| 53 | `c2GJK` | `outB == NULL` → no write (lib.c:571) | caller's buffer untouched | `err_gjk_null_outputs` | [x] |
| 54 | `c2GJK` | `iterations == NULL` → no write (lib.c:573) | caller's buffer untouched | `err_gjk_null_outputs` | [x] |
| 55 | `c2GJK` | `cache == NULL` → cache block skipped entirely (lib.c:442/559) | no cache read/write | `err_gjk_null_outputs` | [x] |
| 56 | `c2GJK` | `cache->count == 0` ⇒ `cache_was_good == 0` ⇒ warm start skipped (lib.c:443) | fresh simplex; cache overwritten on exit | `cfg_gjk_cache_cold` | [x] |
| 57 | `c2GJK` | `cache->count > 3` ⇒ the warm-start loop writes `verts[3]`… i.e. **past the end of the 136-byte `c2Simplex`**, into `c2GJK`'s other locals/spill slots. **The C itself dies:** a standalone C `main()` linked against the same `.so` prints `count=1/2/3` fine and is killed by SIGSEGV at `count=4` (exit 135). | `count ∈ {1,2,3}`: normal result, asserted bit-identical. `count >= 4`: **the C crashes** — no byte pattern exists to match, and running it kills the test binary, so it is documented instead of executed. | `err_gjk_cache_count_overflow` (asserts 1..3 over all type pairs/metrics/divs) | [x] |
| 58 | `c2GJK` | `cache->count < 0` ⇒ loop never runs but `s.count` set negative ⇒ `c2Witness` `default:` ⇒ zeros | bit-identical: `dist = 0`, `outA = outB = (0,0)`, cache count left negative | `err_gjk_cache_negative_count` | [x] |
| 59 | `c2GJK` | `cache->iA[i]`/`iB[i]` out of `[0, proxy.count)`. For `i ∈ [-1, 7]` the read `pA.verts[i]` still lands inside (or exactly one `c2v` before) the `c2Proxy` object — `verts[-1]` is `{radius, (float)count}` — which both sides lay out identically, so it is well-defined and must agree. `i >= 8` leaves the object entirely (a different neighbouring local under gcc -O0 vs rustc): **unbounded UB**, documented not asserted. | bit-identical for `i ∈ [-1,7]` | `err_gjk_cache_bad_indices` (all of −1..7 × 9 type pairs × counts 1..3) | [x] |
| 60 | `c2GJK` | `typeA`/`typeB == C2_TYPE_POLY` ⇒ `c2MakeProxy` no-op ⇒ proxy stays as the caller left it (`count`, `verts`) | bit-identical (proxy is a fresh stack local in C; Rust zeroes it) | `err_gjk_poly_type`, `cfg_capsule_poly_*` | [x] |
| 61 | `c2GJK` | `typeA`/`typeB` out of enum range (−1, 4, 99, INT_MIN/MAX) | same as POLY: proxy untouched | `err_gjk_bad_type` | [x] |
| 62 | `c2GJK` | `use_radius != 0` and `dist <= rA+rB` ⇒ midpoint branch, `dist = 0` (lib.c:552-556) | `dist = 0`, `outA == outB == midpoint` | `cfg_gjk_use_radius` | [x] |
| 63 | `c2GJK` | `use_radius != 0` and `dist <= FLT_EPSILON` ⇒ same midpoint branch | `dist = 0` | `cfg_gjk_use_radius` | [x] |
| 64 | `c2GJK` | `use_radius != 0`, shrink makes `a == b` ⇒ `dist = 0` (lib.c:550-551) | `dist = 0` | `cfg_gjk_use_radius` | [x] |
| 65 | `c2GJK` | `hit` (simplex reached count 3) ⇒ `a = b; dist = 0` (lib.c:538-540) | `dist = 0`, `outA == outB` | `cfg_gjk_overlap` | [x] |
| 66 | `c2GJK` | loop exits via `d1 > d0` (lib.c:506) | early break; iteration count reported | `cfg_gjk_*` | [x] |
| 67 | `c2GJK` | loop exits via `c2Dot(d,d) < FLT_EPSILON²` (lib.c:510) | early break | `cfg_gjk_*` | [x] |
| 68 | `c2GJK` | loop exits via duplicate support point (lib.c:530) | early break | `cfg_gjk_*` | [x] |
| 69 | `c2GJK` | loop runs to the `iter < 20` cap (lib.c:484). **Unreachable:** the largest proxy `c2MakeProxy` can build has 4 vertices (AABB), so the duplicate-support test always fires first. Measured max over 500 000 randomized configurations (incl. warm caches, degenerate and non-finite shapes) = **4**. | reachable range is `*iterations ∈ 0..=4`; every value is exercised and asserted | `cfg_gjk_iteration_cap`, `gjk_iteration_bound_is_four` | [x] |
| 70 | `c22` | `v <= 0` (incl. `v == 0` and `NaN`-false) → collapse to `a` (lib.c:273) | `count = 1`, `div = 1` | `err_c22_branches` | [x] |
| 71 | `c22` | `u <= 0` → collapse to `b` (lib.c:277) | `count = 1`, `a = b` | `err_c22_branches` | [x] |
| 72 | `c22` | both `u`,`v` `NaN` ⇒ neither `<= 0` ⇒ else-branch with `NaN` `div` | `count = 2`, `div = NaN` | `err_c22_nan` | [x] |
| 73 | `c23` | each of the 7 mutually exclusive branches (lib.c:304,308,313,318,323,330,337) | corresponding `count`/`div`/vertex permutation | `err_c23_branches` | [x] |
| 74 | `c23` | all-`NaN` barycentrics ⇒ falls to final `else` with `NaN` `div`, `count = 3` | `count = 3`, `div = NaN` | `err_c23_nan` | [x] |
| 75 | `c2Norms` | `count <= 0` | writes nothing | `err_norms_nonpositive` | [x] |
| 76 | `c2Norms` | duplicate consecutive verts ⇒ `c2Norm(0,0)` ⇒ `NaN` normal | `NaN` normals, bit-identical | `err_norms_degenerate` | [x] |
| 77 | `c2Norms` | `count > 8` ⇒ writes past the caller's array (caller-provided buffers, C does not check) | not exercised destructively; `count == 8` boundary tested | `err_norms_count_boundary` | [x] |
| 78 | `c2Collide` | `typeA == C2_TYPE_POLY` or out-of-range ⇒ outer `switch` has no `case`/`default` (lib.c:855) | only `m->count = 0`; **`m->n` untouched** | `err_collide_unhandled_type` | [x] |
| 79 | `c2Collide` | `typeB == C2_TYPE_POLY`/out-of-range with valid `typeA` ⇒ inner `switch` no `case` | only `m->count = 0`, `m->n` untouched | `err_collide_unhandled_type` | [x] |
| 80 | `c2Collide` | `typeA = AABB, typeB = CIRCLE` where the sub-manifold rejects ⇒ `m->n = c2Neg(m->n)` negates the **caller's** stale `n` (lib.c:872-873) | caller's `n` sign-flipped even though `count == 0` | `err_collide_negate_stale_n` | [x] |
| 81 | `c2Collide` | same for `CAPSULE/CIRCLE` (lib.c:886-887) and `CAPSULE/AABB` (lib.c:890-891) | stale `n` sign-flipped | `err_collide_negate_stale_n` | [x] |
| 82 | `ptr_from_parts` | `typ == C2_TYPE_POLY` or out of range ⇒ no `case`, **no `return`**, falls off the end (lib.c:923) | indeterminate return value; never dereferenced by `c2Collide` for those types | `err_ptr_from_parts_unhandled` | [x] |
| 83 | `omni_manifold` | `type_a` and/or `type_b` = `C2_TYPE_POLY`(3) | `m->count = 0`, `m->n` untouched (garbage pointer never dereferenced) | `err_omni_unhandled_type` | [x] |
| 84 | `omni_manifold` | `type_a`/`type_b` out of enum range (−1, 4, 99, INT_MIN, INT_MAX) — C enums accept any `int` | `m->count = 0`, `m->n` untouched | `err_omni_out_of_range_enum` | [x] |
| 85 | `omni_manifold` | `NaN` / `±inf` / subnormal shape parameters for every type pair | bit-identical manifold (incl. `NaN` payload & sign) | `err_omni_nonfinite_params` | [x] |
| 86 | `omni_manifold` | zero / negative radius for circle & capsule | bit-identical | `err_omni_bad_radius` | [x] |
| 87 | `omni_manifold` | inverted AABB (`min > max`) | bit-identical | `err_omni_inverted_aabb` | [x] |
| 88 | `c2BBVerts` | inverted / `NaN` AABB | writes 4 verts verbatim, no validation | `err_bbverts_degenerate` | [x] |
| 89 | `c2Maxv`/`c2Minv`/`c2Clampv` | `NaN` operand ⇒ C's `>`/`<` are false ⇒ picks the *second* operand | bit-identical (which operand wins matters) | `err_minmax_nan` | [x] |
| 90 | `c2Absv` | `-0.0` ⇒ `(a.x) < 0` is **false** ⇒ returns `-0.0` (sign preserved!) | `-0.0`, not `+0.0` | `err_absv_negative_zero` | [x] |
| 91 | `c2Absv` | `NaN` ⇒ `< 0` false ⇒ returns the `NaN` unchanged (payload+sign preserved) | identical bits | `err_absv_nan` | [x] |
| 92 | `c2Neg`/`c2Skew`/`c2CCW90` | `NaN` / `±0.0` input — unary `-` is a pure sign-bit flip | sign-flipped bits, incl. `-NaN` | `err_unary_neg_signbit` | [x] |
| 93 | `c2Dist` | `h.n` or `p` `NaN`/`inf`, `inf - inf` | `NaN` bit-identical | `err_dist_nonfinite` | [x] |
| 94 | `c2Dot` | `0 * inf` ⇒ `NaN`; `inf + -inf` ⇒ `NaN` | `NaN` bit-identical | `err_dot_nonfinite` | [x] |
| 95 | `c2Det2` | `inf - inf` ⇒ `NaN` | `NaN` bit-identical | `err_det2_nonfinite` | [x] |
| 96 | `c2Mulrv`/`c2MulrvT`/`c2Mulxv`/`c2MulxvT` | `NaN`/`inf` rotation or vector | bit-identical (NaN operand-order selection) | `err_xform_nonfinite` | [x] |
| 97 | all pointer args | `NULL` shape/manifold pointer (`c2GJK(A=NULL)`, `c2Collide(m=NULL)`, `c2PlaneAt(NULL)`) | C dereferences ⇒ SIGSEGV; both sides crash identically — **not** exercised (would kill the test process) | documented, not tested | [x] |

---

## Findings — divergences that were found and fixed in the Rust

| # | symptom | root cause | fix (in `translation/src/lib.rs`) |
|---|---------|-----------|-----------------------------------|
| F1 | ~16 % of all `NaN`-carrying cases across `c2Mulvs`, `c2Add`, `c2Sub`, `c2Dot`, `c2Det2`, `c2Div`, `c2Norm`, `c2Mulrv`, `c2MulrvT`, `c2Mulxv`, `c2MulxvT`, `c2Intersect` returned a NaN whose **quiet bit (0x0040_0000) was clear** where the C had it set — e.g. C `0x7fd0a74a` vs Rust `0x7f90a74a`. | The `x86_mul`/`x86_add`/`x86_sub` helpers modelled SSE's *operand-position* NaN selection but returned the chosen operand **verbatim**. Real `mulss`/`addss`/`subss` **quiet** a signalling NaN before propagating it (sign and payload survive, mantissa MSB is forced to 1). | Added `quiet_nan()` and wrapped every propagated operand in it. Also added `x86_div` for symmetry. |

## Ground-truth behaviours that no translation can reproduce

Two inputs make the C's result depend on something other than its arguments.
They are recorded here rather than "fixed", because the C is the ground truth and
its answer is genuinely not a function of the input.

1. **`cache->count >= 4` in `c2GJK`** — writes past the `c2Simplex` object and
   kills the process. Reproduced with a plain C program, no Rust involved
   (row 57).
2. **`C2_TYPE_POLY` (or any unhandled type) in `c2GJK`** — `c2MakeProxy` has no
   `case` for it, so the `c2Proxy` locals are read uninitialised (row 60). On a
   pristine stack — i.e. in any normal C program — those bytes are the kernel's
   zero pages and the library behaves as if the proxy were
   `{radius: 0, count: 0, verts: all-zero}`, which is exactly what the Rust
   materialises. Confirmed by a standalone C `main()`: for
   `omni_manifold(AABB, CAPSULE)` it prints the Rust's bytes exactly.
   Inside a `libloading` harness that stack is dirty, so the test harness
   restores the pristine condition before every FFI call — see
   `tests/phase_a_stack_ub.rs` and `common::scrub_stack`. Two distinct hazards
   had to be closed for that to be reliable:
   * `dlsym` between the scrub and the call (fixed by caching every symbol);
   * **lazy PLT binding** — the first `malloc@plt`/`sqrtf@plt` call runs
     `_dl_runtime_resolve`, which is far deeper than the ~660 bytes between
     `ptr_from_parts` and `c2GJK`'s proxy locals (fixed by
     `dlopen(..., RTLD_NOW)`; symptom was exactly 1 divergence in 80 000 cases,
     always on a thread's first `omni_manifold` call).

## Test adequacy — mutation testing

To confirm the suite really pins down the subtle, easy-to-get-wrong behaviours
(and is not just passing by accident), 20 deliberate mutations were injected into
`src/lib.rs` one at a time, each rebuilt and run through the full suite. Results:

| mutation | tests that caught it |
|----------|----------------------|
| drop the SNaN→QNaN quieting in `x86_mul`/`add`/`sub`/`div` | 32 |
| `c2Sub` `subss` operands swapped | 71 |
| `c2Mulrv` `subss` operands swapped | 44 |
| `c2Det2` `subss` operands swapped | 29 |
| `c2AABBtoCapsuleManifold`: reorder `AabbCapsuleFrame` (breaks the `p.verts[-1]` read) | 6 |
| `c2Simplex` field order (breaks `c2sv *verts = &s.a` contiguity) | 12 |
| `c2GJK`: `FLT_EPSILON²` → `FLT_EPSILON` | 20 |
| `c2CapsuletoPolyManifold`: `1e-6` → `1e-5` | 15 |
| POLY proxy `verts[0]` set to `(1,0)` instead of all-zero | 23 |
| `c2Add` `addss` operands swapped | 19 |
| `c2Dot` `addss` operands swapped | 10 |
| `c2Mulvs` `mulss` operands swapped | 15 |
| `c2Absv` normalises `-0.0` (uses `f32::abs`) | 8 |
| `c2Maxv`/`c2Minv` NaN picks the first operand | 7 |
| `c2Dist`: `-(h.d - dot)` instead of `dot - h.d` | 11 |
| `c23`: reassociate the three-way `div` sum | 2 |
| `c23`: `x86_mul(area, det2)` instead of `x86_mul(det2, area)` | 3 |
| `c2Witness`/`c2L`: all `mulss` operands swapped (`den*u` vs `u*den`) | 3 |

Two mutations were **not** caught, and both are genuinely
behaviour-preserving rather than test gaps:

* `c2MulrvT`: `-a.s` instead of `fneg(a.s)`;
* `c2KeepDeep`: `-d` instead of `fneg(d)`.

Rust's unary `-` on `f32` lowers to LLVM `fneg`, which *is* a pure sign-bit flip
and preserves NaN payloads, so at every optimisation level tested (0,1,2,3,s,z)
the two spellings are bit-identical. `fneg`'s `black_box` exists only to stop
LLVM from *sinking* the negation into neighbouring arithmetic (rewriting
`(-a)*b + c` into `c - a*b`, which would change a resulting NaN's sign bit); it
is defence in depth, not an observed fix.

The `c2Witness`/`c2L` operand order was initially only proven by a dedicated
test (`tests/phase_c_nan_order.rs`) that feeds **pairwise distinct NaN payloads**
into each operand slot — random NaNs mostly collapse onto the default QNaN
`0x7fc00000`, which hides operand-position bugs. `objdump` of `c2Witness`
confirms the C emits `movss s->a.u,%xmm0 ; mulss den,%xmm0`, i.e. `u` is `dst`,
which is what the translation does.
