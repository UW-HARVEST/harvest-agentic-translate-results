# ERRORS.md — error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. The C library has **no error enum,
no `RETURN_ERROR` macro, no `assert`, no `errno` use and no `return -1` /
`return NULL`** (verified by grep, see below). Its entire rejection surface
consists of:

* `return 0` sentinels (7 sites),
* null-pointer guards (`if (!ptr)`),
* `switch` arms with no matching `case` (invalid `C2_TYPE` enum values),
* `default:` arms of `switch (s->count)` (out-of-range simplex counts),
* numeric guard/early-exit predicates (`FLT_EPSILON`, `FLT_MAX`, `-1.0e8f`,
  iteration cap `20`),
* division by zero / degenerate geometry producing `inf` / `NaN`.

Grep evidence:

```
$ grep -n 'return 0\|return -1\|return NULL\|RETURN_ERROR\|assert\|errno\|ERROR' src/lib.c
164, 529, 535, 587, 599, 611, 615   (all `return 0`)
$ grep -n 'if (!' src/lib.c
368 (!ax_ptr), 372 (!bx_ptr), 405 (!(metric guard)), 409 (!cache_was_read)
```

Sentinel convention for this library: **`int` returns use `0` = "no
collision / rejected", `1` = accepted**; **`float` returns use `0.0f` =
"touching / rejected"**; **`void` functions reject by writing nothing.**

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| 1 | `c2Collided` | `typeA == C2_TYPE_CIRCLE` (0), `typeB` not in {0,1,2} (e.g. 3, -1, 99, `INT_MIN`, `INT_MAX`) — `default:` at line 587 | returns `0` | [x] |
| 2 | `c2Collided` | `typeA == C2_TYPE_AABB` (1), `typeB` not in {0,1,2} — `default:` at line 599 | returns `0` | [x] |
| 3 | `c2Collided` | `typeA == C2_TYPE_CAPSULE` (2), `typeB` not in {0,1,2} — `default:` at line 611 | returns `0` | [x] |
| 4 | `c2Collided` | `typeA` not in {0,1,2} (any `typeB`, incl. invalid) — outer `default:` at line 615 | returns `0` (does **not** deref `A`/`B`) | [x] |
| 5 | `c2MakeProxy` | `type` not in {0,1,2} — `switch` has no `default:`, so `*p` is left **completely unwritten** (radius/count/verts all untouched) | `*p` unchanged | [x] |
| 6 | `c2GJKSimplexMetric` | `s->count == 1` — `case 1:` | returns `0.0f` | [x] |
| 7 | `c2GJKSimplexMetric` | `s->count` outside {1,2,3} (0, -1, 4, 99, `INT_MIN`) — `default:` falls into `case 1:` | returns `0.0f` | [x] |
| 8 | `c2Witness` | `s->count` outside {1,2,3} — `default:` | writes `*a = (0,0)`, `*b = (0,0)` | [x] |
| 9 | `c2Witness` | `s->div == 0.0f` (den = `1/0` = `+inf`) with `count` 2 or 3 | `inf`/`NaN` components, bit-identical in both | [x] |
| 10 | `c2Witness` | `s->div == -0.0f` (den = `-inf`) | `-inf`/`NaN`, bit-identical | [x] |
| 11 | `c2L` | `s->count` outside {1,2} (0, 3, -1, 99) — `default:` | returns `(0,0)` | [x] |
| 12 | `c2L` | `s->div == 0` with `count == 2` | `inf`/`NaN`, bit-identical | [x] |
| 13 | `c2D` | `s->count == 3` or outside {1,2,3} — `case 3:`/`default:` | returns `(0,0)` | [x] |
| 14 | `c2Support` | `count <= 0` (0, -1, `INT_MIN`) — loop body never runs, but `verts[0]` **is** still read | returns `0` | [x] |
| 15 | `c2Support` | all dots equal / `NaN` dots (`dot > dmax` always false) | returns `0` (first index wins) | [x] |
| 16 | `c2Div` | `b == 0.0f` → `1.0f/0.0f` = `+inf`, times `0` component = `NaN` | `inf`/`NaN` per component, bit-identical | [x] |
| 17 | `c2Div` | `b == -0.0f` → `-inf` | `-inf`/`NaN`, bit-identical | [x] |
| 18 | `c2Norm` | zero vector `(0,0)` → `c2Len` = 0 → division by zero | `(NaN, NaN)` | [x] |
| 19 | `c2Norm` | non-finite input (`inf`, `NaN` components) | `NaN`s, bit-identical | [x] |
| 20 | `c2Len` | components `inf`/`NaN` → `sqrtf` of `inf`/`NaN` | `inf`/`NaN`, bit-identical | [x] |
| 21 | `c2Len` | `c2Dot(a,a)` overflows to `+inf` (e.g. `x = FLT_MAX`) | `+inf` | [x] |
| 22 | `c2GJK` | `ax_ptr == NULL` — guard at line 368 | uses `c2xIdentity()` instead of dereferencing | [x] |
| 23 | `c2GJK` | `bx_ptr == NULL` — guard at line 372 | uses `c2xIdentity()` | [x] |
| 24 | `c2GJK` | `outA == NULL` | no write; return value still valid | [x] |
| 25 | `c2GJK` | `outB == NULL` | no write | [x] |
| 26 | `c2GJK` | `iterations == NULL` | no write | [x] |
| 27 | `c2GJK` | `cache == NULL` | cache block skipped entirely, cold start | [x] |
| 28 | `c2GJK` | `cache != NULL` **and** `cache->count == 0` (`cache_was_good` false) | cache **not** read, cold start; cache still written on exit | [x] |
| 29 | `c2GJK` | `cache != NULL`, `cache->count != 0`, and the metric guard at line 405 is satisfied (`min_metric < max_metric*2 && metric < -1.0e8f`) — e.g. `cache->metric = -1.0e30f` with a warm 3-simplex whose metric is very negative | `cache_was_read` stays 0 → simplex is **discarded** and reset to a cold 1-simplex | [x] |
| 30 | `c2GJK` | `cache->metric = NaN` (guard comparisons all false) | `cache_was_read = 1`, warm start | [x] |
| 31 | `c2GJK` | early exit `d1 > d0` (no progress) | `break` before adding a vertex; `iter` reflects the truncated count | [x] |
| 32 | `c2GJK` | early exit `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (search direction degenerate, e.g. coincident shapes) | `break`, `dist` from current witness | [x] |
| 33 | `c2GJK` | early exit: new support point duplicates a saved one (`dup`) | `break` **without** `++s.count` (the written `verts[s.count]` is discarded) | [x] |
| 34 | `c2GJK` | iteration cap: loop can never exceed `iter == 20`. The cap itself is **unreachable**: the widest proxy has 4 vertices, so the support point repeats (row #33) or the simplex closes (row #38) first — measured maximum `iter` is 3. The test asserts the `0..=20` bound on every call and asserts the cap stays unreachable. | `*iterations ∈ [0, 20]` | [x] |
| 35 | `c2GJK` | `use_radius != 0` **and** `dist <= rA + rB` (overlapping within radii) | `a = b = midpoint`, returns `0.0f` | [x] |
| 36 | `c2GJK` | `use_radius != 0` **and** `dist <= FLT_EPSILON` (witness points coincide) | `a = b = midpoint`, returns `0.0f` | [x] |
| 37 | `c2GJK` | `use_radius != 0`, shrink makes `a == b` exactly. Reached deterministically by a zero-extent AABB at `x = 1e7` (so `rA = 0`) against a circle one float-step away with `r = 0.5`: `dist == 1.0 > rA+rB`, but `b - n*0.5 == 1e7 + 0.5` ties-to-even back to `1e7 == a`. | `dist` forced to `0.0f` | [x] |
| 38 | `c2GJK` | `hit != 0` (3-simplex containing origin) | `a = b`, returns `0.0f`; `use_radius` branch skipped | [x] |
| 39 | `c2GJK` | negative radius shape (`c2Circle.r < 0`, `c2Capsule.r < 0`) — never validated | no rejection; `dist -= rA+rB` grows the distance | [x] |
| 40 | `c2GJK` | `NaN`/`inf` coordinates in the input shapes | no rejection; propagates, `iter` may differ from finite case | [x] |
| 41 | `c2AABBtoCapsule` | `c2GJK(...) != 0.0f` — `return 0` at line 529. NB the wrapper passes `use_radius = 1`, and in that mode a `NaN` distance always fails `dist > rA+rB` and takes the midpoint arm, so `dist` is forced to `0.0f`: **`c2GJK` can never return `NaN` when `use_radius != 0`** (verified empirically over 200 000 non-finite inputs — 0 occurrences — while `use_radius = 0` produced 20 563). The `NaN != 0` path is therefore unreachable from this wrapper; the test asserts the invariant instead. | returns `0` iff `dist != 0.0f` | [x] |
| 42 | `c2CapsuletoCapsule` | `c2GJK(...) != 0.0f` — `return 0` at line 535 (same `use_radius = 1` remark as #41) | returns `0` iff `dist != 0.0f` | [x] |
| 43 | `c2AABBtoAABB` | inverted/degenerate AABB (`min > max`) — never validated | no rejection; separation test evaluated as written | [x] |
| 44 | `c2AABBtoAABB` | `NaN` bounds — all four `<` comparisons false | returns `1` (reports collision) | [x] |
| 45 | `c2CircletoCircle` | negative radii such that `A.r + B.r < 0` → `r2 = (A.r+B.r)^2 > 0` | still compares `d2 < r2`; no rejection | [x] |
| 46 | `c2CircletoCircle` / `c2CircletoAABB` / `c2CircletoCapsule` | `NaN` inputs → `d2 < r2` is false | returns `0` | [x] |
| 47 | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` → `n = (0,0)`, `da = 0` (not `< 0`), `db = 0` (not `< 0`) → takes the `bp` branch (**no** division by `c2Dot(n,n)`) | distance to `B.b`; no `NaN` | [x] |
| 48 | `c2CircletoCapsule` | `da >= 0 && db < 0` with `c2Dot(n,n) == 0` — unreachable for a degenerate capsule (see #47), but reachable with `NaN`/`inf` coords → `0/0` = `NaN` | `NaN` propagates, `d2 < r*r` false → `0` | [x] |
| 49 | `c22` | degenerate `a == b` → `u = 0`, `v = 0`; `v <= 0` wins first | `count = 1`, `div = 1`, keeps vertex `a` | [x] |
| 50 | `c23` | degenerate: collinear/coincident points → `area = 0` → `uABC = vABC = wABC = 0`; earlier arms are checked first | whichever arm the C evaluates first, bit-identical | [x] |
| 51 | `c23` | final `else` arm with a degenerate `div`, which `c2GJK` then feeds straight into `c2Witness` as `1.0f/div`. Measured over 800 000 solver calls (tiny/huge/arbitrary coordinates): `div == 0` **never** occurs, but `div == NaN` (199 320) and `div == ±inf` (346 982) both do. The `div == 0` case is still covered directly against `c2Witness` in rows #9/#10. | `inf`/`NaN` witness, bit-identical | [x] |
| 52 | `c2BBVerts` | inverted AABB (`min > max`) | no rejection; writes exactly 4 verts as spelled | [x] |
| 53 | `reverse_collide` | `NaN` / `inf` / negative `r` — no validation at all | bitmask computed from the three `c2Collided` calls | [x] |

## Deliberately excluded (undefined behaviour in C — not differentially testable)

| function | trigger | why excluded |
|----------|---------|--------------|
| `c2GJK` | `typeA`/`typeB` not in {0,1,2} | `c2MakeProxy` writes nothing, so the C reads its **uninitialised** stack `c2Proxy` (`pA.count`, `pA.verts[0]`). The result depends on leftover stack bytes and is not a defined value; Rust uses a zeroed `c2Proxy`. Verified empirically in `tests/errors.rs::gjk_invalid_type_is_ub_documented` (documented, not asserted). |
| `c2GJK` | `cache->count > 3` or `< 0`, or `cache->iA/iB[i]` outside `[0, pX.count)` | writes/reads past the `c2Simplex`/`c2Proxy` arrays — out-of-bounds access in C. Only `cache->count ∈ {0,1,2,3}` with in-range indices is exercised. |
| `c2BBVerts`, `c2Support`, `c2Witness`, `c2MakeProxy`, `c22`, `c23`, `c2D`, `c2L`, `c2GJKSimplexMetric` | `NULL` pointer arguments | the C dereferences unconditionally (no guard) → segfault in both. Not a defined rejection. Only `c2GJK`'s six documented null-guarded parameters (#22–#27) are tested. |

## Where each row is tested

All rows live in `tests/errors.rs`:

| rows | test |
|------|------|
| 1–4 | `rows1_4_collided_invalid_enum` |
| 5 | `row5_makeproxy_invalid_enum` |
| 6–7 | `rows6_7_metric_out_of_range_count` |
| 8–10 | `rows8_10_witness_rejections` |
| 11–13 | `rows11_13_c2l_c2d_rejections` |
| 14–15 | `rows14_15_support_rejections` |
| 16–21 | `rows16_21_division_and_length_degeneracies` |
| 22–28 | `rows22_28_gjk_null_guards` |
| 29–30 | `rows29_30_cache_metric_guard` |
| 31–40 | `rows31_38_gjk_exits_and_radius_arms` |
| 41–42 | `rows41_42_gjk_wrapper_sentinels` |
| 43–48, 52 | `rows43_52_predicate_degeneracies` |
| 49–51 | `rows49_51_solver_degeneracies` |
| 53 | `row53_reverse_collide_unvalidated` |
| excluded (UB) | `gjk_invalid_type_is_ub_documented` |

## Out-of-range enum values crossing the FFI boundary

A C `enum` is just an `int`, so a value with no valid variant is a real input.
Rows 1–5 are driven with the full set

```
3, 4, 5, 99, 255, 256, 1000, -1, -2, -99,
INT_MIN, INT_MIN+1, INT_MAX, INT_MAX-1, 0x01000000, -0x01000000, 0x7ffffffe
```

for `typeA`, for `typeB`, and for both at once. Row 4 additionally passes
**NULL for both shape pointers**, which proves the outer `default:` arm returns
`0` without dereferencing them — and that the Rust wrapper does the same rather
than reading through a null pointer before dispatching.

## Sentinel agreement, not just "both failed"

Every row asserts the *same* value, and where the C has a documented sentinel the
test also pins the sentinel itself:

* `c2Collided` invalid enum → `assert_eq!(cv, 0)` as well as `C == Rust`.
* `c2GJKSimplexMetric` out-of-range count → `+0.0f` **by bit pattern**.
* `c2Witness` `default:` → both witnesses exactly `(0, 0)`.
* `c2L` / `c2D` out-of-range count → exactly `(0, 0)`.
* `c2Support` `count <= 0` and all-ties → exactly index `0`.
* `c2MakeProxy` unknown type → the caller's `c2Proxy` is byte-identical to the
  sentinel it was pre-filled with (nothing written at all).
* `c2AABBtoAABB` with all-NaN bounds → exactly `1`.
* `c2AABBtoCapsule` / `c2CapsuletoCapsule` → exactly `dist != 0.0f`, cross-checked
  against the raw `c2GJK` distance from *both* libraries.
* `c2GJK` NULL-transform → output byte-identical to the explicit-identity call.
* `c2GJK` `*iterations` → always within `0..=20`.
