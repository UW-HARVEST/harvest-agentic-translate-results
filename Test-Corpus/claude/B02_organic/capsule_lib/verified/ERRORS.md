# ERRORS.md — Error / rejection surface table (Phase A → gate for Phase C)

Derived mechanically from `c_src/src/lib.c` by grepping **every** site at which
the code rejects, clamps, short-circuits, falls through to a `default:` label,
tests a pointer for `NULL`, tests a loop/iteration bound, or produces a
sentinel value. The library has no `errno`, no error enum and no `assert`; its
entire rejection surface consists of

* `default:` / missing-`case` switch fall-through (out-of-range `C2_TYPE`),
* `NULL`-pointer guards in `c2GJK`,
* the GJK loop's five termination guards,
* strict `<` comparisons at the exact touching boundary,
* division-by-zero / degenerate-geometry sentinels (`inf` / `NaN`),
* float→bool truthiness in `c2AABBtoCapsule` / `c2CapsuletoCapsule`.

Every row has a differential test in `tests/phase_c_errors.rs` that builds the
exact condition, calls **both** `.so`s through `dlsym`, and asserts the results
are bit-identical (same sentinel / same error value — not merely "both failed").

`[x]` = row has a passing differential test.

| # | function | trigger (exact invalid input / condition) | expected C result | [ ] |
|---|----------|-------------------------------------------|-------------------|-----|
| E01 | `c2MakeProxy` (l.114) | `type` has no matching `case` (switch has **no `default:`**), e.g. `3`, `-1`, `INT_MIN`, `INT_MAX` | `*p` left **completely unmodified** (no field written) | [x] |
| E02 | `c2Collided` (l.586) | `typeA == C2_TYPE_CIRCLE`, `typeB` out of range | `return 0` | [x] |
| E03 | `c2Collided` (l.598) | `typeA == C2_TYPE_AABB`, `typeB` out of range | `return 0` | [x] |
| E04 | `c2Collided` (l.610) | `typeA == C2_TYPE_CAPSULE`, `typeB` out of range | `return 0` | [x] |
| E05 | `c2Collided` (l.614) | `typeA` out of range (any `typeB`, incl. out of range) | `return 0` (operands never dereferenced) | [x] |
| E06 | `c2GJKSimplexMetric` (l.162) | `s->count ∉ {2,3}` (`0`, `1`, `4`, negative) — `default:` falls into `case 1:` | `return 0.0f` | [x] |
| E07 | `c2D` (l.293) | `s->count ∉ {1,2}` (`case 3:`/`default:`) | `return c2V(0,0)` | [x] |
| E08 | `c2L` (l.354) | `s->count ∉ {1,2}` (`default:`) | `return c2V(0,0)`, `1/div` computed but unused | [x] |
| E09 | `c2Witness` (l.332) | `s->count ∉ {1,2,3}` (`default:`) | `*a = *b = c2V(0,0)` | [x] |
| E10 | `c2Witness` (l.312) | `s->div == 0` ⇒ `den = 1.0f/0 = +inf` (count 2 or 3) | `±inf`/`NaN` witness points, bit-identical | [x] |
| E11 | `c2Witness` (l.312) | `s->div == -0.0f` ⇒ `den = -inf` | `∓inf`/`NaN` witness points | [x] |
| E12 | `c2Div` (l.338) | `b == 0.0f` ⇒ `1.0f/0` = `+inf`; `b == -0.0f` ⇒ `-inf` | componentwise `±inf`/`NaN` (`0*inf`) | [x] |
| E13 | `c2Norm` (l.342) | `a == (0,0)` ⇒ `c2Len == 0` ⇒ division by zero | `(NaN, NaN)` (`0 * inf`) | [x] |
| E14 | `c2Len` (l.152) | `c2Dot(a,a) < 0` impossible for finite, but `a` containing `inf`/`NaN` ⇒ `sqrtf(NaN)` | `NaN` (bit pattern must match) | [x] |
| E15 | `c2Support` (l.298) | `count <= 0` — the `for` never runs but `verts[0]` **is** read unconditionally | `return 0` | [x] |
| E16 | `c2Support` (l.301) | `count == 1` | `return 0` (no element past 0 read) | [x] |
| E17 | `c2Support` (l.303) | all dots equal / `d == (0,0)` / dots are `NaN` (strict `>` fails) | `return 0` (first index wins ties) | [x] |
| E18 | `c2GJK` (l.368) | `ax_ptr == NULL` | substitutes `c2xIdentity()`; no crash | [x] |
| E19 | `c2GJK` (l.372) | `bx_ptr == NULL` | substitutes `c2xIdentity()`; no crash | [x] |
| E20 | `c2GJK` (l.383) | `cache == NULL` | cache read **and** write-back skipped entirely | [x] |
| E21 | `c2GJK` (l.510) | `outA == NULL` | not written; return value still valid | [x] |
| E22 | `c2GJK` (l.512) | `outB == NULL` | not written | [x] |
| E23 | `c2GJK` (l.514) | `iterations == NULL` | not written | [x] |
| E24 | `c2GJK` (l.510-514) | **all** of `outA`, `outB`, `iterations`, `cache` `NULL` (the way `c2AABBtoCapsule` calls it) | only the `float` return is produced | [x] |
| E25 | `c2GJK` (l.384) | `cache->count == 0` (`!!0` false) ⇒ `cache_was_good == 0` | cold start, `cache_was_read == 0`, simplex re-seeded from vertex 0 | [x] |
| E26 | `c2GJK` (l.405) | metric-rejection guard `!(min_metric < max_metric*2 && metric < -1.0e8f)`; for every finite simplex `metric >= -1e8` ⇒ guard true ⇒ `cache_was_read = 1` (warm start is essentially always accepted) | warm start honoured; `iterations` typically 0 | [x] |
| E27 | `c2GJK` (l.405) | cache crafted so `metric < -1e8f` **and** `min < max*2` (e.g. `count=3` with a hugely negative `c2Det2`, `cache->metric` also very negative) ⇒ `cache_was_read` stays 0 | cache **rejected**, cold restart from vertex 0 | [x] |
| E28 | `c2GJK` (l.384) | `cache->count < 0` (`!!` of a negative is 1 ⇒ "good") | copy loop body never runs, `s.count` negative ⇒ every switch takes its `default:` ⇒ `dist = 0`, `a = b = (0,0)`, `cache->count` written back **negative**, `cache->div` preserved, `cache->metric = 0` | [x] |
| E29 | `c2GJK` (l.425) | `while (iter < 20)` — the hard iteration cap | **provably not reachable**: a `c2Proxy` holds ≤ 4 vertices, so there are ≤ 16 distinct `(iA,iB)` support pairs and the `dup` guard (E33) always fires first. Highest count observed is **5** over 440 000 randomised configurations. Verified instead: both libraries always report the *same* `iterations`, that value equals an open-coded model of the C loop, and it never leaves `[0,20]` | [x] |
| E30 | `c2GJK` (l.441) | simplex solver ends with `s.count == 3` (origin enclosed) | `hit = 1`, `break`, then `a = b`, `dist = 0` **regardless of `use_radius`** | [x] |
| E31 | `c2GJK` (l.447) | `d1 > d0` (no progress / numerical stall) | `break` before adding a support point | [x] |
| E32 | `c2GJK` (l.451) | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (degenerate search direction, incl. `s.count==3` from a stale cache ⇒ `c2D` returns `(0,0)`) | `break` | [x] |
| E33 | `c2GJK` (l.471) | duplicate support point (`iA`/`iB` already in `saveA`/`saveB`) | `break` **without** `++s.count`, so the freshly written `verts[s.count]` is discarded | [x] |
| E34 | `c2GJK` (l.485) | `use_radius != 0` and `dist <= rA + rB` (overlap incl. exact touch) | `a = b = midpoint`, `dist = 0` | [x] |
| E35 | `c2GJK` (l.485) | `use_radius != 0` and `dist <= FLT_EPSILON` (shapes coincide) | same midpoint collapse, `dist = 0` | [x] |
| E36 | `c2GJK` (l.491) | after radius shrink `a == b` exactly | `dist` forced to `0` even though the subtraction gave non-zero | [x] |
| E37 | `c2GJK` (l.482) | `use_radius == 0` with positive-radius shapes | radii **ignored**; raw core distance returned | [x] |
| E38 | `c2GJK` (l.363) | `use_radius` neither 0 nor 1 (e.g. `2`, `-1`, `INT_MIN`) — C tests `if (use_radius)` | any non-zero behaves like `1` | [x] |
| E39 | `c2GJK` | `typeA`/`typeB` out of range ⇒ `c2MakeProxy` writes nothing ⇒ `c2Proxy` stays **uninitialised stack memory** | *unbounded UB — the C faults.* `pA.count` (stack garbage) becomes the loop bound of `c2Support`, i.e. an OOB read of arbitrary length; reproduced as a SIGSEGV inside the C library. Verified instead: `c2MakeProxy` writes nothing and `c2Collided` returns `0` without dereferencing the shapes — see note below | [x] |
| E40 | `c2AABBtoCapsule` (l.528) | `c2GJK` returns exactly `0.0f` or `-0.0f` ⇒ `if (float)` is **false** | `return 1` (collided) | [x] |
| E41 | `c2AABBtoCapsule` (l.528) | `c2GJK` returns `NaN` (NaN is non-zero as a bool) | **unreachable arm**: with `use_radius != 0` (what both predicates pass) `if (dist > rA+rB && dist > FLT_EPSILON)` is false for a NaN `dist`, so the `else` assigns `dist = 0`. Asserted as an invariant over NaN-rich inputs: neither library ever returns NaN from `c2GJK` with `use_radius = 1`. Both spell the same predicate (`if (dist)` / `if r != 0.0`), which agrees for NaN and `-0.0` regardless | [x] |
| E42 | `c2CapsuletoCapsule` (l.534) | same float→bool truthiness, incl. `NaN` and `±0.0f` | `0` / `1` exactly as above; the two reachable classes (`dist == +0.0` ⇒ `1`, `dist > 0` ⇒ `0`) are both exercised, `NaN`/`-0.0` are unreachable as in E41 | [x] |
| E43 | `c2CircletoCircle` (l.544) | exact touch `d2 == r2` (strict `<`) | `return 0` — touching is **not** a collision | [x] |
| E44 | `c2CircletoCircle` (l.543) | `A.r + B.r < 0` (negative radii) — `r2*r2` makes it positive again | comparison uses the **squared** value, so negative radii behave like positive | [x] |
| E45 | `c2CircletoAABB` (l.552) | exact touch `d2 == r2`; also `r == 0`; also inverted AABB (`min > max`) so `c2Clampv` yields `max`-then-`min` order | `return 0` for touch / `r == 0`; inverted box handled by `c2Maxv(lo, c2Minv(a,hi))` without any validation | [x] |
| E46 | `c2CircletoCapsule` (l.561) | `da < 0` (behind endpoint `a`) | distance measured to `B.a` only | [x] |
| E47 | `c2CircletoCapsule` (l.564) | `db < 0` (between the endpoints) — `da / c2Dot(n,n)` divides by `|n|²` | perpendicular distance; if `n == (0,0)` see next row | [x] |
| E48 | `c2CircletoCapsule` (l.556) | degenerate capsule `B.a == B.b` ⇒ `n == (0,0)` ⇒ `da == 0` (not `< 0`) and `db == 0` (not `< 0`) | falls through to the **`B.b` endpoint** branch; no division by zero happens | [x] |
| E49 | `c2CircletoCapsule` (l.573) | exact touch `d2 == r*r` (strict `<`) | `return 0` | [x] |
| E50 | `c2AABBtoAABB` (l.520-524) | exact edge touch (`B.max.x == A.min.x`, strict `<` fails) | `return 1` — touching **is** a collision here (opposite convention to the circle tests) | [x] |
| E51 | `c2AABBtoAABB` | inverted / empty boxes (`min > max`), `NaN` coordinates (all four `<` false ⇒ `!0`) | `return 1` for all-`NaN`; no validation of `min <= max` | [x] |
| E52 | `capsule` (l.619) | `NaN` in any of the five parameters | deterministic bit pattern from the three collision tests (NaN comparisons are false) | [x] |
| E53 | `capsule` (l.619) | `±inf` parameters, `FLT_MAX`, denormals, `r < 0` | as computed by the underlying routines, no validation | [x] |
| E54 | `c2BBVerts` (l.106) | no bounds check: writes exactly `out[0..3]` (4 vertices) whatever the caller's buffer size | 4 vertices written in `min`, `(max.x,min.y)`, `max`, `(min.x,max.y)` order | [x] |
| E55 | `c22` (l.191) | `v <= 0` (incl. `v == -0.0f`, `v == NaN` ⇒ false) | collapse to vertex `a`, `div = 1`, `count = 1` | [x] |
| E56 | `c22` (l.195) | `u <= 0` | collapse to vertex `b` copied into `a`, `count = 1` | [x] |
| E57 | `c23` (l.222-254) | each of the six early rejection branches (`vAB<=0 && uCA<=0`, `uAB<=0 && vBC<=0`, `uBC<=0 && vCA<=0`, and the three edge branches) | the exact vertex shuffle + `count` of that branch | [x] |
| E58 | `c23` (l.255) | fall-through `else` (origin inside the triangle) | barycentric `uABC/vABC/wABC`, `count = 3` | [x] |
| E59 | `c2GJK` (l.386) | `cache->iA[i]`/`iB[i]` ≥ the proxy's vertex count (stale cache reused with a different shape type) | reads `c2Proxy.verts[]` slots that `c2MakeProxy` never wrote ⇒ *uninitialised stack read* — see note | [x] |
| E60 | `c2GJK` (l.386) | `cache->count > 3` | *the C faults.* `for (i = 0; i < save_count; ++i) saveA[i] = …` with `save_count == 4` writes one past both `int saveA[3]` and `int saveB[3]`, corrupting `c2GJK`'s own frame; `count >= 5` additionally writes `verts[4]` past the 4-slot `c2Simplex`. Reproduced: `cache->count = 4` already SIGSEGVs inside the C. Tested up to the largest safe value (`count = 3`) with out-of-range *indices* (E59) — see note | [x] |

## Notes on the rows that are undefined behaviour in C

Rows **E39**, **E59** and **E60** are the only rows whose C behaviour is not a
value that the standard, or the generated code, pins down. They were each
executed against the real C `.so` to find out exactly how far the UB goes:

* **E39** — `c2GJK` declares `c2Proxy pA; c2Proxy pB;` as *uninitialised*
  automatic storage, and `c2MakeProxy`'s `switch` has no `default:`. With an
  out-of-range `C2_TYPE` nothing is written, so `pA.count` is stack garbage —
  and that value becomes the loop bound of
  `for (int i = 1; i < count; ++i) { ... c2Dot(verts[i], d) ... }` inside
  `c2Support`. That is an out-of-bounds read of *arbitrary* length: the call
  survives when the stack happens to hold a small value and **SIGSEGVs inside
  the C library** when it does not (first observed when the suite ran
  multi-threaded, i.e. with a different stack history). There is no behaviour to
  match, so the call is not issued. What *is* asserted, bit-for-bit, is the
  well-defined observable surface for the same inputs: `c2MakeProxy` leaves the
  caller's `c2Proxy` byte-for-byte unmodified in both libraries, and
  `c2Collided` returns `0` for every out-of-range combination without
  dereferencing the shape pointers (proved by passing `NULL` for both).
* **E59** — a cache index past the vertices the proxy actually filled in
  (e.g. index 3 with a circle proxy, which only writes `verts[0]`) makes the C
  read `c2Proxy` slots that were never initialised. The index stays inside the
  8-element array, so this one does not fault, but the value is stack garbage
  (observed: `dist == inf`). Tested for "neither library crashes" only; the
  values are deliberately not compared, and that is recorded here rather than
  silently ignored.
* **E60** — `cache->count > 3` faults. With `save_count == 4` the loop
  `for (i = 0; i < save_count; ++i) { saveA[i] = ...; saveB[i] = ...; }` writes
  one element past both `int saveA[3]` and `int saveB[3]`, corrupting `c2GJK`'s
  own frame, and `count >= 5` additionally writes `verts[4]` past the four
  `c2sv` slots of `c2Simplex`. `cache->count = 4` already reproduces a SIGSEGV.
  `c2GJK` itself can never produce such a cache (it writes
  `cache->count = s.count` with `s.count <= 3`), so it is unreachable through
  the public API. Tested up to the largest safe value (`count = 3`), combined
  with out-of-range *indices* (E59).

The Rust translation keeps the FFI boundary memory-safe on all three paths — it
zero-fills the proxy (`c2Proxy::default()`), clamps the vertex index (`ix()`) and
clamps the cache loop to 3 — so it cannot fault where the C does.

Every other row (**E01–E38, E40–E58**) is fully defined and is asserted
**bit-for-bit** between the two libraries.

## Rows whose trigger turned out to be unreachable

Two rows describe branches that exist in the C source but that no input can
reach. Both are recorded rather than quietly dropped, and each is replaced by
the strongest assertion that *is* checkable:

| row | why it is unreachable | asserted instead |
|-----|----------------------|------------------|
| **E29** (`while (iter < 20)` falling through) | a `c2Proxy` holds at most 4 vertices ⇒ at most 16 distinct `(iA,iB)` support pairs ⇒ the `dup` guard (E33) always fires first. Highest `iterations` seen: **5**, over 40 000 classified plus 400 000 additional randomised configurations (huge / `inf` / `NaN` geometry, synthetic caches, arbitrary transforms) | both libraries always report the **same** `iterations`; that value equals an open-coded model of the C loop, evaluated with the C library's own exported primitives; and it never leaves `[0, 20]` |
| **E41 / E42** (`c2GJK` returning `NaN` ⇒ `if (dist)` true ⇒ `return 0`) | both predicates pass `use_radius = 1`, and for a `NaN` `dist` the guard `if (dist > rA + rB && dist > FLT_EPSILON)` is false, so the `else` branch assigns `dist = 0`. `c2GJK` therefore returns `+0.0` or a positive value, never `NaN` | that invariant itself (no `NaN` return from either library over 20 000 `NaN`-rich inputs), plus the full truthiness mapping for the two reachable classes; the two spellings (`if (dist)` and `if r != 0.0`) also agree for `NaN` and for `-0.0` |

## The one tolerated difference: NaN payload bits

`Diff::f32` compares `f32::to_bits()`, so `+0.0 != -0.0` and every ordinary
value must match exactly. The single exception is the payload of a result where
**both** libraries produced a NaN.

`addss`/`mulss` are commutative, so the compiler chooses which operand lands in
the destination register — and x86 returns the *destination* operand's NaN when
both operands are NaN. GCC and LLVM choose differently for identical source:

```text
C   c2Mulvs:  movss a.x,%xmm0 ; mulss b,%xmm0     -> dst = a.x  (LHS)
RS  c2Mulvs:  movaps b,%xmm0  ; mulss a.x,%xmm0   -> dst = b    (RHS)
C   c2Dot   : mulss %xmm2,%xmm0   (dst = b.y)     -> dst = RHS
RS  c2Dot   : mulss -0xc(%rsp),%xmm1 (dst = a.y)  -> dst = LHS
```

IEEE-754 §6.2.3 leaves the payload unspecified when an operand is NaN and states
that the sign of a NaN is not interpreted; C adds no constraint. Matching GCC
would mean replicating GCC's register allocation, which no Rust source spelling
can express and which flips again at a different `-O` level.

So the policy is: **NaN-ness is compared strictly** (a NaN on one side and a
number on the other is a hard failure), **every non-NaN result is compared
bit-for-bit**, and only the payload bits of a mutually-NaN result are tolerated.
Each occurrence is counted and printed per row, e.g.
`[B06 ...] OK (209952 bit-exact checks, 4496 tolerated NaN-payload-only diffs)`.
No observable behaviour depends on those bits: nothing in the library inspects a
NaN's sign or payload, and every comparison involving a NaN is false either way.

## Row → test mapping (mechanically extracted)

Generated by scanning `tests/phase_c_errors.rs` for each row id, so it cannot
drift from the tests. All 60 rows are covered.

| row | differential test(s) in `tests/phase_c_errors.rs` |
|-----|--------------------------------------------------|
| E01 | `E01_makeproxy_out_of_range_type_writes_nothing` |
| E02 | `E01_makeproxy_out_of_range_type_writes_nothing`, `E02_collided_circle_with_bad_typeB` |
| E03 | `E03_collided_aabb_with_bad_typeB` |
| E04 | `E04_collided_capsule_with_bad_typeB` |
| E05 | `E01_makeproxy_out_of_range_type_writes_nothing`, `E05_collided_bad_typeA` |
| E06 | `E05_collided_bad_typeA`, `E06_simplex_metric_bad_count_returns_zero` |
| E07 | `E07_c2D_bad_count_returns_zero_vector` |
| E08 | `E08_c2L_bad_count_returns_zero_vector` |
| E09 | `E05_collided_bad_typeA`, `E09_c2Witness_bad_count_returns_zero_vectors` |
| E10 | `E09_c2Witness_bad_count_returns_zero_vectors`, `E10_E11_witness_zero_div` |
| E11 | `E09_c2Witness_bad_count_returns_zero_vectors`, `E10_E11_witness_zero_div` |
| E12 | `E10_E11_witness_zero_div`, `E12_c2Div_by_zero` |
| E13 | `E10_E11_witness_zero_div`, `E13_c2Norm_zero_vector` |
| E14 | `E10_E11_witness_zero_div`, `E14_c2Len_nonfinite` |
| E15 | `E14_c2Len_nonfinite`, `E15_E16_support_zero_and_one` |
| E16 | `E14_c2Len_nonfinite`, `E15_E16_support_zero_and_one` |
| E17 | `E14_c2Len_nonfinite`, `E17_support_ties_and_nan_pick_first` |
| E18 | `E17_support_ties_and_nan_pick_first`, `E18_E24_gjk_null_pointer_guards` |
| E19 | `E18_E24_gjk_null_pointer_guards` |
| E20 | `E18_E24_gjk_null_pointer_guards` |
| E21 | `E18_E24_gjk_null_pointer_guards` |
| E22 | `E18_E24_gjk_null_pointer_guards` |
| E23 | `E18_E24_gjk_null_pointer_guards` |
| E24 | `E17_support_ties_and_nan_pick_first`, `E18_E24_gjk_null_pointer_guards` |
| E25 | `E18_E24_gjk_null_pointer_guards`, `E25_gjk_cache_count_zero_is_cold` |
| E26 | `E18_E24_gjk_null_pointer_guards`, `E26_gjk_warm_cache_is_accepted` |
| E27 | `E18_E24_gjk_null_pointer_guards`, `E27_gjk_cache_rejected_when_metric_below_minus_1e8` |
| E28 | `E18_E24_gjk_null_pointer_guards`, `E28_gjk_negative_cache_count` |
| E29 | `E28_gjk_negative_cache_count`, `E29_E33_gjk_all_five_loop_guards` |
| E30 | `E28_gjk_negative_cache_count`, `E29_E33_gjk_all_five_loop_guards` |
| E31 | `E28_gjk_negative_cache_count` |
| E32 | `E28_gjk_negative_cache_count` |
| E33 | `E28_gjk_negative_cache_count`, `E29_E33_gjk_all_five_loop_guards` |
| E34 | `E29_E33_gjk_all_five_loop_guards`, `E34_E35_gjk_radius_collapse_to_midpoint` |
| E35 | `E34_E35_gjk_radius_collapse_to_midpoint` |
| E36 | `E36_gjk_radius_shrink_makes_points_coincide` |
| E37 | `E37_gjk_use_radius_zero_ignores_radii` |
| E38 | `E29_E33_gjk_all_five_loop_guards`, `E38_gjk_use_radius_arbitrary_nonzero` |
| E39 | `E38_gjk_use_radius_arbitrary_nonzero`, `E39_out_of_range_type_observable_behaviour` |
| E40 | `E40_E42_predicate_float_truthiness`, `E59_E60_gjk_out_of_contract_cache_does_not_crash` |
| E41 | `E40_E42_predicate_float_truthiness` |
| E42 | `E40_E42_predicate_float_truthiness`, `E59_E60_gjk_out_of_contract_cache_does_not_crash` |
| E43 | `E40_E42_predicate_float_truthiness`, `E43_E44_circle_to_circle_boundaries` |
| E44 | `E43_E44_circle_to_circle_boundaries` |
| E45 | `E45_circle_to_aabb_boundaries` |
| E46 | `E46_E47_E48_E49_circle_to_capsule_branches` |
| E47 | `E46_E47_E48_E49_circle_to_capsule_branches` |
| E48 | `E46_E47_E48_E49_circle_to_capsule_branches` |
| E49 | `E46_E47_E48_E49_circle_to_capsule_branches` |
| E50 | `E50_E51_aabb_to_aabb_boundaries` |
| E51 | `E40_E42_predicate_float_truthiness`, `E50_E51_aabb_to_aabb_boundaries` |
| E52 | `E50_E51_aabb_to_aabb_boundaries`, `E52_capsule_with_nan_arguments` |
| E53 | `E50_E51_aabb_to_aabb_boundaries`, `E53_capsule_extreme_arguments` |
| E54 | `E53_capsule_extreme_arguments`, `E54_bbverts_writes_exactly_four` |
| E55 | `E54_bbverts_writes_exactly_four`, `E55_E56_c22_both_collapse_branches` |
| E56 | `E55_E56_c22_both_collapse_branches` |
| E57 | `E57_E58_c23_all_seven_branches` |
| E58 | `E54_bbverts_writes_exactly_four`, `E57_E58_c23_all_seven_branches` |
| E59 | `E38_gjk_use_radius_arbitrary_nonzero`, `E59_E60_gjk_out_of_contract_cache_does_not_crash` |
| E60 | `E38_gjk_use_radius_arbitrary_nonzero`, `E59_E60_gjk_out_of_contract_cache_does_not_crash` |

## How to run

```sh
cargo test --offline                       # all 113 differential tests
cargo test --offline --test phase_c_errors  # just this table
./verify_all.sh                            # every feature combo x debug/release
./mutation_check.sh                        # prove the suite catches real bugs
```
