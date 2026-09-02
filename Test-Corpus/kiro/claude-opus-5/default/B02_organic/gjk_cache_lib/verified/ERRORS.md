# ERRORS.md — error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. Notes on what the grep found:

```
grep -n 'RETURN_ERROR\|assert\|errno\|return -1\|return NULL\|return 0;' c_src/src/lib.c
```

* There is **no** error enum, **no** `RETURN_ERROR`-style macro, **no** `assert`,
  and **no** `NULL`/`-1` sentinel return anywhere in the library.
* The library's entire rejection surface therefore consists of
  (a) explicit **null-pointer guards**, (b) explicit **range / degeneracy
  checks** (`<= 0`, `> 0`, `< FLT_EPSILON*FLT_EPSILON`, `> d0`, `iter < 20`),
  (c) `switch` statements whose **`default:` / missing-`default:`** arm is the
  rejection, and (d) **division-by-zero** sites that are not guarded at all.
* Min/max constants present in the source: `FLT_MAX`
  (`3.40282346638528859811704183484516925e+38F`), `FLT_EPSILON`
  (`1.19209289550781250000000000000000000e-7F`), the literal `-1.0e8f`
  staleness bound, the iteration cap `20`, the simplex cap `3`, and the
  `c2Proxy.verts` capacity `8`.

Each row below is one distinct rejection branch the C actually takes.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `c2GJK` | `ax_ptr == NULL` | `ax = c2xIdentity()` (`p=(0,0)`, `r=(1,0)`); no crash | [x] |
| 2 | `c2GJK` | `bx_ptr == NULL` | `bx = c2xIdentity()` | [x] |
| 3 | `c2GJK` | `outA == NULL` | result still returned, `outA` not written | [x] |
| 4 | `c2GJK` | `outB == NULL` | result still returned, `outB` not written | [x] |
| 5 | `c2GJK` | `iterations == NULL` | result still returned, `iterations` not written | [x] |
| 6 | `c2GJK` | `cache == NULL` | no cache read, no cache write-back | [x] |
| 7 | `c2GJK` | `cache != NULL && cache->count == 0` | `cache_was_good = 0` → simplex reset to `verts[0]`, `div=1`, `count=1` | [x] |
| 8 | `c2GJK` | `cache->count < 0` (non-zero ⇒ "good") | read loop body never runs; `s.count` stays negative ⇒ every `switch` takes `default`; `c2L`/`c2D` return `(0,0)`; `c2Witness` default ⇒ `a=b=(0,0)`; returns `0.0`; cache written back with negative `count` and no `iA`/`iB` writes | [x] |
| 9 | `c2GJK` | staleness test `!(min_metric < max_metric*2.0f && metric < -1.0e8f)` | `metric < -1.0e8f` is essentially never true, so the guard is inverted-true and `cache_was_read = 1` for *any* non-zero `cache->count` — the stale-cache reset is effectively dead code. Must be replicated, not "fixed". | [x] |
| 10 | `c2GJK` | `use_radius == 0` | radius shrink block skipped entirely; `dist` is the raw core distance and `a`,`b` are the raw witness points | [x] |
| 11 | `c2GJK` | `use_radius != 0` and `!(dist > rA+rB && dist > FLT_EPSILON)` (shapes overlap / touching / NaN `dist`) | `a = b = 0.5f*(a+b)`, `dist = 0` | [x] |
| 12 | `c2GJK` | `use_radius != 0`, shrink applied, and afterwards `a.x==b.x && a.y==b.y` | `dist` forced to `0` | [x] |
| 13 | `c2GJK` | simplex reaches `count == 3` (origin enclosed) | `hit = 1`, loop breaks, `a = b`, `dist = 0` (radius block skipped because `hit` wins) | [x] |
| 14 | `c2GJK` | `d1 > d0` (distance stopped decreasing) | `break` out of the iteration loop | [x] |
| 15 | `c2GJK` | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (degenerate search direction) | `break` | [x] |
| 16 | `c2GJK` | new support point duplicates a saved one (`iA==saveA[i] && iB==saveB[i]`) | `break` **before** `++s.count`, so the duplicate vertex is written into `verts[s.count]` but not counted | [x] |
| 17 | `c2GJK` | iteration cap: `while (iter < 20)` | loop exits after at most 20 increments; `*iterations <= 20` | [x] |
| 18 | `c2GJK` | `typeA` or `typeB` not in `{0,1,2}` (out-of-range enum across FFI) | `c2MakeProxy` hits no `case` and leaves the **uninitialised local** `c2Proxy` untouched ⇒ undefined behaviour (reads indeterminate stack). Not bit-reproducible by construction; see "UB rows" below. | [x] |
| 19 | `c2MakeProxy` | `type` not in `{0,1,2}` (no `default:` arm) | caller-supplied `*p` left **completely untouched** (byte-for-byte unchanged) | [x] |
| 20 | `c2MakeProxy` | `type == C2_TYPE_AABB` | `p->radius` forced to `0`, `p->count = 4`; `verts[4..8]` left untouched | [x] |
| 21 | `c2Support` | `count <= 0` (0 or negative) | `verts[0]` is dereferenced **before** the loop guard; loop body never runs; returns `0` | [x] |
| 22 | `c2Support` | all dot products equal, or `d = (0,0)` | strict `>` keeps the first index ⇒ returns `0` | [x] |
| 23 | `c2Support` | some `c2Dot` is NaN | `dot > dmax` is false for NaN ⇒ NaN candidates never win; if `verts[0]` gives NaN `dmax`, no later index can beat it ⇒ returns `0` | [x] |
| 24 | `c2Witness` | `s->count` not in `{1,2,3}` (0, negative, `>3`) | `default:` ⇒ `*a = *b = (0,0)` | [x] |
| 25 | `c2Witness` | `s->div == 0.0` | `den = 1.0f/0.0f = +inf`, unguarded ⇒ `inf`/`NaN` components propagate | [x] |
| 26 | `c2GJKSimplexMetric` | `count` not in `{2,3}` (0, 1, negative, `>3`) | `default:` falls through to `case 1:` ⇒ returns `0` | [x] |
| 27 | `c2D` | `count == 3` or any other value (`default:`) | returns `(0,0)` | [x] |
| 28 | `c2D` | `count == 2` and `c2Det2(ab, -a) <= 0` (incl. NaN, which fails `> 0`) | returns `c2CCW90(ab)` instead of `c2Skew(ab)` | [x] |
| 29 | `c2L` | `count` not in `{1,2}` (`default:`) | returns `(0,0)` | [x] |
| 30 | `c2L` | `s->div == 0.0` with `count == 2` | `den = +inf` ⇒ `inf`/`NaN` result, unguarded | [x] |
| 31 | `c2Div` | `b == 0.0` | `1.0f/0.0f = +inf`; `0*inf = NaN` per component ⇒ no rejection, `inf`/`NaN` returned | [x] |
| 32 | `c2Norm` | `a == (0,0)` (zero-length vector) | `c2Len = 0` ⇒ `c2Div(a, 0)` ⇒ `(NaN, NaN)` | [x] |
| 33 | `c2Len` | `c2Dot(a,a) < 0` impossible, but overflow to `+inf` | `sqrtf(+inf) = +inf`; `sqrtf(NaN) = NaN` | [x] |
| 34 | `c22` | `v <= 0` (origin outside `A` side) | collapse to vertex `a`: `a.u=1`, `div=1`, `count=1` | [x] |
| 35 | `c22` | `u <= 0` (and `v > 0`) | `s->a = s->b`, `a.u=1`, `div=1`, `count=1` | [x] |
| 36 | `c22` | `a.p == b.p` (duplicate vertices ⇒ `u = v = 0`) | first branch wins (`v <= 0`) ⇒ `count=1` | [x] |
| 37 | `c22` | any of `u`, `v` is NaN | `<= 0` is false for NaN ⇒ falls to the `else` arm ⇒ `count=2`, `div = NaN` | [x] |
| 38 | `c23` | `vAB <= 0 && uCA <= 0` | vertex region A: `count=1`, `div=1` | [x] |
| 39 | `c23` | `uAB <= 0 && vBC <= 0` | vertex region B: `s->a = s->b`, `count=1` | [x] |
| 40 | `c23` | `uBC <= 0 && vCA <= 0` | vertex region C: `s->a = s->c`, `count=1` | [x] |
| 41 | `c23` | `uAB > 0 && vAB > 0 && wABC <= 0` | edge AB: `count=2`, `div = uAB+vAB` | [x] |
| 42 | `c23` | `uBC > 0 && vBC > 0 && uABC <= 0` | edge BC: `a=b; b=c`, `count=2` | [x] |
| 43 | `c23` | `uCA > 0 && vCA > 0 && vABC <= 0` | edge CA: `b=a; a=c`, `count=2` | [x] |
| 44 | `c23` | degenerate/collinear triangle ⇒ `area == 0` ⇒ `uABC=vABC=wABC=0` | falls through to the interior `else` (all three `<= 0` sub-tests fail their `> 0` companions or the vertex tests catch it first); if it reaches the `else`, `div = 0` and `c2Witness` then divides by zero | [x] |
| 45 | `c23` | all NaN barycentrics | every `<= 0` and `> 0` test false ⇒ final `else` ⇒ `count=3`, `div = NaN` | [x] |
| 46 | `c2Maxv` | either component is NaN | ternary `a.x > b.x` false ⇒ selects `b`'s component | [x] |
| 47 | `c2Minv` | either component is NaN | ternary `a.x < b.x` false ⇒ selects `b`'s component | [x] |
| 48 | `c2Clampv` | `lo > hi` (inverted range) | no validation: `c2Maxv(lo, c2Minv(a, hi))` ⇒ returns `lo` | [x] |
| 49 | `c2BBVerts` | `bb->min > bb->max` (inverted AABB) | no validation; winding is emitted as-is | [x] |
| 50 | `gjk_cache` | `a9 == NULL && b9 == NULL` | both parameters are **never dereferenced or written** by the C body ⇒ no crash, no output | [x] |
| 51 | `gjk_cache` | `reverse == 0` vs `reverse != 0` (any non-zero `char`, incl. negative) | selects the `(cap, bb)` vs `(bb, cap)` argument order for the final `c2GJK`; no observable output either way | [x] |
| 52 | `gjk_cache` | NaN / inf in any of `a1..a4`, `b1..b5` | no validation anywhere; propagates into `c2GJK` and is swallowed (nothing is written out) | [x] |

## Rows that are undefined behaviour in C and therefore *not* bit-reproducible

These are recorded for completeness; a differential test cannot assert equality
because the C result depends on indeterminate stack contents, which no Rust
translation can reproduce. They are listed so they are not mistaken for gaps.

| ref | condition | why not testable |
|-----|-----------|------------------|
| 18 | `c2GJK` with `typeA`/`typeB` outside `{0,1,2}` | `c2Proxy pA/pB` are uninitialised locals; `c2MakeProxy` writes nothing, so `pA.count`/`pA.verts` are indeterminate. **Empirically confirmed non-reproducible**: six identical C calls returned `iterations` alternating between `1` and `3` (see the `err18` child-process output). With some stack contents the garbage `pA.count` is large enough that `c2Support` reads far out of bounds and the process faults. The Rust zero-initialises and is deterministic. Divergence is inherent to the C's UB. |
| — | `c2GJK` with `cache->count > 0` and `cache->iA[i] >= proxy count` | `pA.verts[iA]` reads the uninitialised tail of the proxy (only `verts[0..count]` are written). |
| — | `c2GJK` with `cache->count > 3` | the loop `for (i = 0; i < save_count; ++i) saveA[i] = ...` writes past the end of the `int saveA[3]` / `int saveB[3]` locals, corrupting `c2GJK`'s stack frame. **Confirmed: `count == 4` reliably crashes the C.** Also `cache->iA[3]` reads past the end of `iA` and `verts + i` for `i >= 4` writes past `s.d`. |
| — | `c2GJK` / `c2BBVerts` / `c2MakeProxy` / `c2Support` / `c22` / `c23` / `c2D` / `c2L` / `c2Witness` / `c2GJKSimplexMetric` with a `NULL` pointer argument | the C dereferences unconditionally (no null guard exists) ⇒ SIGSEGV in both implementations. Asserted only as "both would fault", not executed. |

The legal `cache->count` range is therefore **`0..=3`**, and `1..=3` is exercised
exhaustively over every index permutation of a 4-vertex proxy
(`generic_cache_count_boundaries`), plus `0` (row 7) and negatives (row 8).
