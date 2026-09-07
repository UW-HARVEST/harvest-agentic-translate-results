# ERRORS.md — Phase C error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. The library has **no** error enums,
no `RETURN_ERROR` macro, no `assert`, and never returns a negative sentinel.
Every "rejection" it performs is one of:

* a `switch` `default:` label that returns a neutral value (`0`, `(0,0)`),
* a `switch` with **no** `default:` label (silently leaves the output untouched),
* a null-pointer check that substitutes a default or skips a store,
* a floating-point degenerate/boundary guard (`<= 0`, `> rA + rB`, `< FLT_EPSILON^2`),
* the hard iteration cap `iter < 20`.

Note on C2_TYPE: the enum has 3 valid variants (0,1,2). A C enum parameter
accepts **any** `int`, so out-of-range enum values are real inputs that the C
handles (and that the Rust must handle identically). Rows 1–7 cover that.

Grep sites are given as `lib.c:LINE`.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| 1  | `c2MakeProxy` | `type` not in {0,1,2} (e.g. `3`, `-1`, `INT_MAX`, `INT_MIN`) — `switch` at `lib.c:113` has **no** `default:` | returns without writing **anything** to `*p`; `p->radius`, `p->count`, `p->verts` keep their prior (caller) contents |
| 2  | `c2Collided` | `typeA` not in {0,1,2} — `default:` `lib.c:613` | returns `0` |
| 3  | `c2Collided` | `typeA == C2_TYPE_CIRCLE (0)` and `typeB` not in {0,1,2} — `default:` `lib.c:585` | returns `0` |
| 4  | `c2Collided` | `typeA == C2_TYPE_AABB (1)` and `typeB` not in {0,1,2} — `default:` `lib.c:597` | returns `0` |
| 5  | `c2Collided` | `typeA == C2_TYPE_CAPSULE (2)` and `typeB` not in {0,1,2} — `default:` `lib.c:609` | returns `0` |
| 6  | `c2GJK` | `typeA` not in {0,1,2}: `c2MakeProxy` writes nothing, so the C proceeds with the **uninitialised** stack local `c2Proxy pA;` (`lib.c:375`) | **unspecified, and the C can crash.** `pA.count` is whatever the stack held, and `c2Support(pA.verts, pA.count, d)` (`lib.c:300`) loops `i < count` over it — measured outcomes for the C include returning `+inf` and taking SIGSEGV. No byte-exact comparison is possible. Asserted instead (`err06_07_c2GJK_invalid_type`, run out-of-process): the Rust `.so` always returns and is *deterministic* for every out-of-range enum value, and for the valid enum values the two libraries still agree exactly. |
| 7  | `c2GJK` | `typeB` not in {0,1,2} (same as row 6 for `pB`) | same as row 6 |
| 8  | `c2GJK` | `ax_ptr == NULL` — `lib.c:367` | substitutes `c2xIdentity()` (p=(0,0), r=(1,0)); no crash |
| 9  | `c2GJK` | `bx_ptr == NULL` — `lib.c:371` | substitutes `c2xIdentity()`; no crash |
| 10 | `c2GJK` | `outA == NULL` — `lib.c:509` | store skipped, distance still returned |
| 11 | `c2GJK` | `outB == NULL` — `lib.c:511` | store skipped, distance still returned |
| 12 | `c2GJK` | `iterations == NULL` — `lib.c:513` | store skipped |
| 13 | `c2GJK` | `cache == NULL` — `lib.c:382` / `lib.c:499` | cache is neither read nor written; cold start from vertex 0 |
| 14 | `c2GJK` | `cache != NULL` with `cache->count == 0` — `cache_was_good = !!cache->count` is false, `lib.c:383` | cache **not** read (cold start), but cache **is** written back on exit |
| 15 | `c2GJK` | `cache != NULL`, `cache->count != 0`, `cache->div == 0` | `c2L`/`c2Witness` divide by `1.0f/0.0f = inf`; C produces `inf`/`NaN` coordinates — Rust must produce the identical bit pattern |
| 16 | `c2GJK` | `cache != NULL`, `cache->count == 1..3`, and the metric check `!(min_metric < max_metric*2.0f && metric < -1.0e8f)` at `lib.c:404` is **true** (which it is for virtually every input, since `metric < -1.0e8f` is essentially never true — an original-source quirk) | `cache_was_read = 1`: the warm-started simplex is used verbatim, the cold-start block `lib.c:408` is skipped |
| 17 | `c2GJK` | `cache != NULL`, `cache->count == 1..3`, and `metric` is NaN (e.g. the cached vertices give a NaN `c2Det2`) | `min_metric`/`max_metric` both collapse to `metric_old` under C's `?:`; the `!( ... )` guard is still true → `cache_was_read = 1` |
| 18 | `c2GJK` | GJK never terminates early: loop bound `iter < 20` at `lib.c:424` | hard-caps at `iter == 20` iterations; `*iterations` is at most `20` |
| 19 | `c2GJK` | search direction degenerates: `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` at `lib.c:450` | `break` out of the loop, treat as converged |
| 20 | `c2GJK` | distance did not decrease: `d1 > d0` at `lib.c:446` | `break` (numerical-stall rejection) |
| 21 | `c2GJK` | duplicate support vertex: `iA == saveA[i] && iB == saveB[i]` at `lib.c:465` | `dup = 1` → `break` (the new vertex is written into the simplex but `s.count` is **not** incremented, and `iter` is **not** incremented) |
| 22 | `c2GJK` | `s.count == 3` after `c23` — `lib.c:440` | `hit = 1`, `a = b`, `dist = 0` (overlap); the `use_radius` branch is skipped entirely |
| 23 | `c2GJK` | `use_radius != 0` and `dist <= rA + rB` (shapes closer than the combined radii) — else-branch `lib.c:492` | `a = b = midpoint(a,b)`, `dist = 0` |
| 24 | `c2GJK` | `use_radius != 0` and `dist <= FLT_EPSILON` (witness points coincide) — same else-branch `lib.c:492` | `a = b = midpoint`, `dist = 0` |
| 25 | `c2GJK` | `use_radius != 0`, radius shrink makes the two witness points identical: `a.x == b.x && a.y == b.y` at `lib.c:490` | `dist` forced to `0` even though the subtraction gave a non-zero value |
| 26 | `c2GJK` | `use_radius == 0` | radii are ignored entirely; raw core-shape distance is returned (may be > 0 for touching capsules/circles) |
| 27 | `c2GJK` | `use_radius` is a nonzero value other than 1 (e.g. `2`, `-1`, `INT_MIN`) | C tests `else if (use_radius)`, i.e. *any* nonzero is true — identical to `use_radius == 1` |
| 28 | `c2GJKSimplexMetric` | `s->count` not in {2,3} (i.e. `0`, `1`, `4`, negative, huge) — `default:` falls through into `case 1:` at `lib.c:161-163` | returns `0.0f` |
| 29 | `c2D` | `s->count == 3` or any other value — `case 3: default:` `lib.c:291` | returns `(0,0)` |
| 30 | `c2D` | `s->count == 2` and `c2Det2(ab, -a.p) <= 0` — `lib.c:287` | returns `c2CCW90(ab)` instead of `c2Skew(ab)` |
| 31 | `c2Witness` | `s->count` not in {1,2,3} — `default:` `lib.c:331` | writes `(0,0)` to **both** `*a` and `*b` |
| 32 | `c2Witness` | `s->div == 0` | `den = 1.0f/0.0f = +inf`; `case 1` is unaffected, `case 2/3` produce `inf`/`NaN` — must match bit-for-bit |
| 33 | `c2L` | `s->count` not in {1,2} — `default:` `lib.c:353` | returns `(0,0)` |
| 34 | `c2L` | `s->div == 0` with `count == 2` | `den = +inf` → `inf`/`NaN` coordinates |
| 35 | `c2Support` | `count <= 0` (`0`, negative) | still dereferences `verts[0]` unconditionally (`lib.c:299`), loop body never runs, returns `0` |
| 36 | `c2Support` | all dots equal / tie (`dot > dmax` is strict) | returns the **lowest** index, never a later tie |
| 37 | `c22` | `v <= 0` (`lib.c:190`) | collapse to vertex A: `count = 1`, `div = 1`, `a.u = 1` |
| 38 | `c22` | `u <= 0` (`lib.c:194`) | collapse to vertex B copied into slot A: `count = 1`, `div = 1` |
| 39 | `c23` | `vAB <= 0 && uCA <= 0` (`lib.c:221`) | collapse to A, `count = 1` |
| 40 | `c23` | `uAB <= 0 && vBC <= 0` (`lib.c:225`) | collapse to B, `count = 1` |
| 41 | `c23` | `uBC <= 0 && vCA <= 0` (`lib.c:230`) | collapse to C, `count = 1` |
| 42 | `c23` | `uAB > 0 && vAB > 0 && wABC <= 0` (`lib.c:235`) | edge AB, `count = 2` |
| 43 | `c23` | `uBC > 0 && vBC > 0 && uABC <= 0` (`lib.c:240`) | edge BC (shifted into slots A,B), `count = 2` |
| 44 | `c23` | `uCA > 0 && vCA > 0 && vABC <= 0` (`lib.c:247`) | edge CA (B←A, A←C), `count = 2` |
| 45 | `c23` | degenerate triangle, `area == 0` → `uABC = vABC = wABC = 0` (`lib.c:217-220`) | **verified empirically:** the final `else` is NOT reached. Because all three `?ABC <= 0` tests then hold, the C always exits through one of the earlier vertex-collapse or edge branches (`count = 1` or `count = 2`), never `count = 3, div = 0`. The Rust must reproduce that same branch choice. |
| 46 | `c2Div` | `b == 0` | `1.0f/0.0f = +inf`; result is `(±inf, ±inf)` or `NaN` for a zero component |
| 47 | `c2Norm` | `a == (0,0)` → `c2Len(a) == 0` | `c2Div(a, 0)` = `(0*inf, 0*inf)` = `(NaN, NaN)` |
| 48 | `c2Len` | `c2Dot(a,a)` overflows to `+inf`, or any component is NaN | `sqrtf(inf) = inf`, `sqrtf(NaN) = NaN`; Rust `f32::sqrt` must agree |
| 49 | `c2Len` | `c2Dot(a,a) < 0` (only reachable with NaN inputs) | `sqrtf` of a negative → `NaN` |
| 50 | `c2Maxv` / `c2Minv` | any NaN component. C uses the ternary idiom `a.x > b.x ? a.x : b.x`, so a NaN comparison is false and **`b`** is selected | must NOT be `f32::max`/`f32::min` (which prefer the non-NaN operand) |
| 51 | `c2Clampv` | `lo > hi` (inverted box) | `c2Maxv(lo, c2Minv(a,hi))` returns `lo` — no rejection, no assert |
| 52 | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` → `n = (0,0)`, `c2Dot(n,n) == 0`, and `da == 0` (not `< 0`) so the `db < 0` branch may divide by zero (`lib.c:564`) | `da / 0.0f` = `NaN` or `±inf` → `d2 = NaN` → `d2 < r*r` is false → returns `0` |
| 53 | `c2CircletoCircle` | negative radii such that `A.r + B.r < 0` | `r2 = (A.r+B.r)^2` is positive again, so a negative-radius pair still reports a hit |
| 54 | `c2CircletoAABB` | negative radius | `r2 = A.r*A.r >= 0`; `d2 < r2` can still be true |
| 55 | `c2AABBtoAABB` | any NaN coordinate | all four `<` comparisons are false → `!(0) == 1` → reports **collision** |
| 56 | `c2AABBtoCapsule` / `c2CapsuletoCapsule` | `c2GJK(...) != 0` → `return 0` (`lib.c:528`, `lib.c:534`); note the C tests the float against `0` implicitly, so `NaN != 0` is true → returns `0` | `0` when the GJK distance is non-zero **or NaN**, `1` only when exactly `0.0f` |
| 57 | `aabb` | any NaN / inf / denormal input coordinate | no validation at all; the three `c2Collided` results are packed as `r0 + (r1<<1) + (r2<<2)`, giving `0..7` |
| 58 | `c2BBVerts` | inverted AABB (`min > max`) | no check; writes the 4 corners in the same order regardless |
| 59 | `c2GJK` | `cache->iA[i]` / `cache->iB[i]` outside `0..proxy.count-1` (but within the 8-element `verts` array) | C reads an **uninitialised** `c2Proxy::verts[]` slot, so the resulting distance is unspecified. Defined and asserted: the call terminates, `*iterations ∈ 0..=20`, the written-back `cache->count ∈ 0..=3`, and the Rust must not panic or abort. (This is why Phase B row 67 restricts hand-made cache indices to the proxy's valid range.) |
| 60 | `c2GJK` | `cache->count >= 4` | **verified empirically: the C crashes.** `int saveA[3], saveB[3];` are written at index `i < save_count == cache->count`, so `saveA[3]`/`saveB[3]` overrun their arrays and corrupt the stack frame; `verts[4]` (`c2sv *verts = &s.a;`) also runs past `c2Simplex::d` onto `div`/`count` and beyond. `c_src/build/libharvest-work-UoQHxV.so` terminates with **SIGSEGV (exit 139)** for `count = 4` and `count = 5`, while `count = 1..3` return normally. This is unbounded stack corruption, not a rejection: there is no defined result to match. The Rust bounds the loops instead, so it returns normally. Verified by `err60_cache_count_four_aliasing`, which runs the C call in a *subprocess* (asserting it dies) and the Rust call in-process (asserting it returns a deterministic value). |
| 61 | `c2GJK` | `cache->count < 0` (`-1`, `INT_MIN`) | `!!count` is true so `cache_was_good` is set, but every `for i < count` loop body is skipped. Result is fully defined: `dist = 0`, `*iterations = 0`, and the cache is written back with `metric = 0`, `count` unchanged (negative), `div = cache->div`. Rust matches bit-for-bit. |
| 62 | `c2GJK` | `A` or `B` is a NULL shape pointer | `c2MakeProxy` dereferences it unconditionally → both builds fault. Not a rejection the library implements; excluded from differential testing (a C segfault has no return value to compare). |
