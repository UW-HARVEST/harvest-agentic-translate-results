# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. Grep results:

```sh
grep -n 'assert\|RETURN_ERROR\|return -1\|return NULL\|errno' src/lib.c   # -> 0 hits
grep -n 'if (!\|<= 0\|> 0\|== 0'                              src/lib.c   # -> 14 hits
```

**There is no error enum, no `assert`, and no error return code anywhere in the
library.** Every public function either returns a value unconditionally or
returns `void`. The rejection surface therefore consists of:

* **defensive null checks** that silently substitute a fallback instead of
  failing (`c2GJK`'s six pointer checks),
* **`switch` / `default:` fall-throughs** that swallow out-of-domain `count`
  values and out-of-domain `C2_TYPE` enum values,
* **unguarded divisions and square roots** whose sentinel is `inf` / `NaN`
  rather than an error code,
* **unvalidated indices, counts and radii** taken straight from caller memory.

Every row below is one distinct such branch. "Expected C result" is the
observable behaviour of the C, which the Rust must reproduce bit-for-bit.
The **test** column names the `#[test]` in `tests/phase_c_errors.rs` that
constructs that exact condition, calls BOTH `.so`s and asserts they agree.

Status legend: `[x]` = differential test passes · `[UB]` = undefined behaviour
in the C, see "UB rows" at the bottom for what is asserted instead.

| #  | function | trigger (exact invalid input / condition) | expected C result | test | ok |
|----|----------|--------------------------------------------|-------------------|------|----|
| 1  | `c2GJK` | `ax_ptr == NULL` | no deref; `ax = c2xIdentity()` = `{p:(0,0), r:{c:1,s:0}}`; result identical to passing an explicit identity | `row01_02_null_transform_fallback` | [x] |
| 2  | `c2GJK` | `bx_ptr == NULL` | as row 1 for `bx` | `row01_02_null_transform_fallback` | [x] |
| 3  | `c2GJK` | `outA == NULL` | `*outA` not written (poison survives); return value unchanged | `row03_05_null_out_params_not_written` | [x] |
| 4  | `c2GJK` | `outB == NULL` | `*outB` not written; return value unchanged | `row03_05_null_out_params_not_written` | [x] |
| 5  | `c2GJK` | `iterations == NULL` | `*iterations` not written; return value unchanged | `row03_05_null_out_params_not_written` | [x] |
| 6  | `c2GJK` | `cache == NULL` | cache neither read nor written; simplex seeded fresh (`count=1`, `div=1`) | `row06_07_cache_null_and_cold` | [x] |
| 7  | `c2GJK` | `cache != NULL` but `cache->count == 0` | `cache_was_good == 0` → read block skipped → fresh seed; the other cache fields are ignored however junky; cache IS written on exit; distance equals the `cache == NULL` distance | `row06_07_cache_null_and_cold` | [x] |
| 8  | `c2GJK` | `cache->count != 0` and `!(min<max*2 && metric < -1e8)` — i.e. essentially always | quirk: `cache_was_read = 1`, the stale simplex is trusted verbatim, no re-seed | `row08_cache_trusted_verbatim` | [x] |
| 9  | `c2GJK` | `cache->count != 0` and `min_metric < max_metric*2 && metric < -1.0e8f` | `cache_was_read` stays 0 → simplex re-seeded from vertex 0 → result identical to a cold cache. Reached with 1e5-scale AABBs (metric ≈ −1e10) | `row09_cache_reseed_on_huge_negative_metric` | [x] |
| 10 | `c2GJK` | warm cache with `cache->count == 3`, indices in range | cache read, `s.count == 3` on entry → first iteration hits `if (s.count==3)` → `hit=1`, `iter==0`, `dist==0`, `a==b` | `row10_warm_cache_count3_immediate_hit` | [x] |
| 11 | `c2GJK` | `cache->count < 0` (`-1`, `-2`, `-3`, `-1000`, `INT_MIN`) | copy loop never runs; `s.count` = the negative value; metric `default:`→0; `switch(s.count)` matches nothing; `count != 3`; `c2L` `default:`→(0,0); `c2D` `default:`→(0,0) so `dot(d,d)=0 < eps²` → break; `c2Witness` `default:` → `a=b=(0,0)`; `dist == 0`; `iter == 0`; the negative count is written back verbatim | `row11_cache_negative_count` | [x] |
| 12 | `c2GJK` | `cache->count == 4` | reads `cache->iA[3]`/`iB[3]`, which by struct layout ARE `cache->iB[0]` (bytes 20..24) and `cache->div` (bytes 32..36); writes `verts[3]` (= `s.d`, still in bounds). Fully deterministic when those aliased values are legal indices. `count==4` matches no `case`, so `iter == 0` | `row12_cache_count_four_struct_aliasing` | [x] |
| 13 | `c2GJK` | `cache->count > 4` | `verts[4]` spans bytes 144..180 of a 152-byte `c2Simplex` → a 36-byte write overruns the struct by 28 bytes and smashes `c2GJK`'s stack locals | `row13_cache_count_gt_four_documented_ub` | [UB] |
| 14 | `c2GJK` | `cache->iA[i]`/`iB[i]` ≥ the proxy's `count` but < 8 (e.g. index 3 with a `C2_TYPE_CIRCLE` proxy, whose `count` is 1) | index is inside `c2Proxy::verts[8]` so no OOB, but the slot was never written by `c2MakeProxy` → reads uninitialised stack. **Measured: the C returns different values for the same input on different runs** | `row14_15_cached_index_out_of_range_no_crash` | [UB] |
| 15 | `c2GJK` | `cache->iA[i]` ≥ 8 (8, 11, 15) | genuine out-of-bounds read past `c2Proxy::verts` | `row14_15_cached_index_out_of_range_no_crash` | [UB] |
| 16 | `c2GJK` | `typeA`/`typeB` is an out-of-range enum value (`3`, `4`, `5`, `7`, `42`, `255`, `0x80000000`, `0xFFFFFFFF`) | `c2MakeProxy`'s `switch` has no `default:` → the `c2Proxy` local stays wholly uninitialised, so `pA.count` is stack residue and `c2Support` walks that many vertices. **Measured: reliably SIGSEGVs** | `row16_bad_enum_into_gjk_documented_ub` | [UB] |
| 17 | `c2MakeProxy` | `type` not in `{0,1,2}` — 12 fixed values plus ~2000 random `u32 > 2` | `switch` falls through with no `default:` → **not one byte of `*p` is written**; a caller-prefilled `c2Proxy` comes back bit-identical, and the `shape` pointer is never read | `row17_bad_enum_makeproxy_writes_nothing` | [x] |
| 18 | `c2MakeProxy` | `shape == NULL` with a valid `type` | unconditional deref → SIGSEGV | `row18_19_24_null_deref_documented_ub` | [UB] |
| 19 | `c2BBVerts` | `out == NULL` or `bb == NULL` | unconditional deref → SIGSEGV | `row18_19_24_null_deref_documented_ub` | [UB] |
| 20 | `c2Support` | `count <= 0` (`0`, `-1`, `-100`, `INT_MIN`) | `verts[0]` is read *before* the loop guard, then `for (i=1; i<count)` never runs → returns `0` | `row20_23_support_degenerate` | [x] |
| 21 | `c2Support` | `count == 1` | loop never runs → returns `0` for any `d` | `row20_23_support_degenerate` | [x] |
| 22 | `c2Support` | `d == (0,0)`, or all vertices equal (every dot ties, never `> dmax`) | returns `0` — the first index wins ties | `row20_23_support_degenerate` | [x] |
| 23 | `c2Support` | one or all `verts[i]` are `NaN` | `dot > dmax` is false for NaN, so NaN candidates never win; if `verts[0]` is NaN then `dmax` is NaN and *no* later index can win → returns `0` | `row20_23_support_degenerate` | [x] |
| 24 | `c2Support` | `verts == NULL` | unconditional `verts[0]` load → SIGSEGV | `row18_19_24_null_deref_documented_ub` | [UB] |
| 25 | `c2Witness` | `s->count` not in `{1,2,3}` (`0`, `4`, `5`, `-1`, `INT_MIN`, `INT_MAX`) | `default:` → `*a = (0,0)`, `*b = (0,0)` | `row25_27_witness_degenerate` | [x] |
| 26 | `c2Witness` | `s->div == 0` (counts 1..3) | `den = 1.0f/0 = +inf`; `count==1` is unaffected; counts 2/3 give `±inf`, or `NaN` where `u == 0` (`inf * 0`) | `row25_27_witness_degenerate` | [x] |
| 27 | `c2Witness` | `s->div` is `-0.0`, `NaN` (incl. a non-canonical payload), `±inf` | `den` = `-inf` / quieted-NaN / `±0`; propagates into the weighted sums | `row25_27_witness_degenerate` | [x] |
| 28 | `c2L` | `s->count` not in `{1,2}` (`3`, `0`, `4`, `-1`, `INT_MIN`, `INT_MAX`) | `default:` → returns `(0,0)` | `row28_29_c2L_degenerate` | [x] |
| 29 | `c2L` | `s->div` is `0`, `-0.0`, `NaN`, `inf` with `count` 1..2 | `den` degenerate → components `±inf` / `NaN` | `row28_29_c2L_degenerate` | [x] |
| 30 | `c2D` | `s->count` == `3` or outside `{1,2}` | `default:` → returns `(0,0)` | `row30_31_c2D_degenerate` | [x] |
| 31 | `c2D` | `count == 2` and `a.p == b.p` (degenerate edge) | `c2Sub(p,p)` = `(+0.0,+0.0)`; `det2 == 0` which is NOT `> 0` → `c2CCW90((+0.0,+0.0))` = **`(+0.0, -0.0)`** — the sign of the second zero is observable | `row30_31_c2D_degenerate` | [x] |
| 32 | `c2GJKSimplexMetric` | `s->count` not in `{2,3}` (`1`, `0`, `4`, `5`, `-1`, `INT_MIN`, `INT_MAX`) | `default:` shares the `case 1:` body → returns `0` | `row32_33_metric_degenerate` | [x] |
| 33 | `c2GJKSimplexMetric` | `count == 2` with equal points; `count == 3` with all points equal | `c2Len((0,0)) = sqrtf(0) = 0`; `c2Det2((0,0),(0,0)) = 0` | `row32_33_metric_degenerate` | [x] |
| 34 | `c2Norm` | `a == (0,0)` (all four sign combinations of `±0.0`) | `c2Len = 0`; `1.0f/0 = +inf`; `0 * inf` → `NaN` in both components | `row34_38_norm_div_sentinels` | [x] |
| 35 | `c2Norm` | `a` has a `NaN` / `inf` component, or is 1e30 / 1e-30 scale | `c2Len` → `NaN`/`inf`; result `NaN` components | `row34_38_norm_div_sentinels` | [x] |
| 36 | `c2Div` | `b == 0` | `1.0f/0 = +inf` → components `±inf`, or `NaN` where the component is `0` | `row34_38_norm_div_sentinels` | [x] |
| 37 | `c2Div` | `b == -0.0` | `1.0f/-0.0 = -inf` → sign-flipped `inf` / `NaN` | `row34_38_norm_div_sentinels` | [x] |
| 38 | `c2Div` | `b` is `NaN` (canonical and non-canonical payload), `±inf`, or denormal | NaN payload propagates through `1.0f/b` then the multiply; `±inf` → `±0` scaling; denormal → overflow | `row34_38_norm_div_sentinels` | [x] |
| 39 | `c2Len` | `a` huge (`1e38`, `±FLT_MAX`) | `c2Dot` overflows to `+inf`; `sqrtf(+inf) = +inf` | `row39_40_len_extremes` | [x] |
| 40 | `c2Len` | `a` has a `NaN` component, `±inf`, or is denormal | `sqrtf(NaN) = NaN` (payload preserved, quieted); denormals do not underflow to a wrong root | `row39_40_len_extremes` | [x] |
| 41 | `c2Maxv` | either operand has a `NaN` component (incl. a signalling NaN) | `(a.x) > (b.x)` is false whenever a NaN is involved → **`b`'s component is returned**. With NaN on both sides the result is always `b` | `row41_43_minmax_nan_and_signed_zero` | [x] |
| 42 | `c2Minv` | either operand has a `NaN` component | `(a.x) < (b.x)` is false → **`b`'s component is returned** | `row41_43_minmax_nan_and_signed_zero` | [x] |
| 43 | `c2Maxv`/`c2Minv` | `+0.0` vs `-0.0`, both argument orders | neither `>` nor `<` holds → `b`'s component wins, so the *sign of the zero* is argument-order dependent | `row41_43_minmax_nan_and_signed_zero` | [x] |
| 44 | `c2Clampv` | `lo > hi` (inverted range, no validation) | no rejection: `c2Maxv(lo, c2Minv(a, hi))` collapses to **`lo`** | `row44_45_clamp_inverted_and_nan` | [x] |
| 45 | `c2Clampv` | `NaN` in `a`, in `lo`, in `hi`, and all 7 combinations; plus arbitrary bit patterns | the "b wins" rule of rows 41/42 composes through both calls | `row44_45_clamp_inverted_and_nan` | [x] |
| 46 | `c22` | `a.p == b.p` (degenerate; `u == v == 0`) | `v <= 0` is true → collapses to `count = 1`, `a.u = 1`, `div = 1` | `row46_47_c22_degenerate` | [x] |
| 47 | `c22` | `a.p`/`b.p` contain `NaN` or `±inf` (so `u`, `v` are NaN) | `v <= 0` false and `u <= 0` false → `else` branch: `div = NaN`, `count = 2` | `row46_47_c22_degenerate` | [x] |
| 48 | `c23` | all three points equal (every `u`/`v`/`area` is 0) | `vAB <= 0 && uCA <= 0` is true → `count = 1`, `div = 1` | `row48_50_c23_degenerate` | [x] |
| 49 | `c23` | collinear points (`area == 0` → `uABC == vABC == wABC == 0`) | one of the `<= 0` guards fires; if the `else` is reached, `div = 0` and `count = 3` (feeding row 26). Measured: the guards always fire first for the sampled geometry | `row48_50_c23_degenerate` | [x] |
| 50 | `c23` | points contain `NaN` (all comparisons false) | falls through every guard to the `else` → `div = NaN`, `count = 3` | `row48_50_c23_degenerate` | [x] |
| 51 | `c2GJK` | the iteration counter and its `while (iter < 20)` cap | both libraries must report the same `*iterations` for every input. Measured over ~162 000 configurations (uniform, "spicy", arbitrary-bit and warm-cache shapes across all 9 type pairs): the observed distribution is `iter ∈ {0,1,2,3,4}` with a maximum of **4**, so the hard cap of 20 is not reachable through the public API — the simplex always terminates via `count==3`/`d1>d0`/`dot(d,d)<eps²`/`dup` first | `row51_iteration_counter_agrees` | [x] |
| 52 | `c2GJK` | `use_radius != 0` and `dist <= rA + rB` (overlap), or `dist <= FLT_EPSILON` | else-branch: `a = b = 0.5f*(a+b)`, `dist = 0` (verified: `a == b` bit-for-bit) | `row52_55_use_radius_branches` | [x] |
| 53 | `c2GJK` | `use_radius != 0` and `dist > rA + rB && dist > FLT_EPSILON` | the shrink branch runs; if it makes the points coincide, `dist` is forced to `0` | `row52_55_use_radius_branches` | [x] |
| 54 | `c2GJK` | `use_radius != 0` and `rA + rB` is `NaN` or overflows to `inf` | `dist > rA+rB` is false for NaN/inf → midpoint branch, `dist = 0` | `row52_55_use_radius_branches` | [x] |
| 55 | `c2GJK` | `use_radius` is a non-canonical truthy `int` (`2`, `-1`, `INT_MIN`, `INT_MAX`, `0x100`) | C tests truthiness only → every non-zero value gives a bit-identical result to `1` (asserted) | `row52_55_use_radius_branches` | [x] |
| 56 | `c2GJK` | degenerate AABB with `min > max` on one or both axes | no validation; `c2BBVerts` produces a reversed winding → negative `area`, sign-flipped `c2Det2`; still returns a finite `dist` | `row56_58_degenerate_shapes` | [x] |
| 57 | `c2GJK` | capsule with `a == b`; zero radius; `-0.0` radius; negative radius (circle and capsule) | no validation; a negative `rA + rB` makes the `dist > rA+rB` branch fire and `dist` *grow* | `row56_58_degenerate_shapes` | [x] |
| 58 | `c2GJK` | all shape floats are `NaN` / `±inf` (circle, AABB, capsule; on A, on B, on both) | NaN propagates; the loop exits via the `dup`/`dot(d,d)` tests; result is `NaN` or `0` | `row56_58_degenerate_shapes` | [x] |
| 59 | `gjk_cache` | `reverse` = `0`, `1`, `-1`, `2`, `127`, `-128`, `0x40`, `-0x40`, `'A'`, `3` | `char` truthiness only: any non-zero swaps which shape is A vs B in the final `c2GJK` | `row59_60_gjk_cache_no_op` | [x] |
| 60 | `gjk_cache` | `a9`/`b9` are `NULL`; any float argument is `NaN`/`inf`/arbitrary bits | **`a9` and `b9` are never dereferenced by the C** → no crash, no writes, `void` return; asserted against pre-poisoned buffers plus canaries either side | `row59_60_gjk_cache_no_op` | [x] |

## Generic FFI boundaries (beyond the table)

`tests/phase_c_errors.rs::generic_boundaries` additionally sweeps, for both
libraries at once:

* every `C2_TYPE` value `0..=8` through `c2MakeProxy` (valid *and* out of range);
* `c2Support` with `count` = `0`, `1`, `8` (the proxy array size) and `9`
  (one past it);
* simplex `count` = `-1, 0, 1, 2, 3, 4, 5` — one step past every documented
  case — through `c2L`, `c2D`, `c2GJKSimplexMetric`, `c2Witness`, `c22`, `c23`;
* `c2GJK` with `use_radius` = `-1, 0, 1, 2, INT_MIN, INT_MAX`;
* `c2GJK` with `cache->count` = `-1, 0, 1, 2, 3`.

`abi_layout_matches_c` pins every struct size (`c2v` 8, `c2r` 8, `c2x` 16,
`c2Circle` 12, `c2AABB` 16, `c2Capsule` 20, `c2GJKCache` 36, `c2Proxy` 72,
`c2sv` 36, `c2Simplex` 152) and `sizeof(int) == sizeof(unsigned) == 4`, so the
"same error value" assertions above really are comparing the same bytes.

## UB rows — what is asserted instead

Rows 13, 14, 15, 16, 18, 19 and 24 are **undefined behaviour in the C**:
uninitialised reads, out-of-bounds reads/writes, or NULL dereferences. A
bit-equality assertion is meaningless for them because the C has no
reproducible result — this was *measured*, not assumed:

* rows 14/15: **11 882** of the sampled out-of-range-index configurations gave
  different answers between the two libraries, and the C returned *different
  values for the same input on different runs* (e.g. `1106020886` then
  `1102845220`), because it reads uninitialised `c2Proxy::verts` slots;
* row 16: calling `c2GJK` with an out-of-range enum **reliably SIGSEGVs** the C,
  since the uninitialised `pA.count` reaches `c2Support` unchecked;
* row 13: `verts[4]` overruns `c2Simplex` by 28 bytes and smashes `c2GJK`'s own
  stack locals.

They are therefore covered as follows:

* **row 17 asserts the well-defined half of row 16** exhaustively — that
  `c2MakeProxy` writes *nothing* for an out-of-range enum — using
  caller-owned, pre-poisoned proxies and ~2000 random out-of-range `u32`
  values. That is the entire enum behaviour the C actually defines.
* **rows 14/15 are executed** for both libraries (`run_both_no_assert`) to
  confirm neither aborts, and the divergence count is reported.
* **rows 13, 18, 19, 24 are deliberately not executed in-process**: they would
  either SIGSEGV the harness or corrupt its stack. Each has a test that pins
  down the structural premise it rests on (struct sizes for row 13; the absence
  of any count/NULL guard in `c2Support` for row 24).
* Phase B keeps the *defined* paths clean by folding carried-over cache indices
  back into range (`clamp_cache`), so rows 54/56 of `CONFIGS.md` test the
  documented behaviour rather than accidental UB.
