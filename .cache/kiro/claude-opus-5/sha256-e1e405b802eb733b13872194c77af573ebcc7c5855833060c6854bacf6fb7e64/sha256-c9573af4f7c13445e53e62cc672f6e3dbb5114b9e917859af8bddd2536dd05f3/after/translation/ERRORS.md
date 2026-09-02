# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. The library has **no** error-return
macro, **no** `assert`, **no** `errno`, and **no** negative/`NULL` error sentinel:
it never allocates and never validates. Its entire "rejection" surface consists of

1. explicit `NULL` pointer checks (`if (!ax_ptr)`, `if (!bx_ptr)`, `if (outA)`,
   `if (outB)`, `if (iterations)`, `if (cache)`),
2. `switch` statements whose `default:` label silently returns a fallback
   (`return 0` / `c2V(0,0)` / no-op) for out-of-range `C2_TYPE` and out-of-range
   `c2Simplex.count`,
3. `<= 0` / `< 0` / `>` degeneracy guards inside the simplex solvers and the
   GJK termination tests,
4. hard limits and magic constants: `iter < 20`, `FLT_MAX`, `FLT_EPSILON`,
   `-1.0e8f`, `c2Proxy.verts[8]`, `c2GJKCache.iA[3]`/`iB[3]`.

Every row below is one distinct branch in the C. Line numbers refer to
`c_src/src/lib.c`. Column `test` names the differential test that pins the row.

| #  | function | trigger (exact invalid input / condition) | expected C result |
|----|----------|--------------------------------------------|-------------------|
|  1 | `c2MakeProxy` (L108) | `type` is not `0`/`1`/`2` (e.g. `3`, `-1`, `255`, `INT_MIN`) — the `switch` has **no** `default:` | `*p` left **completely untouched**; no field written, no crash |
|  2 | `c2GJKSimplexMetric` (L159 `default:` falls into `case 1:`) | `s->count` ∉ {2,3} (0, 1, 4, -1, huge) | returns `0.0f` |
|  3 | `c2GJKSimplexMetric` (L164) | `s->count == 2` | `c2Len(b.p - a.p)` (never negative, may be `NaN`/`inf`) |
|  4 | `c2GJKSimplexMetric` (L166) | `s->count == 3` | `c2Det2(b.p-a.p, c.p-a.p)` — **signed**, may be negative |
|  5 | `c22` (L190) | `v = dot(a, a-b) <= 0` | collapses to `count=1`, `a.u=1`, `div=1`; `b`/`c`/`d` untouched |
|  6 | `c22` (L194) | `v > 0 && u = dot(b, b-a) <= 0` | `a = b`, `count=1`, `a.u=1`, `div=1` |
|  7 | `c22` (L200) | `u > 0 && v > 0` | `count=2`, `div = u+v` (may be `0`/`inf`/`NaN` → later `1/div`) |
|  8 | `c22` (L186-189) | `a.p == b.p` (duplicate vertex ⇒ `u == v == 0`) | takes the `v <= 0` branch ⇒ `count=1` |
|  9 | `c23` (L221) | `vAB <= 0 && uCA <= 0` (vertex-A region) | `count=1`, `a.u=1`, `div=1` |
| 10 | `c23` (L225) | `uAB <= 0 && vBC <= 0` (vertex-B region) | `a = b`, `count=1` |
| 11 | `c23` (L230) | `uBC <= 0 && vCA <= 0` (vertex-C region) | `a = c`, `count=1` |
| 12 | `c23` (L235) | `uAB>0 && vAB>0 && wABC<=0` (edge AB) | `count=2`, `div = uAB+vAB` |
| 13 | `c23` (L240) | `uBC>0 && vBC>0 && uABC<=0` (edge BC) | `a=b; b=c;` `count=2`, `div = uBC+vBC` |
| 14 | `c23` (L247) | `uCA>0 && vCA>0 && vABC<=0` (edge CA) | `b=a; a=c;` `count=2`, `div = uCA+vCA` |
| 15 | `c23` (L254) | none of the above (interior) | `count=3`, `div = uABC+vABC+wABC`. Algebraically `div == area*area`, because `det(b,c)+det(c,a)+det(a,b) == area` |
| 16 | `c23` (L219) | collinear/degenerate triangle ⇒ `area == 0` ⇒ `uABC=vABC=wABC=0` | **collapses to a vertex or edge branch, never the interior branch.** One of rows #9-#14 always fires first, so `count` becomes 1 or 2 and `div` becomes `1.0` or `u+v` — it does *not* reach the interior branch with `div == 0`. Verified: exhaustive over integer collinear triples, plus 20M randomized f32 triples at scales `1e-30`…`1e20`, produced **zero** interior-branch cases with `div == 0`. Consequently `1/div == inf` is not reachable from `c23` |
| 17 | `c2D` (L287) | `s->count == 1` | `-a.p` |
| 18 | `c2D` (L289) | `s->count == 2`, `c2Det2(ab, -a.p) > 0` | `c2Skew(ab)` |
| 19 | `c2D` (L292) | `s->count == 2`, `c2Det2(ab, -a.p) <= 0` (incl. `== 0`, `NaN`) | `c2CCW90(ab)` |
| 20 | `c2D` (L296 `case 3: default:`) | `s->count` ∉ {1,2} (0, 3, 4, -1, huge) | `c2V(0,0)` |
| 21 | `c2L` (L353 `default:`) | `s->count` ∉ {1,2} (0, 3, 4, -1) | `c2V(0,0)` |
| 22 | `c2L` (L346) | `s->div == 0` | `den = inf` ⇒ result `inf`/`NaN` (never guarded) |
| 23 | `c2Witness` (L331 `default:`) | `s->count` ∉ {1,2,3} (0, 4, -1, huge) | `*a = *b = c2V(0,0)` |
| 24 | `c2Witness` (L304) | `s->div == 0` | `den = inf` ⇒ `inf`/`NaN` written to `*a`,`*b` (no guard) |
| 25 | `c2Support` (L301) | `count <= 0` (`0`, `-1`, `INT_MIN`) | **still dereferences `verts[0]`** and returns `0` — no bounds check |
| 26 | `c2Support` (L305) | all dots equal / `NaN` (`dot > dmax` never true) | returns `0` (first index wins ties) |
| 27 | `c2GJK` (L367) | `ax_ptr == NULL` | substitutes `c2xIdentity()` — **not** an error |
| 28 | `c2GJK` (L371) | `bx_ptr == NULL` | substitutes `c2xIdentity()` |
| 29 | `c2GJK` (L510) | `outA == NULL` | result A silently discarded, no write, no crash |
| 30 | `c2GJK` (L512) | `outB == NULL` | result B silently discarded |
| 31 | `c2GJK` (L514) | `iterations == NULL` | iteration count silently discarded |
| 32 | `c2GJK` (L380) | `cache == NULL` | cache read **and** write both skipped |
| 33 | `c2GJK` (L381) | `cache != NULL && cache->count == 0` | `cache_was_good == 0` ⇒ fresh 1-vertex simplex; cache still **written** on exit |
| 34 | `c2GJK` (L404) | `cache->count != 0` — the guard is `!(min_metric < max_metric*2 && metric < -1.0e8f)`. `c2GJKSimplexMetric` returns `0` for `count==1` and `>= 0` for `count==2`, so `metric < -1.0e8f` is essentially never true | `cache_was_read = 1` for **every** non-zero-count cache: the warm simplex is always accepted, the freshness test is dead code |
| 35 | `c2GJK` (L404) | `cache->count == 3` and the cached triangle has `det2 < -1.0e8f` **and** `min_metric < 2*max_metric` | `cache_was_read` stays `0` ⇒ cache discarded, fresh simplex. Only reachable path that rejects a cache |
| 36 | `c2GJK` (L424) | non-terminating configuration | loop is hard-capped at `iter < 20`; `*iterations <= 20` always |
| 37 | `c2GJK` (L441) | `d1 > d0` (no progress / regression) | `break` out of the loop, keep current simplex |
| 38 | `c2GJK` (L450) | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (search direction collapsed, incl. `count==3`-unreachable and `d == 0`) | `break` |
| 39 | `c2GJK` (L465) | new support pair `(iA,iB)` duplicates a saved pair | `break` **before** `++s.count`, so the freshly written `verts[s.count]` is left in the array but not counted |
| 40 | `c2GJK` (L437) | simplex reached `count == 3` after `c23` | `hit = 1`, `a = b`, `dist = 0` — radii **ignored** even when `use_radius` is set |
| 41 | `c2GJK` (L484) | `use_radius != 0` and `dist <= rA+rB` | `a = b = midpoint(a,b)`, `dist = 0` |
| 42 | `c2GJK` (L485) | `use_radius != 0` and `dist <= FLT_EPSILON` (touching / identical shapes) | same midpoint branch, `dist = 0` |
| 43 | `c2GJK` (L487-491) | `use_radius != 0`, `dist > rA+rB`, `dist > FLT_EPSILON` | `dist -= rA+rB`; `c2Norm(b-a)`; witnesses pushed onto the surfaces |
| 44 | `c2GJK` (L492) | after the shrink, `a.x==b.x && a.y==b.y` | `dist` forced to `0` (float cancellation guard) |
| 45 | `c2GJK` (L481) | `use_radius == 0` | raw core distance returned, radii never applied, witnesses left on the cores |
| 46 | `c2GJK` (L479) | `typeA`/`typeB` out of range ⇒ `c2MakeProxy` no-op ⇒ `pA`/`pB` are **uninitialised stack** in C | *indeterminate, and memory-unsafe.* `c2Support` iterates `i < pA.count` with the garbage `count`, so the C reads far past `verts[8]` and **segfaults** for many stack states (measured: ~1 run in 3 when the frame is pre-dirtied with `0x40000000`-ish patterns; also reproduces spontaneously under `--test-threads=4`). Not differentially testable — see below |
| 47 | `c2Norm`/`c2Div` (L338/L342) | `b == 0` ⇒ `1.0f/0.0f` | `+inf`, propagated as `inf`/`NaN`; no guard |
| 48 | `c2Norm` (L342) | `a == c2V(0,0)` ⇒ `c2Len == 0` | `c2V(NaN, NaN)` (`0 * inf`) |
| 49 | `c2AABBtoAABB` (L521) | inverted AABB (`min > max`) | evaluated with plain `<`; **no** normalisation, returns whatever the 4 half-space tests say |
| 50 | `c2AABBtoAABB` (L525) | `NaN` in any coordinate | all four `<` are false ⇒ `d0|d1|d2|d3 == 0` ⇒ returns **1** (reports overlap) |
| 51 | `c2AABBtoCapsule` (L528) | `c2GJK(...) != 0.0f` (incl. `NaN`, which is truthy in C) | returns `0` |
| 52 | `c2AABBtoCapsule` (L529) | `c2GJK(...) == 0.0f` (also `-0.0f`) | returns `1` |
| 53 | `c2CapsuletoCapsule` (L534/535) | same two branches as #51/#52 | `0` / `1` |
| 54 | `c2CircletoCircle` (L543) | negative radius | `r2 = (A.r+B.r)^2` is squared, so a sum of `-5` behaves like `+5`: sign is **lost** |
| 55 | `c2CircletoCircle` (L543) | `d2 == r2` (exact touch) | strict `<` ⇒ returns `0` |
| 56 | `c2CircletoAABB` (L550) | inverted AABB (`min > max`) | `c2Clampv` = `max(lo, min(a,hi))` with inverted bounds returns `lo`; no normalisation |
| 57 | `c2CircletoAABB` (L553) | `A.r == 0` or `d2 == r2` | strict `<` ⇒ `0` |
| 58 | `c2CircletoCapsule` (L559) | `da = dot(ap,n) < 0` | distance measured to endpoint `B.a` |
| 59 | `c2CircletoCapsule` (L563) | `da >= 0 && db = dot(p-B.b, n) < 0` | perpendicular distance; `da / c2Dot(n,n)` — **divides by `dot(n,n)`** |
| 60 | `c2CircletoCapsule` (L563) | degenerate capsule `B.a == B.b` ⇒ `n == 0` ⇒ `da == 0`, `db == 0` | `da >= 0`, `db >= 0` ⇒ **else** branch, distance to `B.b`; the `0/0` division is *not* reached |
| 61 | `c2CircletoCapsule` (L567) | `da >= 0 && db >= 0` | distance measured to endpoint `B.b` |
| 62 | `c2Collided` (L586) | `typeA == C2_TYPE_CIRCLE`, `typeB` ∉ {0,1,2} | `return 0` |
| 63 | `c2Collided` (L598) | `typeA == C2_TYPE_AABB`, `typeB` ∉ {0,1,2} | `return 0` |
| 64 | `c2Collided` (L610) | `typeA == C2_TYPE_CAPSULE`, `typeB` ∉ {0,1,2} | `return 0` |
| 65 | `c2Collided` (L614) | `typeA` ∉ {0,1,2} (any `typeB`, incl. invalid) | `return 0` — `B` is never dereferenced |
| 66 | `c2Collided` (L592) | `typeA=AABB, typeB=CIRCLE` | **arguments swapped**: `c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A)` |
| 67 | `c2Collided` (L604/606) | `typeA=CAPSULE, typeB=CIRCLE` / `typeB=AABB` | arguments swapped likewise |
| 68 | `aabb` (L620) | any input, incl. `NaN`/`inf` | 3-bit mask; bit0 circle, bit1 aabb, bit2 capsule; result ∈ `[0,7]` |
| 69 | `c2Maxv`/`c2Minv` (L60/L66) | `NaN` operand | ternary select, **not** `fmaxf`: `NaN > x` is false ⇒ returns the **second** operand for `c2Maxv`, and for `c2Minv` `NaN < x` false ⇒ second operand. Asymmetric and must be replicated exactly |
| 70 | `c2Clampv` (L71) | `lo > hi` | `c2Maxv(lo, c2Minv(a,hi))` ⇒ always `lo`; no swap, no assert |

## Constants and hard limits (grepped, no separate rows needed)

| constant | value | site |
|----------|-------|------|
| `FLT_MAX` (spelled out) | `3.40282346638528859811704183484516925e+38F` | `d0`, `d1` init (L420-421) |
| `FLT_EPSILON` (spelled out) | `1.19209289550781250000000000000000000e-7F` | direction collapse (L450), `dist` guard (L485) |
| GJK iteration cap | `20` | `while (iter < 20)` (L424) |
| cache-reject threshold | `-1.0e8f` | L404 |
| `c2Proxy.verts` capacity | `8` | L100 — max proxy vertices is 4 (AABB), so 4 slots are always unwritten |
| `c2GJKCache.iA/iB` capacity | `3` | L41-42 |
| `saveA`/`saveB` capacity | `3` | L418 |
| max `c2Simplex` vertices | `4` (`a,b,c,d`) | L146 |

## Not differentially testable (undefined behaviour in the C)

These are real rejection-adjacent inputs, but the C's answer is *indeterminate*,
so "byte-identical" is not a meaningful assertion. They are excluded from the
Phase C suite by construction, and the reason is recorded here rather than being
silently skipped:

- Row #46 / `c2GJK` with `typeA` or `typeB` out of range: `c2MakeProxy` writes
  nothing, so C proceeds with an **uninitialised** `c2Proxy` on the stack
  (`pA.count`, `pA.verts[0]`, `pA.radius` are garbage). Rust zero-initialises.
  This is not just a value mismatch: `c2Support` loops `i < count` with the
  garbage `count`, so the C reads past `verts[8]` and **segfaults** for many
  stack states. Verified by dlopening the C `.so` from a driver that dirties the
  stack frame first — SIGSEGV in roughly 1 run in 3 — and it also reproduced
  spontaneously in the test harness under `--test-threads=4`. No implementation
  can match indeterminate stack contents, and none should match a crash, so
  `tests/phase_c_boundaries.rs::row46_invalid_gjk_type_is_undefined_behaviour`
  is `#[ignore]`d rather than asserting on UB.
  Note that `c2Collided` (rows #62-#65) *does* guard its enum with `default:
  return 0`, and that guard **is** fully tested; only the raw `c2GJK` entry
  point is exposed to this.
- `c2GJK` with `cache->count > 3`: C reads `cache->iA[3]` (past the array, into
  the `iB`/`div` fields) and then writes `saveA[3]` past a 3-element stack array.
  Rust panics on the bounds check instead. UB in C.
- `c2GJK` with a cache whose `iA[i]`/`iB[i]` index at or beyond the proxy's
  `count`: in C, `pA.verts[iA]` for `iA` in `[count, 8)` reads uninitialised
  (but in-bounds) struct bytes; for `iA >= 8` or `iA < 0` it reads off the end
  of the struct entirely. Rust reads zeros / panics. UB in C.
- Null `c2Simplex*` / `c2Proxy*` / `c2v*` out-pointers to `c22`, `c23`, `c2D`,
  `c2L`, `c2Witness`, `c2Support`, `c2MakeProxy`, `c2BBVerts`: the C has no null
  check on these and segfaults. Both sides crash; nothing to compare.
  (Only `c2GJK` checks its pointers — rows #27-#32 — and those *are* tested.)

## Row → test mapping (Phase C)

Every row above has a differential test that constructs the exact trigger, calls
both `.so`s, and asserts they produce the **same specific** fallback value — not
merely that both failed. Rows are checked off only because their named test
passes.

| rows | test | file |
|------|------|------|
| 1 | `err01_makeproxy_invalid_type_is_a_noop` | `tests/phase_c_lowlevel.rs` |
| 2, 3, 4 | `err02_to_err04_simplex_metric_default` | `tests/phase_c_lowlevel.rs` |
| 5, 6, 7, 8 | `err05_to_err08_c22_guards` | `tests/phase_c_lowlevel.rs` |
| 9-16 | `err09_to_err16_c23_guards` | `tests/phase_c_lowlevel.rs` |
| 17, 18, 19, 20 | `err17_to_err20_c2D_guards` | `tests/phase_c_lowlevel.rs` |
| 21, 22 | `err21_err22_c2L_guards` | `tests/phase_c_lowlevel.rs` |
| 23, 24 | `err23_err24_c2Witness_guards` | `tests/phase_c_lowlevel.rs` |
| 25, 26 | `err25_err26_c2Support_guards` | `tests/phase_c_lowlevel.rs` |
| 27, 28 | `err27_err28_null_transforms_equal_identity` | `tests/phase_c_gjk.rs` |
| 29, 30, 31 | `err29_to_err31_null_out_pointers` | `tests/phase_c_gjk.rs` |
| 32 | `err32_cache_null_skips_read_and_write` | `tests/phase_c_gjk.rs` |
| 33 | `err33_cache_count_zero_is_cold` | `tests/phase_c_gjk.rs` |
| 34, 35 | `err34_err35_cache_freshness_test` | `tests/phase_c_gjk.rs` |
| 36, 37, 38, 39 | `err36_to_err39_loop_termination` | `tests/phase_c_gjk.rs` |
| 40-45 | `err40_to_err45_radius_stage` | `tests/phase_c_gjk.rs` |
| 46 | *not testable* — `row46_invalid_gjk_type_is_undefined_behaviour` (`#[ignore]`) | `tests/phase_c_boundaries.rs` |
| 47, 48 | `err47_err48_norm_of_zero_through_gjk` | `tests/phase_c_gjk.rs` |
| 49, 50 | `err49_err50_aabbtoaabb_inverted_and_nan` | `tests/phase_c_wrappers.rs` |
| 51, 52, 53 | `err51_to_err53_gjk_backed_booleans` | `tests/phase_c_wrappers.rs` |
| 54, 55, 56, 57 | `err54_to_err57_circle_boundaries` | `tests/phase_c_wrappers.rs` |
| 58, 59, 60, 61 | `err58_to_err61_circletocapsule_regions` | `tests/phase_c_wrappers.rs` |
| 62, 63, 64, 65 | `err62_to_err65_collided_invalid_enums` | `tests/phase_c_wrappers.rs` |
| 66, 67 | `err66_err67_collided_argument_swaps` | `tests/phase_c_wrappers.rs` |
| 68 | `err68_aabb_result_range` | `tests/phase_c_wrappers.rs` |
| 69, 70 | `err69_err70_minmax_nan_and_inverted_clamp` | `tests/phase_c_wrappers.rs` |

Generic boundaries not tied to a single row — all-`NULL` pointer sets, zero and
oversized `c2Support` lengths, and one-step-past-range enum values on every
`C2_TYPE` parameter — are in `tests/phase_c_boundaries.rs`
(`bound_all_null_pointers`, `bound_support_lengths`,
`bound_one_past_range_enums`).

## Branch coverage actually observed

Printed by the tests (`cargo test --release -- --nocapture --test-threads=1`),
so the rows are not merely *written* but demonstrably *reached*:

```
c23 branch coverage (vertA,vertB,vertC,edgeAB,edgeBC,edgeCA,interior)
                                       = [147, 138, 147, 53, 62, 66, 12]
c2CircletoCapsule regions (da<0, db<0, else) = [280, 203, 224]
c2GJK radius stage: hit=3000 midpoint=15046 shrink=12954
c2GJK cache count histogram (0..4)     = [0, 7708, 2496, 596, 0]
c2GJK iteration histogram              = [24556, 14234, 5568, 642, 0, ...]
aabb() result-mask histogram (0..7)    = [7887, 644, 152, 192, 2600, 351, 90, 248]
count==3 warm caches exercised         = 1968
```

Note on the iteration cap (row 36): the observed maximum is 3. That is inherent,
not a coverage gap — the largest proxy has 4 vertices, so the simplex runs out of
distinct support pairs and hits the `dup` break (row 39) long before `iter`
reaches 20. The cap is asserted as an invariant (`0 <= *iterations <= 20` on both
sides) rather than driven to its limit, because no input can drive it there.

## Floating-point comparison policy

All comparisons are bit-exact (`f32::to_bits`), including `+0.0` vs `-0.0`,
`inf` sign, and denormals — with **one** measured exception: two `NaN`s are
treated as equal regardless of payload. The sign bit of a `NaN` propagated
through `mulss`/`addss` is a register-allocation artifact, not source semantics:
compiling the exact `c2Dot` expression `a.x*b.x + a.y*b.y` with two
opposite-signed `NaN` inputs yields `0x7fc00000` at `-O0` and `0xffc00000` at
`-O1`/`-O2`/`-O3`/`-Os` from identical C. IEEE-754 leaves this unspecified, so it
is below the resolution of the C's own behaviour. Rationale is recorded at
`tests/common/mod.rs::same_f32`.
