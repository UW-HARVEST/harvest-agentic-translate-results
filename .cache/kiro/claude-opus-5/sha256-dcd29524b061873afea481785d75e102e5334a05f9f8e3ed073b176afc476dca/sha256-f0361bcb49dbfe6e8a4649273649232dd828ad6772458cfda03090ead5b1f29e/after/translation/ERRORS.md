# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. This library has **no error-return
macros, no `assert`, no `return -1`, no `return NULL`, and no error enum** —
`gjk` returns `void` and `c2GJK` returns a `float` distance that is never a
sentinel. Its entire "rejection" surface is therefore made of:

* null-pointer guards (`if (!ax_ptr)`, `if (cache)`, `if (outA)`, …),
* `switch` statements with a missing or fall-through `default:` label,
* degenerate/early-out `break` conditions inside the GJK loop,
* the hard iteration cap `while (iter < 20)`,
* unguarded divisions (`1.0f / s->div`, `1.0f / b` in `c2Div`) that produce
  `inf`/`NaN` instead of erroring,
* unguarded indexing (`verts[0]` when `count <= 0`, `pA.verts[iA]` with a
  caller-supplied cache index).

Every row below is one distinct rejection/degenerate branch that the C code
*actually* contains. `✔` = a differential test exists and passes against both
`.so`s.

| # | function | trigger (exact invalid input/condition) | expected C result | ✔ |
|---|----------|------------------------------------------|-------------------|---|
| 1 | `c2GJK` | `ax_ptr == NULL` | substitutes `c2xIdentity()`; no error | ✔ |
| 2 | `c2GJK` | `bx_ptr == NULL` | substitutes `c2xIdentity()`; no error | ✔ |
| 3 | `c2GJK` | `cache == NULL` | cache read *and* write both skipped | ✔ |
| 4 | `c2GJK` | `outA == NULL` | `*outA` not written; return value unaffected | ✔ |
| 5 | `c2GJK` | `outB == NULL` | `*outB` not written; return value unaffected | ✔ |
| 6 | `c2GJK` | `iterations == NULL` | `*iterations` not written | ✔ |
| 7 | `c2GJK` | all of `outA`,`outB`,`iterations`,`cache` NULL | only the `float` return is produced | ✔ |
| 8 | `c2GJK` | `cache->count == 0` (`cache_was_good` false) | cold-start simplex from vert 0; cache is still *written* on exit | ✔ |
| 9 | `c2GJK` | `cache->count < 0` | read loop skipped, `s.count` negative ⇒ `switch` matches nothing, `c2L`/`c2D` take `default`, loop breaks on `dot(d,d) < eps²`; returns `0.0`, cache rewritten with the negative count | ✔ |
| 10 | `c2GJK` | `cache->count != 0` and `!(min_metric < max_metric*2 && metric < -1.0e8f)` | `cache_was_read = 1` — the `metric < -1.0e8f` conjunct is the C original's inverted/typo'd test, so the cache is accepted for essentially every finite metric. Must be replicated, not "fixed". | ✔ |
| 11 | `c2GJK` | `cache->count == 4` … `> 3` | reads `cache->iA[3]`, which aliases `cache->iB[0]`, and `cache->iB[3]`, which aliases `cache->div` — out-of-bounds by C's own array bounds. **Documented as UB; not asserted.** | n/a |
| 12 | `c2GJK` | `cache->iA[i]` / `cache->iB[i]` outside `[0, proxy.count)` | unguarded `pA.verts[iA]` read — inside `c2v verts[8]` for `0..7` (reads *uninitialised* slots for a circle/capsule proxy), wild OOB past 7. **UB; not asserted.** | n/a |
| 13 | `c2GJK` | `typeA`/`typeB` not in `{0,1,2}` (out-of-range enum across FFI) | `c2MakeProxy`'s `switch` has **no `default:`**, so the proxy is left as the *uninitialised stack* `c2Proxy pA;` of `c2GJK`. Result is indeterminate in C. **UB; asserted only for "does not abort", not for value equality.** | ✔ (no-crash) |
| 14 | `c2GJK` | `use_radius == 0` | radius shrink block skipped entirely; raw simplex distance returned | ✔ |
| 15 | `c2GJK` | `use_radius != 0` and `dist <= rA + rB` | `else` branch: `a = b = midpoint(a,b)`, `dist = 0` | ✔ |
| 16 | `c2GJK` | `use_radius != 0` and `dist <= FLT_EPSILON` (touching/overlapping) | same `else` branch: midpoint collapse, `dist = 0` | ✔ |
| 17 | `c2GJK` | `use_radius != 0`, shrink applied, then `a.x==b.x && a.y==b.y` | `dist = 0` (exact float equality test) | ✔ |
| 18 | `c2GJK` | simplex reaches `s.count == 3` | `hit = 1`, loop breaks, `a = b`, `dist = 0`, radius block skipped | ✔ |
| 19 | `c2GJK` | `d1 > d0` (no progress toward origin) | `break` out of the loop mid-iteration; `iter` **not** incremented | ✔ |
| 20 | `c2GJK` | `c2Dot(d,d) < FLT_EPSILON * FLT_EPSILON` (degenerate search dir) | `break`; `iter` not incremented | ✔ |
| 21 | `c2GJK` | new support pair duplicates a saved pair (`dup`) | `break` *after* writing `verts[s.count]` but *without* `++s.count` | ✔ |
| 22 | `c2GJK` | 20 iterations reached (`while (iter < 20)`) | loop exits normally, witness computed from whatever simplex exists. **Measured**: structurally unreachable — proxies hold at most 4 vertices, so the `dup` (duplicate support pair) test always terminates first. The highest `iter` observed anywhere in the suite is **3**. The test asserts `iter <= 20` on both sides rather than claiming to reach the cap. | ✔ |
| 23 | `c2MakeProxy` | `type` not in `{0,1,2}` | missing `default:` ⇒ `*p` completely untouched (caller's bytes preserved) | ✔ |
| 24 | `c2GJKSimplexMetric` | `s->count == 1` | `case 1: return 0` | ✔ |
| 25 | `c2GJKSimplexMetric` | `s->count` anything other than 1/2/3 (0, negative, ≥4) | `default:` falls through into `case 1:` ⇒ `0` | ✔ |
| 26 | `c2D` | `s->count == 3` | `case 3`/`default` ⇒ `c2V(0,0)` | ✔ |
| 27 | `c2D` | `s->count` not 1/2/3 (0, negative, ≥4) | `default` ⇒ `c2V(0,0)` | ✔ |
| 28 | `c2D` | `s->count == 2` and `c2Det2(ab, -a.p) == 0` (collinear w/ origin) | `> 0` is false ⇒ takes `c2CCW90(ab)`, not `c2Skew(ab)` | ✔ |
| 29 | `c2Witness` | `s->count` not 1/2/3 | `default` ⇒ both outputs `c2V(0,0)` (note: `den` already computed) | ✔ |
| 30 | `c2Witness` | `s->div == 0` with `count >= 2` | `den = 1/0 = +inf` ⇒ `inf`/`NaN` propagated into outputs, no error | ✔ |
| 31 | `c2Witness` | `s->div == 0` with `count == 1` | `den` unused ⇒ exact copy of `sA`/`sB`, no NaN | ✔ |
| 32 | `c2L` | `s->count == 3` or any value not 1/2 | `default` ⇒ `c2V(0,0)` | ✔ |
| 33 | `c2L` | `s->div == 0`, `count == 2` | `den = +inf` ⇒ `inf`/`NaN` components | ✔ |
| 34 | `c2Support` | `count <= 0` (0 or negative) | still dereferences `verts[0]` unconditionally, loop body never runs, returns `0` | ✔ |
| 35 | `c2Support` | all dots equal / ties | strict `>` ⇒ keeps the **lowest** index | ✔ |
| 36 | `c2Support` | `d` contains `NaN` | every `dot > dmax` is false ⇒ returns `0` | ✔ |
| 37 | `c2Div` | `b == 0.0` | `c2Mulvs(a, 1/0)` ⇒ `±inf`, or `NaN` for a zero component | ✔ |
| 38 | `c2Div` | `b == -0.0` | `1/-0 = -inf` ⇒ sign-flipped infinities | ✔ |
| 39 | `c2Norm` | `a == (0,0)` | `c2Len = 0` ⇒ `1/0 = inf`, `0*inf = NaN` for both components | ✔ |
| 40 | `c2Len` | `a` has a negative-squared overflow / `inf` component | `dot` overflows to `+inf`, `sqrtf(inf) = inf`; `NaN` in ⇒ `NaN` out | ✔ |
| 41 | `c2Maxv`/`c2Minv` | either operand component is `NaN` | ternary `a>b ? a : b` is false for NaN ⇒ **always returns `b`'s component** | ✔ |
| 42 | `c2Clampv` | `lo > hi` (inverted range) | no validation: `c2Maxv(lo, c2Minv(a,hi))` ⇒ returns `lo` | ✔ |
| 43 | `c22` | `v <= 0` (origin beyond `a`) | collapse to vertex `a`, `u=1`, `div=1`, `count=1` | ✔ |
| 44 | `c22` | `u <= 0` (origin beyond `b`) | `s->a = s->b`, `u=1`, `div=1`, `count=1` | ✔ |
| 45 | `c22` | `a.p == b.p` (duplicate points ⇒ `u = v = 0`) | `v <= 0` wins ⇒ collapse to `a` | ✔ |
| 46 | `c23` | `vAB <= 0 && uCA <= 0` | collapse to vertex `a` | ✔ |
| 47 | `c23` | `uAB <= 0 && vBC <= 0` | collapse to vertex `b` (`s->a = s->b`) | ✔ |
| 48 | `c23` | `uBC <= 0 && vCA <= 0` | collapse to vertex `c` (`s->a = s->c`) | ✔ |
| 49 | `c23` | `uAB>0 && vAB>0 && wABC<=0` | edge AB, `count=2` | ✔ |
| 50 | `c23` | `uBC>0 && vBC>0 && uABC<=0` | edge BC with the `a=b; b=c` shuffle | ✔ |
| 51 | `c23` | `uCA>0 && vCA>0 && vABC<=0` | edge CA with the `b=a; a=c` shuffle | ✔ |
| 52 | `c23` | degenerate triangle, `area == 0` ⇒ `uABC=vABC=wABC=0` | **Measured**: a bit-exactly zero area ALWAYS matches an earlier vertex or edge branch — the interior `else` is never reached. Verified over 400 000 exact-zero-area triples (result counts were only 1 or 2, never 3), and asserted on both libraries. | ✔ |
| 53 | `c23` | `else` (interior) with `div == 0` | `count = 3`, `div = 0` ⇒ a later `c2Witness` divides by zero. Reachable **only** through subnormal/overflow underflow, where `uABC`, `vABC` and `wABC` all flush to zero; three such inputs were found by exhaustive search over 2 000 000 random triples and are pinned in the test. | ✔ |
| 54 | `c2BBVerts` | inverted AABB (`min > max`) | no validation; emits a self-intersecting quad | ✔ |
| 55 | `gjk` | `reverse == 0` | AABB is shape A, capsule is shape B | ✔ |
| 56 | `gjk` | `reverse != 0` (incl. negative `char`, e.g. `-1`, `0x80`) | capsule is shape A, AABB is shape B — `if (reverse)` is a plain truth test | ✔ |
| 57 | `gjk` | `a == NULL` or `b == NULL` | forwarded straight into `c2GJK`'s `if (outA)` / `if (outB)` guards ⇒ silently skipped | ✔ |
| 58 | `gjk` | `a` and `b` alias the same `c2v` | no aliasing guard; `*outA` then `*outB` ⇒ final value is `b` | ✔ |
| 59 | `gjk` | zero-extent AABB (`a1==a3 && a2==a4`) | degenerate 4-vert proxy with 4 identical verts | ✔ |
| 60 | `gjk` | zero-length capsule (`b1==b3 && b2==b4`) | degenerate 2-vert proxy | ✔ |
| 61 | `gjk` | negative capsule radius `b5 < 0` | no validation; `dist > rA+rB` compares against a negative sum, `dist -= rA+rB` *grows* the distance | ✔ |
| 62 | `gjk` | `NaN` / `±inf` in any of `a1..b5` | no validation; propagates through every comparison (all `>`/`<=` false for NaN) | ✔ |
