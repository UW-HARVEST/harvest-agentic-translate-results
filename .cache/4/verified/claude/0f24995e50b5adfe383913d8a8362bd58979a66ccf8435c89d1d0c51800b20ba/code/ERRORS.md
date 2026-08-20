# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c` (the only C translation unit).
There is **no** error enum, **no** `RETURN_ERROR` macro, **no** `assert()`, and
**no** `return -1` / `return NULL` in this library.  Its entire
rejection/degenerate surface consists of:

* `switch` statements with (or *without*) a `default:` label —
  lines 109, 156, 278, 308, 343, 426, 572, 574, 586, 598, 619;
* NULL-pointer guards — lines 363, 367, 378, 495, 505, 507, 509;
* the cache-validity predicate — lines 379, 400;
* the six `break` statements that terminate the GJK loop —
  lines 438, 443, 448, 467, and the `while (iter < 20)` bound at line 420;
* the `use_radius` clamp predicate — lines 480–481, 486;
* every division that can divide by zero — lines 334, 342, 492, 560;
* the hard-coded limits: `20` (line 420), `FLT_MAX` (line 416),
  `FLT_EPSILON` = `1.1920928955078125e-7` (line 446, 481),
  `-1.0e8f` (line 400), `2.0f` (line 400), `0.5f` (line 489),
  and the fixed array bounds `c2Proxy::verts[8]`, `c2Simplex` = 4 `c2sv`,
  `c2GJKCache::iA[3]` / `iB[3]`, `saveA[3]` / `saveB[3]`.

Legend for **status**: `[x]` = differential test written **and passing**;
`UB` = the C code's behaviour on this input is undefined / reads
uninitialised memory, so it is not differentially specifiable (documented,
with the reason, at the bottom of this file).

## Table

| #  | function | trigger (exact invalid input / condition) | expected C result | status |
|----|----------|-------------------------------------------|-------------------|--------|
|  1 | `c2Collided` | `typeA` ∉ {0,1,2} — e.g. `3`, `7`, `0xFFFFFFFF` (outer `default:`, line 609) | returns `0`; neither `A` nor `B` dereferenced | [x] |
|  2 | `c2Collided` | `typeA == C2_TYPE_CIRCLE`, `typeB` ∉ {0,1,2} (inner `default:`, line 581) | returns `0` | [x] |
|  3 | `c2Collided` | `typeA == C2_TYPE_AABB`, `typeB` ∉ {0,1,2} (inner `default:`, line 593) | returns `0` | [x] |
|  4 | `c2Collided` | `typeA == C2_TYPE_CAPSULE`, `typeB` ∉ {0,1,2} (inner `default:`, line 605) | returns `0` | [x] |
|  5 | `omni_collide` | `type_a` ∉ {0,1,2}, `type_b` valid | returns `0` (garbage ptr from `ptr_from_parts` never dereferenced) | [x] |
|  6 | `omni_collide` | `type_a` valid, `type_b` ∉ {0,1,2} | returns `0` | [x] |
|  7 | `omni_collide` | both `type_a` and `type_b` ∉ {0,1,2} | returns `0` | [x] |
|  8 | `omni_collide` | enum value `3` (one past `C2_TYPE_AABB`) specifically, for all 4 (a,b) pairings with valid types | returns `0` | [x] |
|  9 | `c2MakeProxy` | `type` ∉ {0,1,2} — `switch` at line 109 has **no** `default:` | function writes **nothing**; `*p` left byte-for-byte unchanged | [x] |
| 10 | `c2GJKSimplexMetric` | `s->count == 1` (explicit `case 1:`) | returns `0.0f` | [x] |
| 11 | `c2GJKSimplexMetric` | `s->count` ∉ {1,2,3} — `0`, `4`, `-1`, `INT_MIN`, `INT_MAX` (`default:` falls through to `case 1:`, line 157) | returns `0.0f` | [x] |
| 12 | `c2D` | `s->count == 3` or ∉ {1,2,3} (`case 3: default:`, line 288) | returns `c2V(0,0)` | [x] |
| 13 | `c2D` | `s->count == 2` and `c2Det2(ab, -a) <= 0` (the non-`c2Skew` fallback, line 285) | returns `c2CCW90(ab)` | [x] |
| 14 | `c2L` | `s->count` ∉ {1,2} — `0`, `3`, `4`, `-1` (`default:`, line 349) | returns `c2V(0,0)` | [x] |
| 15 | `c2L` | `s->div == 0` → `den = 1.0f/0 = +inf`, `count` ∈ {1,2} | `count==1`: returns `s->a.p` unscaled; `count==2`: `inf`/`NaN` components (exact bit pattern must match) | [x] |
| 16 | `c2L` | `s->div == -0.0` → `den = -inf` | `-inf`/`NaN` components | [x] |
| 17 | `c2Witness` | `s->count` ∉ {1,2,3} — `0`, `4`, `-1` (`default:`, line 327) | `*a = *b = c2V(0,0)` | [x] |
| 18 | `c2Witness` | `s->div == 0` → `den = +inf`, `count` ∈ {2,3} | `inf`/`NaN` components (exact bits must match) | [x] |
| 19 | `c2Witness` | `s->count == 1` with `s->div == 0` | `*a = s->a.sA`, `*b = s->a.sB`; `den` computed but unused | [x] |
| 20 | `c2Support` | `count == 0` — loop `for (i=1; i<count; ...)` never runs but `verts[0]` **is** read at line 295 | returns `0` | [x] |
| 21 | `c2Support` | `count < 0` (e.g. `-1`, `INT_MIN`) | returns `0`, `verts[0]` still read | [x] |
| 22 | `c2Support` | `count == 1` (boundary: no loop iteration) | returns `0` | [x] |
| 23 | `c2Support` | all dots equal / `d == (0,0)` → `dot > dmax` never true | returns `0` (first index wins ties) | [x] |
| 24 | `c2Support` | `d` contains NaN → every `dot > dmax` comparison is false | returns `0` | [x] |
| 25 | `c2Div` | `b == 0.0f` → `1.0f/0 = +inf`, then `a.x*inf` | `±inf` per component, `NaN` for a `0.0` component | [x] |
| 26 | `c2Div` | `b == -0.0f` → `-inf` | sign-flipped `±inf` / `NaN` | [x] |
| 27 | `c2Norm` | zero-length input `c2V(0,0)` → `c2Len == 0` → division by zero | `NaN, NaN` | [x] |
| 28 | `c2Norm` | input with a `NaN` or `inf` component | `NaN` propagation, exact bits must match | [x] |
| 29 | `c2Len` | input with a `NaN` component → `sqrtf(NaN)` | `NaN` | [x] |
| 30 | `c2Len` | `c2Dot(a,a)` overflows to `+inf` (e.g. `c2V(1e30,1e30)`) | `sqrtf(inf) = inf` | [x] |
| 31 | `c2GJK` | `ax_ptr == NULL` (line 363) | `ax = c2xIdentity()`, no deref | [x] |
| 32 | `c2GJK` | `bx_ptr == NULL` (line 367) | `bx = c2xIdentity()`, no deref | [x] |
| 33 | `c2GJK` | `cache == NULL` (lines 378, 495) | cache neither read nor written; fresh simplex | [x] |
| 34 | `c2GJK` | `outA == NULL` (line 505) | not written; return value still the distance | [x] |
| 35 | `c2GJK` | `outB == NULL` (line 507) | not written | [x] |
| 36 | `c2GJK` | `iterations == NULL` (line 509) | not written | [x] |
| 37 | `c2GJK` | all of `outA`, `outB`, `iterations`, `cache` `NULL` simultaneously | only the `float` return value observable | [x] |
| 38 | `c2GJK` | `cache != NULL` with `cache->count == 0` → `cache_was_good == 0` (line 379) | cache **not** read; fresh 1-vertex simplex; cache still written on exit | [x] |
| 39 | `c2GJK` | `cache != NULL` with `cache->count < 0` → `!!count` is **true** so the cache *is* "good", but `for (i=0; i<count)` never runs (line 381) | `s.count` = the negative value, `s.div = cache->div`; metric `0`; every downstream `switch` takes `default`; `c2Witness` → `(0,0)`; `dist == 0` | [x] |
| 40 | `c2GJK` | cache predicate `min_metric < max_metric*2.0f && metric < -1.0e8f` is **true** (line 400) → `cache_was_read` stays `0` | cached simplex silently discarded, fresh 1-vertex simplex built | [x] |
| 41 | `c2GJK` | cache predicate false → `cache_was_read = 1` (warm start) | cached simplex kept, GJK resumes from it | [x] |
| 42 | `c2GJK` | GJK terminates because `s.count == 3` (line 436) → `hit = 1` | `a = b`, returns `0.0f` exactly | [x] |
| 43 | `c2GJK` | GJK terminates because `d1 > d0` (no progress, line 442) | `break`; `iter` is **not** incremented for that pass | [x] |
| 44 | `c2GJK` | GJK terminates because `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (line 446) | `break` | [x] |
| 45 | `c2GJK` | GJK terminates because the new support point duplicates a saved one (line 466) | `break`; the new vertex is written to `verts[s.count]` but `s.count` is **not** incremented | [x] |
| 46 | `c2GJK` | GJK runs the `while (iter < 20)` bound out (line 420) | loop exits with `iter == 20` | **unreachable** (see below) |
| 47 | `c2GJK` | `use_radius == 0` (line 477) | raw simplex distance returned, **no** radius subtraction, `a`/`b` unmodified | [x] |
| 48 | `c2GJK` | `use_radius != 0` and `dist <= rA + rB` (line 480) | `a = b = 0.5*(a+b)`, returns `0.0f` | [x] |
| 49 | `c2GJK` | `use_radius != 0` and `dist <= FLT_EPSILON` (line 481) | `a = b = 0.5*(a+b)`, returns `0.0f` | [x] |
| 50 | `c2GJK` | `use_radius != 0`, radius branch taken, and afterwards `a.x==b.x && a.y==b.y` (line 486) | `dist` forced to `0.0f` although `a`/`b` keep their shifted values | [x] |
| 51 | `c2GJK` | `use_radius` = any non-zero int other than 1 (e.g. `2`, `-1`, `INT_MIN`) — C tests truthiness | same as `use_radius == 1` | [x] |
| 52 | `c2GJK` | shapes with negative radius (`c2Circle.r < 0`, `c2Capsule.r < 0`) → `rA + rB < 0` | `dist > rA+rB` more easily true; `dist -= rA+rB` **increases** dist | [x] |
| 53 | `c2GJK` | `NaN` in a shape field → every `<`/`>` comparison false: `d1 > d0` false, `dist > rA+rB` false | takes the `else` (midpoint) branch, returns `0.0f`; loop runs the full 20 iterations | [x] |
| 54 | `c2AABBtoAABB` | inverted AABB (`min > max`) — pure comparison code, no guard (line 514) | `!(d0|d1|d2|d3)` evaluated as-is; can report a "hit" for empty boxes | [x] |
| 55 | `c2AABBtoAABB` | `NaN` in any component → all four `<` false → `d0..d3` all `0` | returns `1` | [x] |
| 56 | `c2AABBtoCapsule` | `c2GJK(...) != 0` is the *only* test (line 523): any non-zero float ⇒ `0` | `-0.0f` is `== 0` so it would return `1`; verified identical | [x] |
| 57 | `c2CapsuletoCapsule` | same `!= 0` float test (line 529), incl. `NaN` distance (`NaN != 0` is **true** ⇒ returns `0`) | identical branch | [x] |
| 58 | `c2CircletoCircle` | `A.r + B.r < 0` (both radii negative) → `r2 = (A.r+B.r)^2 > 0` | can report a hit for negative radii | [x] |
| 59 | `c2CircletoCircle` | `NaN` position/radius → `d2 < r2` false | returns `0` | [x] |
| 60 | `c2CircletoAABB` | inverted AABB — `c2Clampv` = `c2Maxv(lo, c2Minv(a,hi))` with `lo > hi` | no guard; clamp yields `lo`, result computed from that | [x] |
| 61 | `c2CircletoAABB` | `NaN` in `A.p` — `c2Maxv`/`c2Minv` are `>`/`<` selects, so NaN picks the *other* operand | exact NaN-vs-select behaviour must match | [x] |
| 62 | `c2CircletoAABB` | negative `A.r` → `r2 = A.r*A.r > 0` | can report a hit | [x] |
| 63 | `c2CircletoCapsule` | `da < 0` (point behind `B.a`, line 555) | `d2 = dot(ap,ap)` | [x] |
| 64 | `c2CircletoCapsule` | `da >= 0 && db < 0` (line 559) → divides by `c2Dot(n,n)` | projection branch | [x] |
| 65 | `c2CircletoCapsule` | `da >= 0 && db >= 0` (line 562) | `d2 = dot(bp,bp)` | [x] |
| 66 | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` → `n == (0,0)`, `da == 0` (not `< 0`), `db == 0` (not `< 0`) → the `bp` branch; **the `/c2Dot(n,n)` division is not reached** | no division by zero; `d2 = dot(A.p-B.b, ...)` | [x] |
| 67 | `c2CircletoCapsule` | degenerate capsule **and** `da >= 0 && db < 0` forced via NaN/inf so `c2Dot(n,n) == 0` **is** divided by | `da/0` → `±inf`/`NaN`, propagated | [x] |
| 68 | `c2CircletoCapsule` | `NaN` anywhere → `da < 0` false, `db < 0` false | `bp` branch, then `d2 < r*r` false ⇒ returns `0` | [x] |
| 69 | `c22` | `v <= 0` (line 186), incl. `v == 0` and `v == -0.0` | `count = 1`, `div = 1`, `a.u = 1` | [x] |
| 70 | `c22` | `v > 0 && u <= 0` (line 190) | `a = b`, `count = 1`, `div = 1` | [x] |
| 71 | `c22` | `u > 0 && v > 0` | `count = 2`, `div = u+v` | [x] |
| 72 | `c22` | `u`/`v` `NaN` → both `<= 0` tests false | falls to the `else`, `count = 2`, `div = NaN` | [x] |
| 73 | `c23` | branch 1: `vAB <= 0 && uCA <= 0` (line 217) | `count = 1` | [x] |
| 74 | `c23` | branch 2: `uAB <= 0 && vBC <= 0` (line 221) | `a = b`, `count = 1` | [x] |
| 75 | `c23` | branch 3: `uBC <= 0 && vCA <= 0` (line 226) | `a = c`, `count = 1` | [x] |
| 76 | `c23` | branch 4: `uAB > 0 && vAB > 0 && wABC <= 0` (line 231) | `count = 2`, `div = uAB+vAB` | [x] |
| 77 | `c23` | branch 5: `uBC > 0 && vBC > 0 && uABC <= 0` (line 236) | `a = b; b = c`, `count = 2` | [x] |
| 78 | `c23` | branch 6: `uCA > 0 && vCA > 0 && vABC <= 0` (line 243) | `b = a; a = c`, `count = 2` | [x] |
| 79 | `c23` | fall-through `else` (line 250) | `count = 3`, `div = uABC+vABC+wABC` | [x] |
| 80 | `c23` | degenerate triangle: `a == b == c` → `area == 0`, all u/v `0`, so branch 1 (`0<=0 && 0<=0`) wins | `count = 1` | [x] |
| 81 | `c23` | `NaN` in a vertex → every comparison false ⇒ fall-through `else`, `count = 3` | `div = NaN` | [x] |
| 82 | `c2Maxv` / `c2Minv` | either operand `NaN` (ternary `a>b?a:b`) | `NaN` on the *unselected* side of the compare is silently dropped; exact operand choice must match | [x] |
| 83 | `c2Clampv` | `lo > hi` (invalid range, no guard) | `c2Maxv(lo, c2Minv(a,hi))` → returns `lo` | [x] |
| 84 | `c2BBVerts` | inverted / `NaN` AABB (no validation at all) | writes the 4 corners verbatim | [x] |
| 85 | `ptr_from_parts` | `typ` ∈ {0,1,2} — `malloc` result is **never checked for NULL** (lines 621, 627, 631) | dereferenced unconditionally; on OOM this would fault in both | n/a (cannot force OOM) |
| 86 | `ptr_from_parts` | `typ` ∉ {0,1,2}: `switch` has no `default:` and control **falls off the end of a non-`void` function** | **UB** — no return value | UB |
| 87 | `c2GJK` | `typeA`/`typeB` ∉ {0,1,2}: `c2MakeProxy` writes nothing, so `c2Proxy pA;` (line 371) stays **uninitialised** and is then read at line 407 | **UB** — reads uninitialised stack | UB |
| 88 | `c2GJK` | `A == NULL` / `B == NULL` with a valid `type` | **UB** — unconditional NULL deref in `c2MakeProxy`, faults in both | UB |
| 89 | `c2GJK` | `cache->count > 3`: indexes `cache->iA[i]`/`iB[i]` past their 3-element bounds, and `saveA[i]`/`saveB[i]` past *their* 3-element bounds | **UB** — out-of-bounds reads/writes on differently-laid-out stack frames | UB |
| 90 | `c2GJK` | `cache->iA[i]`/`iB[i]` outside `0..proxy.count` (no range check at line 384) | **UB** — `pA.verts[iA]` past `verts[8]` for large indices; in-range-of-`verts[8]` values *are* covered by row 41 | UB (partially [x]) |
| 91 | `c22` / `c23` / `c2D` / `c2L` / `c2Witness` / `c2GJKSimplexMetric` | `s == NULL` | **UB** — unconditional deref, faults in both | UB |
| 92 | `c2Support` | `verts == NULL` (with any `count`) — `verts[0]` read unconditionally | **UB** — faults in both | UB |
| 93 | `c2BBVerts` / `c2MakeProxy` | `out == NULL` / `bb == NULL` / `p == NULL` | **UB** — unconditional deref | UB |
| 94 | `c2Collided` | valid `typeA`/`typeB` but `A`/`B` NULL, or pointing at a shape of the *wrong* type (e.g. a 12-byte `c2Circle` read as a 20-byte `c2Capsule`) | **UB** — the C blindly `*(c2Capsule*)A` | UB |

## Why the `UB` rows are not differentially asserted

Rows 86–94 all reduce to one of three things the C standard leaves undefined,
and for which "C's answer" is not a value but *whatever the stack/heap
happened to contain*:

* **falling off a non-`void` function** (row 86) — gcc emits no `mov` into
  `%eax`/`%rax`, so the caller reads a leftover register. The value changes with
  the call that preceded it. The Rust translation returns `NULL` on that path,
  and `tests/errors.rs::row_86_ptr_from_parts_invalid_type_is_ub` documents
  that the *only* consumer (`omni_collide`) never dereferences it, which is
  why rows 5–8 still match exactly.
* **reading uninitialised storage** (rows 87, 89) — `c2Proxy pA;` /
  `int saveA[3]`.  gcc's stack frame and rustc's are laid out differently, so a
  byte-for-byte comparison would be asserting on garbage, not on behaviour.
* **NULL / wrong-type dereference** (rows 88, 91–94) — both libraries fault
  identically (`SIGSEGV`); `tests/errors.rs` asserts the *guarded* pointers
  (rows 31–37) instead, which is the part the C actually checks.

Every one of these is nevertheless *reachable only from inputs the C itself
never generates*: `c2Collided` filters out-of-range enums before any
dereference (rows 1–4), and `omni_collide` is the only public entry point in
`include/lib.h`.

## Row 46 in detail — the `iter < 20` bound is unreachable

`c2MakeProxy` builds proxies with at most **4** vertices (AABB; 2 for a capsule,
1 for a circle).  Combined with the `d1 > d0` monotonicity check (line 442) and
the duplicate-support check (line 466), GJK always terminates long before 20
passes.  Evidence, all in `tests/search_iter_cap.rs` (`--ignored`):

| search | samples | max `iter` found |
|--------|---------|------------------|
| uniform random shapes/transforms/caches | 400 000 | 4 |
| pool-driven mutation hill-climbing      | 800 000 | 7 |
| bit-level mutation + warm caches, 2 000 restarts | 6 000 000 | **7** |

`tests/errors.rs::row46_gjk_iteration_bound` therefore:

1. pins the exact `iter == 7` input the search found and asserts C and Rust
   agree on all five outputs there, and
2. sweeps 40 000 randomised inputs asserting `iterations` is *identical* in both
   and always inside `[0, 20]`.

The bound was confirmed to be exactly 7 by mutation testing the Rust source
(`while iter < 20`):

| mutant | caught by the suite? |
|--------|----------------------|
| `iter < 9` | no — unreachable |
| `iter < 8` | no — unreachable |
| `iter < 7` | **yes** (`row46_gjk_iteration_bound`) — cuts the last reachable pass |

So the suite is tight right up to the largest iteration count the library can
actually produce; the literal `20` is dead code in this configuration.

## Equivalent mutants (behaviourally unobservable, not test gaps)

`c2CircletoCapsule`: changing `if (da < 0)` (line 555) to `if (da <= 0)` is
**not** observable, and provably so:

* when `da == 0` and `c2Dot(n,n)` is non-zero, the projection branch computes
  `e = ap - n*(da/dot(n,n)) = ap - n*0 = ap`, so `d2 = dot(ap,ap)` — exactly
  what the `da < 0` branch computes;
* the only branch that yields a *different* `d2` is the `bp` branch, which
  requires `db >= 0`; but `db == dot(A.p-B.b, n) ≈ -|n|²`, so `db >= 0` forces
  `|n| == 0`, and then `B.a == B.b` makes `ap == bp` anyway.

Both implementations are therefore identical for every input on this pair of
branches; `rows63to68_c2CircletoCapsule_branches` covers all three branches
including both `da == 0` and `db == 0` boundaries.

## Mutation testing (suite sensitivity)

To prove the differential suite is not vacuously green, 23 deliberate bugs were
injected into `src/lib.rs`, one at a time, and the suite re-run:

| mutant | caught |
|--------|--------|
| `c2Dot`: swap the `fadd` operand order (NaN payload only) | yes (2 tests) |
| `c22`: `v <= 0` → `v < 0` | yes (8) |
| `c2Support`: `dot > dmax` → `dot >= dmax` (tie-breaking) | yes (9) |
| `c2Witness` `default:` → `(1,0)` | yes (2) |
| `c2MakeProxy` AABB `count = 4` → `3` | yes (9) |
| midpoint `0.5f` → `0.4999999f` | yes (8) |
| `FLT_EPSILON` constant perturbed | yes (1) |
| `c2Collided` AABB×CIRCLE argument order swapped | yes (1) |
| `ptr_from_parts` capsule `r = e` → `r = d` | yes (1) |
| `c2Det2`: `t1 - t2` → `t2 - t1` | yes (12) |
| `c2GJK`: `d1 > d0` → `d1 >= d0` | yes (2) |
| `c23` branch 6: swap the `b = a; a = c` order | yes (10) |
| `c2AABBtoAABB`: `<` → `<=` | yes (1) |
| `c2Clampv`: `Maxv`/`Minv` swapped | yes (2) |
| `c2MulrvT`: `-a.s` → `a.s` | yes (2) |
| cache predicate `-1.0e8f` → `-1.0e7f` | yes (1) † |
| cache predicate `-1.0e8f` → `-1.0e9f` | yes (1) † |
| cache predicate `max*2.0f` → `max*3.0f` | yes (1) † |
| cache predicate: drop the `!` | yes (2) |
| `cache_was_good`: `count != 0` → `count > 0` | yes (1) |
| `iter < 20` → `< 7` | yes (1) |
| `iter < 20` → `< 8` / `< 9` | no — **unreachable**, see above |
| `c2CircletoCapsule`: `da < 0` → `da <= 0` | no — **equivalent mutant**, see above |

† These three were **initially missed**.  `metric` is *recomputed* from the
cached simplex (it is `c2Det2` of the count-3 simplex, i.e. an area), so it only
reaches the `1e8` magnitude for shapes ~`1e4` units across, which the original
tests never built.  `rows40to41_cache_predicate_both_conjuncts` was added: it
replicates lib.c:378-401 using the C library's own primitives to classify each
input, sweeps the shape scale across `1.0 … 1.0e6`, and asserts all **four**
quadrants of `(min_metric < max_metric*2)` × `(metric < -1.0e8f)` are reached
(observed: `[[80, 96], [5462, 1874]]`).

## Empirical confirmation of the UB rows

The UB classifications above are not assumptions — each was probed against the
compiled C `.so` (gcc 11.5.0, `-O0`, small driver linked directly to
`libtranslated_rust.so`).  Results:

| row | probe | observed C behaviour | why it cannot be matched |
|-----|-------|----------------------|--------------------------|
| 86 | `ptr_from_parts(3, 9,9,9,9,9)` called 5× after a valid call | returns `0x41100000` every time — the **bit pattern of the float `9.0f`**, i.e. the leftover argument-register value; not a pointer at all | it is whatever gcc last left in `%rax` at that call site; Rust returns `NULL`. Neutralised because `c2Collided` never dereferences it (`row86_ptr_from_parts_ub_is_neutralised_by_c2Collided`) |
| 87 | `c2GJK(&circle, /*type=*/3, …)` after a call on two AABBs | returns `dist = +0.0`, `outA = (2.5, 3)` — visibly the **previous call's AABB proxy data** still on the stack | `c2Proxy pA;` is uninitialised; its contents are the previous call's frame. rustc's frame layout differs, so there is no value to agree on |
| 89 | `cache->count = 4` | **fatal signal** (exit 135) — `saveA[3]`/`saveB[3]` write past their 3-element bounds and smash the frame | crashes; nothing to compare |
| 90 | circle proxy (1 initialised vertex) with `cache->iA[0] = 5` | returned `(0,0)` in this probe (the stale bytes happened to be zero) — but a capsule proxy with `iA[0] = 3` **did diverge** in an early version of `row59_handcrafted_cache`: C `0.41897845` (`0x3ed68458`) vs Rust `0.42005455` (`0x3ed71164`) | the value read is stale stack, sometimes zero and sometimes not; rustc zero-initialises `c2Proxy`. `row59_handcrafted_cache` now constrains indices to `0..proxy_vert_count(type)` |

This is the one class of input where "the C implementation is always correct"
cannot be turned into an executable assertion: the C has no *behaviour* there,
only a leftover byte pattern. Every one of these rows is unreachable from the
public header (`omni_collide`), because `c2Collided` filters out-of-range enums
before any dereference and `omni_collide` never passes a cache.
