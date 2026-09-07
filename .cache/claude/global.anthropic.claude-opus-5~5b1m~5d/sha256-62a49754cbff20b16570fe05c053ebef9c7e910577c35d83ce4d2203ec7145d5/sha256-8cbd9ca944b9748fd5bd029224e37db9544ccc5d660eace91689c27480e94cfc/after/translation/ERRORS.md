# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. This library has **no** error enum,
no `RETURN_ERROR` macro, no `assert`, and no `errno` usage. Its entire rejection
surface consists of:

* `default: return 0;` sentinel branches in `c2Collided` (4 of them),
* `switch` statements **without** a `default` label, where an unmatched value is
  a silent no-op / fall-through (`c2MakeProxy`, `ptr_from_parts`),
* `switch` `default:` labels that produce a fixed neutral value
  (`c2GJKSimplexMetric` → `0`, `c2D` → `(0,0)`, `c2L` → `(0,0)`, `c2Witness` → `(0,0)`),
* NULL-pointer *tolerance* checks in `c2GJK` (`!ax_ptr`, `!bx_ptr`, `if (outA)`,
  `if (outB)`, `if (iterations)`, `if (cache)`),
* numeric guards / degenerate-input clamps (`div`-by-zero in `c2Div`/`c2Norm`,
  `c2Dot(d,d) < FLT_EPSILON^2`, `dist > rA+rB && dist > FLT_EPSILON`,
  `d1 > d0`, `iter < 20`, `cache->count` truthiness, the `metric` window test).

Grep commands used to derive the rows:

```sh
grep -n 'return 0\|return NULL\|default\|assert\|if (!\|if (.*)$\|<= 0\|< 0\|EPSILON\|FLT_MAX\|< 20' c_src/src/lib.c
```

Every row below is exercised by a differential test in
`translation/tests/errors.rs` (test name in the last column).

| #  | function | trigger (exact invalid input/condition) | expected C result | test |
|----|----------|------------------------------------------|-------------------|------|
| 1  | `c2Collided` | `typeA` not in {0,1,2} (outer `switch` `default:`) — e.g. `3`, `-1`, `INT_MIN`, `INT_MAX` | returns `0`; `A`/`B` never dereferenced | `err_collided_bad_typeA` |
| 2  | `c2Collided` | `typeA == C2_TYPE_CIRCLE(1)` and `typeB` not in {0,1,2} (inner `default:`) | returns `0` | `err_collided_circle_bad_typeB` |
| 3  | `c2Collided` | `typeA == C2_TYPE_AABB(2)` and `typeB` not in {0,1,2} (inner `default:`) | returns `0` | `err_collided_aabb_bad_typeB` |
| 4  | `c2Collided` | `typeA == C2_TYPE_CAPSULE(0)` and `typeB` not in {0,1,2} (inner `default:`) | returns `0` | `err_collided_capsule_bad_typeB` |
| 5  | `omni_collide` | `type_a` and/or `type_b` out of range (propagates rows 1–4 through `ptr_from_parts`) | returns `0` (no crash: the indeterminate pointer is never dereferenced) | `err_omni_collide_bad_enums` |
| 6  | `ptr_from_parts` | `typ` not in {0,1,2}: `switch` has no `default` and the function falls off the end of a non-`void` function | C: indeterminate return value (UB), never dereferenced by any caller. Rust returns `NULL`. Only the *observable* consequence (row 5) is asserted. | `err_ptr_from_parts_bad_type` (documented, non-asserting on the pointer value) |
| 7  | `c2MakeProxy` | `type` not in {0,1,2}: `switch` has no `default` | proxy struct left **completely untouched** (`radius`/`count`/`verts` keep their prior contents) | `err_make_proxy_bad_type_leaves_proxy_untouched` |
| 8  | `c2GJKSimplexMetric` | `s->count` is `0`, `1`, or anything not 2/3 (`default:` falls into `case 1:`) | returns `0.0f` | `err_simplex_metric_bad_count` |
| 9  | `c2D` | `s->count == 3` or any other value (`case 3:`/`default:`) | returns `c2V(0,0)` | `err_c2D_count_3_and_out_of_range` |
| 10 | `c2L` | `s->count` not 1 or 2 (`default:`) | returns `c2V(0,0)` | `err_c2L_bad_count` |
| 11 | `c2L` | `s->div == 0` with `count == 2` → `den = 1/0 = +inf` | returns `inf`/`NaN` components (bit-identical between C and Rust) | `err_c2L_zero_div` |
| 12 | `c2Witness` | `s->count` not 1/2/3 (`default:`) | writes `c2V(0,0)` to **both** `*a` and `*b` | `err_witness_bad_count` |
| 13 | `c2Witness` | `s->div == 0` → `den = +inf` | `inf`/`NaN` outputs, bit-identical | `err_witness_zero_div` |
| 14 | `c2Support` | `count <= 0` (`0`, negative): the loop is skipped but `verts[0]` is still read unconditionally | returns `0` | `err_support_nonpositive_count` |
| 15 | `c2Div` | `b == 0` → `1.0f/0.0f = +inf`, multiplied into both components | `(±inf, ±inf)` or `NaN` for a zero component; bit-identical | `err_div_by_zero` |
| 16 | `c2Norm` | `a == (0,0)` → `c2Len == 0` → division by zero | `(NaN, NaN)` | `err_norm_zero_vector` |
| 17 | `c2Len` | `c2Dot(a,a)` overflows to `+inf` (huge components) / is `NaN` | `sqrtf(inf) = inf`, `sqrtf(NaN) = NaN`; also `sqrtf` of a negative never happens since `dot(a,a) >= 0` unless NaN | `err_len_overflow_and_nan` |
| 18 | `c2GJK` | `ax_ptr == NULL` → `ax = c2xIdentity()` | identity transform used, no crash | `err_gjk_null_transforms` |
| 19 | `c2GJK` | `bx_ptr == NULL` → `bx = c2xIdentity()` | identity transform used, no crash | `err_gjk_null_transforms` |
| 20 | `c2GJK` | `outA == NULL` (guarded by `if (outA)`) | nothing written; return value still valid | `err_gjk_null_outputs` |
| 21 | `c2GJK` | `outB == NULL` (guarded by `if (outB)`) | nothing written | `err_gjk_null_outputs` |
| 22 | `c2GJK` | `iterations == NULL` (guarded by `if (iterations)`) | nothing written | `err_gjk_null_outputs` |
| 23 | `c2GJK` | `cache == NULL` (guarded by `if (cache)`) | cache neither read nor written | `err_gjk_null_outputs` |
| 24 | `c2GJK` | `cache->count == 0` → `cache_was_good == 0`, cache **not** read, simplex re-initialised from vertex 0 | fresh start; cache written back on exit | `err_gjk_cache_count_zero` |
| 25 | `c2GJK` | `cache->count != 0` but the metric window test `!(min < max*2 && metric < -1e8)` is **true** → `cache_was_read = 1` | warm-started simplex is used verbatim | `err_gjk_cache_warm_start` |
| 26 | `c2GJK` | `cache->metric` = `NaN` (all comparisons false → `min = max = metric_old`) | `cache_was_read = 1` (window test true) | `err_gjk_cache_nan_metric` |
| 27 | `c2GJK` | `typeA`/`typeB` out of range → `c2MakeProxy` no-op → proxy `count == 0`, `verts[0]` is whatever the *uninitialised* stack slot holds | indeterminate but **must not** be papered over; the Rust must at minimum not diverge in the observable `count == 0` path. Documented; only the deterministic sub-case (`count==0` reached via a zeroed proxy) is asserted. | `err_gjk_bad_type_documented` |
| 28 | `c2GJK` | `use_radius == 0` → the whole radius-adjustment block is skipped | raw simplex distance returned; `a`/`b` are the raw witness points | `err_gjk_use_radius_zero` |
| 29 | `c2GJK` | `use_radius != 0` and `dist <= rA + rB` (overlapping) → else-branch: `a = b = midpoint`, `dist = 0` | returns exactly `0.0f`, `a == b == (a+b)/2` | `err_gjk_radius_overlap_branch` |
| 30 | `c2GJK` | `use_radius != 0` and `dist <= FLT_EPSILON` (coincident shapes) → same else-branch | returns `0.0f` | `err_gjk_radius_epsilon_branch` |
| 31 | `c2GJK` | radius shrink makes `a.x == b.x && a.y == b.y` → `dist = 0` (overwrites the just-computed positive `dist`) | returns `0.0f` despite the subtraction | `err_gjk_radius_collapse` |
| 32 | `c2GJK` | `hit == 1` (simplex reached `count == 3`) → `a = b`, `dist = 0` | returns `0.0f`, `outA == outB` | `err_gjk_hit_branch` |
| 33 | `c2GJK` | early exit `if (d1 > d0) break;` (non-monotonic descent) | loop exits with `iter` < 20 | `err_gjk_iteration_bounds` |
| 34 | `c2GJK` | search direction degenerate: `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` → break | loop exits early | `err_gjk_iteration_bounds` |
| 35 | `c2GJK` | duplicate support vertex (`iA==saveA[i] && iB==saveB[i]`) → break **before** `++s.count` | the newly written `verts[s.count]` is left dangling and *not* counted | `err_gjk_dup_break` |
| 36 | `c2GJK` | `iter` reaches the hard cap `20` | `*iterations <= 20`, identical in both | `err_gjk_iteration_bounds` |
| 37 | all shape fns | `NaN` / `±inf` in any coordinate or radius | every comparison is a plain IEEE compare; `NaN` makes `<`/`>` false → the specific fallback branch is taken. Must be bit-identical. | `err_nan_inf_inputs` |
| 38 | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` → `n == (0,0)`, `da == 0` → `da < 0` false, `db == 0` → `db < 0` false → `d2 = dot(bp,bp)` (the `n`-divide branch is skipped, avoiding 0/0) | well-defined result | `err_degenerate_capsule` |
| 39 | `c2CircletoCapsule` | `A.r + B.r` negative (negative radii) → `r*r` positive → may report a collision | replicate exactly, no clamping | `err_negative_radii` |
| 40 | `c2CircletoCircle` / `c2CircletoAABB` | negative radius → `r2 = r*r >= 0`, `d2 < r2` may be true | replicate exactly | `err_negative_radii` |
| 41 | `c2AABBtoAABB` / `c2CircletoAABB` | inverted AABB (`min > max`) → no validation; `c2Clampv` yields `max(min, min(p,max))` | replicate exactly | `err_inverted_aabb` |
| 42 | `c2BBVerts` | writes exactly 4 `c2v`; a caller-supplied buffer shorter than 4 is an overflow the C does not check | 4 elements always written | `err_bbverts_writes_exactly_four` |

---

## Phase C status: ALL 42 ROWS CHECKED OFF ✅

`tests/errors.rs` — 38 tests, 38 passed, 0 failed. Row → test mapping is the
last column of the table above; several rows share a test where the C shares a
single guard (e.g. rows 18/19 are the two halves of one `if (!ax_ptr)` pair, and
rows 20–23 are the four `if (out…)` guards exercised together over all 8 NULL
masks).

Each test asserts the **specific** sentinel, not merely "both failed":

* rows 1–5 assert the return is exactly `0`,
* row 8 asserts exactly `+0.0` (`0x00000000`, not `-0.0`),
* rows 9, 10, 12 assert exactly `c2V(0,0)` in every output slot,
* rows 29–32 assert `dist` is exactly `+0.0` **and** `outA == outB`,
* row 7 asserts the proxy is byte-for-byte unchanged from a distinctive pre-fill,
* row 42 asserts guard slots past vertex 4 are untouched.

Tests that exercise a rare branch also assert the branch was actually reached
(`assert!(hits > …)`), so a row cannot be silently checked off by a test that
never got there:

| guard | row(s) | observed |
|-------|--------|----------|
| `saw_nonfinite > 0` | 11, 13 | non-finite results produced by `div == 0` |
| `saw_inf > 0 && saw_nan > 0` | 15, 17 | both overflow and NaN produced |
| `differed_from_cold > 0` | 25 | the warm start actually changed the outcome |
| `differed > 0` | 28 | `use_radius` actually changed the outcome |
| `hits > 100` | 29, 30, 32 | branch reached thousands of times |
| `collapses > 0` | 31 | the `a == b` collapse actually triggered |
| `distinct >= 3` | 33, 34, 36 | ≥3 distinct loop-exit iteration counts observed |
| `reported > 0` | 39, 40 | negative radii actually reported a collision |
| branch counters | 24, 25 (`c22`/`c23`), 50 | every `if`/`else` arm reached |

### Two rows are *deliberately* not asserted for equality (rows 6 and 27)

Both are cases where the C's own result is **indeterminate**, so demanding
byte-identical output would mean asserting on undefined behaviour:

* **Row 6** `ptr_from_parts` with an unknown `typ`: the `switch` has no
  `default` and the function falls off the end of a non-`void` function. The
  value in `rax` is whatever was left there. The test asserts only what is
  well-defined — that Rust deterministically yields `NULL`, that neither library
  crashes, and that the single observable consequence (`omni_collide` returning
  `0` without dereferencing it, row 5) matches.
* **Row 27** `c2GJK` with an unknown type: `c2MakeProxy` is a no-op, so the C's
  local `c2Proxy pA;` retains uninitialised stack contents. The test asserts
  neither library crashes or hangs and that `*iterations` stays inside `[0, 20]`
  in both. This path is unreachable through the public API, because
  `c2Collided`/`omni_collide` return `0` for an unknown type *before* calling
  `c2GJK` at all (asserted in rows 1–5).

Everything else in the table is asserted for exact equality.
