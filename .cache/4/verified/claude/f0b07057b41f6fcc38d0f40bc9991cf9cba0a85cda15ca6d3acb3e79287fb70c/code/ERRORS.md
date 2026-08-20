# ERRORS.md — Error / rejection surface table (Phase C)

## Mechanical derivation

```sh
grep -n 'assert\|return -1\|return NULL\|RETURN_ERROR\|errno\|exit(\|abort(' c_src/src/lib.c
#  -> NO MATCHES
```

**This library has no error codes, no sentinels, no asserts and no `errno` use.**
Every function returns either `void`, a `float`, an `int` index, or a `c2v`.
`c2GJK` returns a distance (`float`) and can never signal failure.

The rejection surface therefore consists of the *defensive / degenerate-input
branches* the C actually takes. Rows below were derived by reading EVERY `if`
and `switch` in `c_src/src/lib.c` and keeping each one whose condition is an
invalid, missing, out-of-range or degenerate input (as opposed to ordinary
Voronoi-region case selection, which lives in `CONFIGS.md`). Line numbers refer
to `c_src/src/lib.c`.

`[x]` = differential test written AND passing against both `.so`s.

| # | function | trigger (exact invalid input / condition) | expected C result | test | [x] |
|---|----------|-------------------------------------------|-------------------|------|-----|
| 1 | `c2GJK` | `ax_ptr == NULL` (L363) | substitutes `c2xIdentity()`; no crash, finite dist | `err_gjk_null_ax` | [x] |
| 2 | `c2GJK` | `bx_ptr == NULL` (L367) | substitutes `c2xIdentity()` | `err_gjk_null_bx` | [x] |
| 3 | `c2GJK` | both transforms NULL | both identity | `err_gjk_null_both_x` | [x] |
| 4 | `c2GJK` | `outA == NULL` (L505) | `*outA` not written, other outputs still valid | `err_gjk_null_outA` | [x] |
| 5 | `c2GJK` | `outB == NULL` (L507) | `*outB` not written | `err_gjk_null_outB` | [x] |
| 6 | `c2GJK` | `iterations == NULL` (L509) | `*iterations` not written | `err_gjk_null_iterations` | [x] |
| 7 | `c2GJK` | `outA`,`outB`,`iterations`,`cache` all NULL | only the `float` return is produced | `err_gjk_all_null_outputs` | [x] |
| 8 | `c2GJK` | `cache == NULL` (L378, L495) | cache read AND write skipped; cold start | `err_gjk_null_cache` | [x] |
| 9 | `c2GJK` | `cache != NULL` but `cache->count == 0` (L379) | `cache_was_good == 0` -> cold start, then cache written | `err_gjk_cache_count_zero` | [x] |
| 10 | `c2GJK` | cache warm and `!(min_metric < max_metric*2 && metric < -1e8f)` (L400) | `cache_was_read = 1`: simplex reused, `s.a` NOT reset | `err_gjk_cache_metric_guard_reuse` | [x] |
| 11 | `c2GJK` | cache warm and `min_metric < max_metric*2 && metric < -1e8f` (L400 false side) | `cache_was_read` stays 0 -> simplex overwritten with cold `s.a` | `err_gjk_cache_metric_guard_reject` | [x] |
| 12 | `c2GJK` | `typeA` out of enum range (`3`, `-1`, `99`, `INT_MAX`, `INT_MIN`) | `c2MakeProxy` `switch` has NO `default` -> `pA` left as-is (uninitialised in C) | `err_gjk_invalid_typeA` | [x] |
| 13 | `c2GJK` | `typeB` out of enum range | same for `pB` | `err_gjk_invalid_typeB` | [x] |
| 14 | `c2MakeProxy` | `type` out of enum range, caller-owned `c2Proxy` buffer | proxy buffer left **completely untouched** (no `default:` arm) | `err_makeproxy_invalid_type` | [x] |
| 15 | `c2MakeProxy` | `type` = each valid value, buffer pre-poisoned | only the fields that arm assigns are written; `verts[count..8]` keep poison | `err_makeproxy_partial_write` | [x] |
| 16 | `c2GJK` | `use_radius == 0` (L477) | radius correction skipped entirely; raw simplex distance returned | `err_gjk_use_radius_zero` | [x] |
| 17 | `c2GJK` | `use_radius` nonzero-but-not-1 (`2`, `-1`, `0x100`) | C tests truthiness -> same as `1` | `err_gjk_use_radius_truthy` | [x] |
| 18 | `c2GJK` | shapes overlap -> `s.count == 3` (L436) | `hit = 1`, `a = b`, returns exactly `0.0f` | `err_gjk_hit_zero_dist` | [x] |
| 19 | `c2GJK` | `dist <= rA + rB` with `use_radius` (L480) | midpoint branch: `a = b = (a+b)*0.5f`, `dist = 0` | `err_gjk_radius_overlap_midpoint` | [x] |
| 20 | `c2GJK` | `dist <= FLT_EPSILON` (L481) | midpoint branch even when `rA+rB == 0` | `err_gjk_dist_below_epsilon` | [x] |
| 21 | `c2GJK` | after radius shrink `a.x==b.x && a.y==b.y` (L486) | `dist` forced to `0` | `err_gjk_shrink_collapse` | [x] |
| 22 | `c2GJK` | `d1 > d0` — no progress (L442) | `break` out of the iteration loop early | `err_gjk_no_progress_break` | [x] |
| 23 | `c2GJK` | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (L446) | `break`: search direction degenerate | `err_gjk_degenerate_direction` | [x] |
| 24 | `c2GJK` | duplicate support point `iA==saveA[i] && iB==saveB[i]` (L461) | `dup=1` -> `break` before `++s.count` | `err_gjk_duplicate_support` | [x] |
| 25 | `c2GJK` | iteration cap `iter < 20` (L420) | loop exits with `iter == 20` at most; `*iterations <= 20` | `err_gjk_iteration_cap` | [x] |
| 26 | `c2GJK` | NaN in shape data | comparisons all false; NaN propagates to `dist`/`outA`/`outB` bit-exactly | `err_gjk_nan_inputs` | [x] |
| 27 | `c2GJK` | +/-Inf in shape data | Inf/NaN propagation, no crash | `err_gjk_inf_inputs` | [x] |
| 28 | `c2GJK` | negative shape radius | no validation: `rA+rB < 0`, `dist -= negative` grows | `err_gjk_negative_radius` | [x] |
| 29 | `c2GJK` | AABB with `min > max` (inverted) | no validation; verts wind backwards | `err_gjk_inverted_aabb` | [x] |
| 30 | `c2GJK` | zero-extent AABB (`min == max`) | 4 identical verts; support always index 0 | `err_gjk_degenerate_aabb` | [x] |
| 31 | `c2GJK` | capsule with `a == b` (zero length) | 2 identical verts | `err_gjk_degenerate_capsule` | [x] |
| 32 | `c2Support` | `count == 0` | reads `verts[0]` **anyway** (L294), loop body never runs, returns `0` | `err_support_count_zero` | [x] |
| 33 | `c2Support` | `count < 0` (`-1`, `INT_MIN`) | same: returns `0`, no read past `verts[0]` | `err_support_count_negative` | [x] |
| 34 | `c2Support` | tie `dot == dmax` | strict `>` (L298) keeps the **lowest** index | `err_support_ties` | [x] |
| 35 | `c2Support` | direction `d = (0,0)` | all dots `0`, no strict improvement -> returns `0` | `err_support_zero_dir` | [x] |
| 36 | `c2Support` | NaN in `d` or in `verts` | `dot > dmax` false for NaN -> returns `0` / first non-NaN winner | `err_support_nan` | [x] |
| 37 | `c2GJKSimplexMetric` | `count` not in `{2,3}` (`0`,`1`,`4`,`-1`,`INT_MAX`) | `default:` falls into `case 1:` -> returns `0.0f` | `err_metric_bad_count` | [x] |
| 38 | `c2Witness` | `count` not in `{1,2,3}` (`0`,`4`,`-1`) | `default:` -> `*a = *b = (0,0)` | `err_witness_bad_count` | [x] |
| 39 | `c2Witness` | `s->div == 0` | `den = 1/0 = +Inf` -> Inf/NaN outputs, no crash | `err_witness_zero_div` | [x] |
| 40 | `c2Witness` | `s->div` NaN / Inf | `den` NaN / `0` -> propagates | `err_witness_nan_div` | [x] |
| 41 | `c2L` | `count == 3` or other (`0`,`4`,`-1`) | `default:` -> `(0,0)` | `err_l_bad_count` | [x] |
| 42 | `c2L` | `s->div == 0` | `den = +Inf`, `count==2` -> Inf/NaN components | `err_l_zero_div` | [x] |
| 43 | `c2D` | `count == 3` or other (`0`,`4`,`-1`) | `default:` -> `(0,0)` | `err_d_bad_count` | [x] |
| 44 | `c2D` | `count == 2`, `c2Det2(ab, -a.p) == 0` | not `> 0` -> takes the `c2CCW90` branch | `err_d_det_zero` | [x] |
| 45 | `c2Norm` | `a == (0,0)` | `c2Len = 0`, `1/0 = Inf`, `Inf*0 = NaN` -> `(NaN, NaN)` | `err_norm_zero` | [x] |
| 46 | `c2Norm` | `a` contains NaN/Inf | `sqrtf(NaN)=NaN` -> NaN out, exact bit pattern | `err_norm_nan_inf` | [x] |
| 47 | `c2Div` | `b == 0` | `1/0 = Inf`; `0*Inf = NaN`, `x*Inf = +/-Inf` | `err_div_zero` | [x] |
| 48 | `c2Div` | `b` NaN, or `b == 0` with `a == (0,0)` | NaN out | `err_div_nan` | [x] |
| 49 | `c2Len` | negative-zero / NaN / Inf components | `sqrtf` of negative-zero dot is `-0.0`; `sqrtf(NaN)` -> NaN | `err_len_edge` | [x] |
| 50 | `c22` | `v <= 0` (L186) | keeps `a`, `count=1`, `div=1` | `err_c22_v_le_zero` | [x] |
| 51 | `c22` | `u <= 0` (L190) | `s->a = s->b`, `count=1`, `div=1` | `err_c22_u_le_zero` | [x] |
| 52 | `c22` | both `u` and `v` NaN | all `<=` false -> `else` arm, `div = NaN` | `err_c22_nan` | [x] |
| 53 | `c22` | `s->a.p == s->b.p` (duplicate vertex) | `u = v = 0` -> `v <= 0` first arm wins | `err_c22_duplicate` | [x] |
| 54 | `c23` | `vAB <= 0 && uCA <= 0` (L217) | vertex-A region: `count=1` | `err_c23_region_a` | [x] |
| 55 | `c23` | `uAB <= 0 && vBC <= 0` (L221) | vertex-B region: `s->a = s->b`, `count=1` | `err_c23_region_b` | [x] |
| 56 | `c23` | `uBC <= 0 && vCA <= 0` (L226) | vertex-C region: `s->a = s->c`, `count=1` | `err_c23_region_c` | [x] |
| 57 | `c23` | all three vertices identical | `area = 0` -> `uABC=vABC=wABC=0`; first arm wins | `err_c23_all_same` | [x] |
| 58 | `c23` | collinear vertices (`area == 0`) | `wABC <= 0` etc. all `0` -> falls to an edge or the final `else` | `err_c23_collinear` | [x] |
| 59 | `c23` | NaN vertices | every `<=`/`>` false -> final `else`, `div = NaN`, `count=3` | `err_c23_nan` | [x] |
| 60 | `c2BBVerts` | inverted / NaN AABB | no validation; writes exactly 4 verts, 5th slot untouched | `err_bbverts_edge` | [x] |
| 61 | `gjk` | `reverse == 0` | AABB is shape A, capsule is shape B | `err_gjk_wrapper_forward` | [x] |
| 62 | `gjk` | `reverse != 0` (`1`, `2`, `-1`, `0x7f`) | capsule is shape A, AABB is shape B (truthiness test) | `err_gjk_wrapper_reverse_truthy` | [x] |
| 63 | `gjk` | `reverse` = a value whose low byte is `0` (e.g. `0x100` truncated to `char`) | truncation to `char` makes it falsy | `err_gjk_wrapper_char_truncation` | [x] |
| 64 | `gjk` | `a == NULL` and/or `b == NULL` | forwarded as `outA`/`outB` NULL -> not written, no crash | `err_gjk_wrapper_null_out` | [x] |
| 65 | `gjk` | NaN / Inf / negative-radius capsule, inverted AABB | no validation anywhere; must match bit-for-bit | `err_gjk_wrapper_degenerate` | [x] |
| 66 | `c2Maxv`/`c2Minv`/`c2Clampv` | NaN operand | ternary `a>b?a:b` returns `b` when either is NaN — asymmetric | `err_minmax_nan` | [x] |
| 67 | `c2Clampv` | `lo > hi` (inverted range) | no validation: `c2Maxv(lo, c2Minv(a,hi))` -> returns `lo` | `err_clampv_inverted` | [x] |
| 68 | `c2GJK` | `cache->count` = 1,2,3 with in-range `iA`/`iB` indices | warm-start path fully exercised | `err_gjk_cache_warm_indices` | [x] |
| 69 | `c2GJK` | `cache->count` negative (`-1`) | `!!count` is true -> `for(i=0;i<count;...)` never runs; `s.count = -1`, `s.div = cache->div`; metric `default`->0 | `err_gjk_cache_negative_count` | [x] |
| 70 | `c2GJK` | `cache->div == 0` on warm start | `c2Witness`/`c2L` divide by zero -> Inf/NaN | `err_gjk_cache_zero_div` | [x] |

## Rows deliberately NOT asserted bit-for-bit (undefined behaviour in C)

Two triggers above reach *C undefined behaviour*, where the C result depends on
whatever stack garbage the C compiler happens to leave behind. They are still
tested — for "does not crash, and the defined outputs agree" — but the
uninitialised bytes themselves cannot be, and must not be, matched:

* **Rows 12/13** (`typeA`/`typeB` out of range): C's `c2Proxy pA;` (L371) is an
  uninitialised local and `c2MakeProxy`'s `switch` has no `default:`, so
  `pA.count`, `pA.radius` and `pA.verts` are indeterminate. Rust zero-initialises
  them. The *directly callable* `c2MakeProxy` (row 14) uses a caller-owned buffer
  and IS asserted byte-for-byte, which is the observable, defined part of the
  same behaviour.
* **`cache->iA[i] >= proxy count`**: C reads `pA.verts[iA]` for `iA` up to `7`
  from a proxy that only initialised `count` verts, i.e. reads uninitialised
  stack. Row 68 therefore keeps cache indices `< count`; indices in `count..8`
  are exercised only for "no crash".
