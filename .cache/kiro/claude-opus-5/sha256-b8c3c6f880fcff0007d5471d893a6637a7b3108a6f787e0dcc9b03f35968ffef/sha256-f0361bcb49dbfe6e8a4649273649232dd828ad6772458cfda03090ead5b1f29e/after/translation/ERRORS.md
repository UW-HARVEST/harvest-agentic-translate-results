# ERRORS.md — Phase A error / rejection surface table

The library has **no error codes, no `errno`, no `assert`, no `RETURN_ERROR`
macro and no `return -1` / `return NULL`**. Verified mechanically:

```sh
grep -nE 'assert|RETURN_ERROR|return *-1|return *NULL|errno|abort|exit\(' c_src/src/lib.c
# -> only  164: return 0;   (the `default:`/`case 1:` arm of c2GJKSimplexMetric)
```

Rejection is therefore expressed as **silent fallback / sentinel values**:
un-handled `switch` arms that fall to `default`, null-pointer guards, degenerate
break conditions, and saturating/collapsing numeric results. Each row below is
one distinct rejection branch that actually exists in `c_src/src/lib.c`.

Sentinel constants the C spells out:
`FLT_MAX = 3.40282346638528859811704183484516925e+38F`,
`FLT_EPSILON = 1.19209289550781250000000000000000000e-7F`,
`-1.0e8f` (cache metric floor), `2.0f` (cache metric ratio), `0.5f` (midpoint
collapse), iteration cap `20`, `c2Proxy::verts[8]`, `c2GJKCache::iA/iB[3]`,
`c2Simplex` = 4 `c2sv` slots.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `c2MakeProxy` | `type` not in {0,1,2} (e.g. 3, 99, `0xFFFFFFFF`, i.e. out-of-range C enum value) | `switch` has **no `default:`** → the `c2Proxy` is left **completely unwritten** (all of `radius`, `count`, `verts` keep the caller's prior bytes) | [x] |
| 2 | `c2GJKSimplexMetric` | `s->count` == 1 | `case 1:` → `0.0f` | [x] |
| 3 | `c2GJKSimplexMetric` | `s->count` == 0, 4, or negative | `default:` falls through to `case 1:` → `0.0f` | [x] |
| 4 | `c2D` | `s->count` == 3 | `case 3:`/`default:` → `c2V(0,0)` | [x] |
| 5 | `c2D` | `s->count` not in {1,2,3} (0, 4, negative) | `default:` → `c2V(0,0)` | [x] |
| 6 | `c2D` | `s->count` == 2 and `c2Det2(ab, -a.p) <= 0` (incl. NaN, which fails `> 0`) | takes `c2CCW90(ab)` instead of `c2Skew(ab)` | [x] |
| 7 | `c2L` | `s->count` not in {1,2} (0, 3, 4, negative) | `default:` → `c2V(0,0)` | [x] |
| 8 | `c2L` | `s->div == 0` with `count` 2 | `den = 1/0 = +inf` → components `inf`/`NaN`; no rejection, propagates | [x] |
| 9 | `c2Witness` | `s->count` not in {1,2,3} (0, 4, negative) | `default:` → `*a = *b = c2V(0,0)` | [x] |
| 10 | `c2Witness` | `s->div == 0` | `den = +inf`, results `inf`/`NaN` (count 1 unaffected) | [x] |
| 11 | `c2Support` | `count <= 0` | still dereferences `verts[0]` (unguarded), loop never runs → returns `0` | [x] |
| 12 | `c2Support` | all dots equal / NaN (`dot > dmax` never true) | returns `0` (first index wins, strict `>`) | [x] |
| 13 | `c2Div` | `b == 0` | `1.0f/0 = ±inf` → `±inf`/`NaN` components, no guard | [x] |
| 14 | `c2Norm` | zero-length vector `(0,0)` | `c2Len == 0` → `c2Div(a, 0)` → `NaN` components (`0*inf`) | [x] |
| 15 | `c2Norm` | non-finite input (NaN / inf component) | `NaN` propagation, no guard | [x] |
| 16 | `c2Maxv` / `c2Minv` | either operand NaN | ternary `a>b?a:b` / `a<b?a:b` is false for NaN → **selects `b`** | [x] |
| 17 | `c2Clampv` | `lo > hi` (inverted range — not validated) | `c2Maxv(lo, c2Minv(a,hi))` → returns `lo` | [x] |
| 18 | `c2GJK` | `ax_ptr == NULL` | substitutes `c2xIdentity()` instead of rejecting | [x] |
| 19 | `c2GJK` | `bx_ptr == NULL` | substitutes `c2xIdentity()` | [x] |
| 20 | `c2GJK` | `cache == NULL` | skips both the cache-read and the cache-write blocks entirely | [x] |
| 21 | `c2GJK` | `cache->count == 0` | `cache_was_good = !!0 = 0` → cache rejected, fresh 1-point simplex | [x] |
| 22 | `c2GJK` | cache present, `count != 0`, but `min_metric < max_metric*2.0f && metric < -1.0e8f` | cache **rejected** (`cache_was_read` stays 0) → fresh simplex | [x] |
| 23 | `c2GJK` | cache present, `count != 0`, metric test **not** satisfied (the common case, incl. NaN metrics) | cache **accepted**, `cache_was_read = 1`, `s.count`/`s.div` taken from cache | [x] |
| 24 | `c2GJK` | `cache->count < 0` | `!!count` is true → read loop body never runs → `s.count` negative → every downstream `switch` takes `default` → `dist = 0`, `a = b = (0,0)`, `iter = 0` | [x] |
| 25 | `c2GJK` | `outA == NULL` | result silently not stored | [x] |
| 26 | `c2GJK` | `outB == NULL` | result silently not stored | [x] |
| 27 | `c2GJK` | `iterations == NULL` | iteration count silently not stored | [x] |
| 28 | `c2GJK` | simplex reaches `count == 3` | `hit = 1`, loop breaks, `a = b`, `dist = 0` (radius branch skipped) | [x] |
| 29 | `c2GJK` | `d1 > d0` (no progress toward origin) | `break` out of the loop, keep the current simplex | [x] |
| 30 | `c2GJK` | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (degenerate search direction) | `break` | [x] |
| 31 | `c2GJK` | new support point duplicates a saved `(iA,iB)` pair | `break` before `++s.count` (the appended vertex stays uncounted) | [x] |
| 32 | `c2GJK` | 20 iterations elapse without termination | `while (iter < 20)` exits, whatever simplex is current is used | [x] |
| 33 | `c2GJK` | `use_radius != 0` and `!(dist > rA+rB && dist > FLT_EPSILON)` (touching/overlapping, or radii swallow the distance) | collapse: `a = b = 0.5f*(a+b)`, `dist = 0` | [x] |
| 34 | `c2GJK` | `use_radius != 0`, shrink applied, and `a.x==b.x && a.y==b.y` afterwards | `dist = 0` (post-shrink degeneracy) | [x] |
| 35 | `c2GJK` | `use_radius == 0` | radius shrink skipped entirely — raw core distance returned even for `r > 0` shapes | [x] |
| 36 | `c2GJK` | `typeA` and/or `typeB` out of enum range | via row 1 the `c2Proxy` is never written → C reads **uninitialised stack** (`pA.count`, `pA.verts[0]`). Documented UB; asserted only for the *direct* `c2MakeProxy` call (row 1), not through `c2GJK` | [x] (documented, not asserted) |
| 37 | `c2GJK` | `cache->count > 3` | reads `cache->iA[i]`/`iB[i]` past `[3]` and writes `saveA[i]` past `[3]` and `verts[i]` past the 4 simplex slots → out-of-bounds. Documented UB; not asserted | [x] (documented, not asserted) |
| 38 | `c2GJK` | non-finite shape coordinates (NaN / ±inf verts, NaN radius) | no validation anywhere; NaN propagates and every `>`/`<=` comparison follows IEEE, changing which branch is taken | [x] |
| 39 | `c22` | `v <= 0` (incl. NaN → false, so NaN takes the `else` chain) | collapse to `count = 1`, `a.u = 1`, `div = 1` | [x] |
| 40 | `c22` | `u <= 0` (and `v > 0`) | `a = b`, `count = 1`, `a.u = 1`, `div = 1` | [x] |
| 41 | `c23` | any of the 6 vertex/edge-region rejections (`vAB<=0&&uCA<=0`, `uAB<=0&&vBC<=0`, `uBC<=0&&vCA<=0`, `wABC<=0`, `uABC<=0`, `vABC<=0`) | reduces `count` to 1 or 2 with the corresponding vertex permutation | [x] |
| 42 | `c23` | degenerate triangle (`area == 0`, collinear verts) | all three `*ABC` become `0` → `<= 0` tests hit → reduces to an edge/vertex, `div` may be `0` | [x] |
| 43 | `c2BBVerts` | `bb->min > bb->max` (inverted AABB — never validated) | writes the 4 corners of the inverted box unchanged | [x] |
| 44 | `gjk_cache` | `a9` / `b9` pointers (including NULL) | **never dereferenced** — the parameters are unused, the function has no observable output at all | [x] |
| 45 | `gjk_cache` | `reverse` any non-zero `char` value (1, -1, 0x7f) | takes the capsule-vs-AABB order; `reverse == 0` takes AABB-vs-capsule. Both discard the result | [x] |
| 46 | `c2GJK` | `cache->iA[i]` / `iB[i]` >= the proxy's vertex count (e.g. index 3 on a 1-vertex `CIRCLE` proxy) | `pA.verts[iA]` is within the `verts[8]` array but was never written by `c2MakeProxy` → the C reads an **indeterminate value** (UB). Documented UB; not asserted — see "Documented UB" below | [x] (documented, not asserted) |

## Test coverage map

Every row above is covered by a differential test that loads both `.so`s:

| ERRORS.md rows | test |
|---|---|
| 1 (+ out-of-range enum across FFI) | `phase_c_errors::row01_make_proxy_out_of_range_enum` |
| 2, 3 | `phase_c_errors::row02_row03_simplex_metric_bad_count` |
| 4, 5 | `phase_c_errors::row04_row05_c2D_bad_count` |
| 6 | `phase_c_errors::row06_c2D_det_not_positive` |
| 7, 8 | `phase_c_errors::row07_row08_c2L` |
| 9, 10 | `phase_c_errors::row09_row10_witness` |
| 11, 12 | `phase_c_errors::row11_row12_support` |
| 13, 14, 15 | `phase_c_errors::row13_row14_row15_div_norm` |
| 16, 17 | `phase_c_errors::row16_row17_minmax_nan_and_inverted_clamp` |
| 18–20, 25–27, 35 | `phase_c_errors::row18_to_row27_and_row35_gjk_null_and_flags` |
| 21, 23, 24 | `phase_c_errors::row21_to_row24_cache_accept_reject` |
| 22 (the reject arm, driven across the `-1.0e8f` floor and the `*2.0f` ratio) | `phase_c_errors::row22_cache_reject_metric_floor_and_ratio` |
| 28–34 | `phase_c_errors::row28_to_row34_gjk_terminal_branches` |
| 36, 37 (defined halves) | `phase_c_errors::row36_row37_documented_ub_boundaries_are_covered_elsewhere` |
| 38 | `phase_c_errors::row38_nonfinite_shape_data` |
| 39, 40 | `phase_c_errors::row39_row40_c22_arms` |
| 41, 42 | `phase_c_errors::row41_row42_c23_arms` |
| 43 | `phase_c_errors::row43_bbverts_inverted` |
| 44, 45 | `phase_c_errors::row44_row45_gjk_cache_params` |

## Documented UB (rows 36, 37, 46) — why these are not asserted

Three inputs make the **C** read memory it never wrote, so the C has no defined
result to match:

* **Row 36** — `typeA`/`typeB` outside `{0,1,2}`: `c2MakeProxy`'s `switch` has no
  `default:`, so inside `c2GJK` the local `c2Proxy pA;` stays uninitialised and
  `pA.count` / `pA.verts[0]` are read as stack garbage. The *defined* half of the
  same behaviour — a direct `c2MakeProxy` call, where the caller owns the proxy —
  **is** asserted (row 1), including `0xFFFFFFFF` and a NULL `shape`.
* **Row 37** — `cache->count > 3`: reads `cache->iA[i]` past `iA[3]`, writes
  `saveA[i]` past `saveA[3]`, and writes `verts[i]` past the 4 `c2sv` slots of
  `c2Simplex`. The result depends on the compiler's stack frame layout.
* **Row 46** — a cache index at or beyond the proxy's vertex count.

The Rust zero-initialises `c2Proxy` and `c2Simplex` (`::default()`), so on these
paths the Rust is **deterministic where the C is not**. This is a deliberate,
documented deviation confined to UB-only inputs: no choice of Rust code can
reproduce GCC's stack garbage, and zero-init is the memory-safe option. All
*defined* neighbours of these boundaries (cache `count` 0, 1, 2, 3 and negative;
every in-range `iA`/`iB` combination) are asserted.

## Mutation evidence

`scripts/mutation_battery.sh` injects 55 known-wrong changes into
`src/lib.rs` — every numeric constant, every comparison operator, every
`switch`-arm body and vertex shuffle — rebuilds the cdylib and re-runs the whole
suite. Result: **46 killed, 9 survivors, and every survivor is provably an
equivalent mutant**:

| surviving mutant | why it cannot be observed |
|---|---|
| `while (iter < 20)` → `< 19`, → `< 8` | The maximum reachable `iter` is **7**, measured over 60M randomized calls. Proxies hold at most 4 vertices and the duplicate-support test ends the loop first, so the cap is dead. Lowering it to `< 7` (the reachable max) **is** killed, which proves the loop-cap mechanism itself is under test. |
| `c2Maxv` / `c2Minv` `a > b` → `b < a` | Identical IEEE-754 predicates (both false for NaN). |
| `c2Dot` term order `a.x*b.x + a.y*b.y` → `a.y*b.y + a.x*b.x` | IEEE addition is commutative. |
| `c2GJK` cache seed `v->u = 0` → `= 1` | Dead store: `u` is never read for `count == 1`, and for `count` 2/3 `c22`/`c23` run at the top of the loop and overwrite it before `c2L`/`c2Witness` read it. Confirmed by a 9M-call oracle search finding 0 divergences. |
| `gjk_cache` `reverse` branches swapped; warm-cache call removed | `gjk_cache` returns `void`, never dereferences `a9`/`b9`, and discards every `c2GJK` result — it has **zero** observable output, so no external caller can distinguish any of its internals. |
| `c2GJKSimplexMetric` `count == 2` operand order | `c2Len` is `sqrt(dx*dx + dy*dy)`, invariant under negating the difference. |

## Compiler-stability evidence

The suite was also run against the C compiled at `-O0`, `-O1`, `-O2` and `-O3`
(out-of-tree, `c_src/` untouched): **0 failures at every level**, i.e. the
translation is bit-faithful across GCC's optimisation pipeline. `-Ofast` does
diverge, by 1–2 ULP in the witness points only — that flag enables
`-ffast-math`, which abandons IEEE semantics and reassociates the float
expressions; it is not this project's build (`CMakeLists.txt` sets no `-O` flag).

## NaN payload caveat

The one relaxation in the comparison helper (`tests/common/mod.rs::feq`) is that
two NaNs compare equal regardless of sign bit / payload. Measured example:
`c2Dot((FLT_MAX, -inf), (-NaN, +NaN))` gives `0x7fc00000` from GCC and
`0xffc00000` from rustc. Which operand's NaN survives `mulss` follows the x86
instruction's operand order, which the C expression `a.x*b.x + a.y*b.y` does not
fix. Everything else is strict: `-0.0 != 0.0`, `inf` must match exactly, and
NaN-vs-non-NaN is a failure.
