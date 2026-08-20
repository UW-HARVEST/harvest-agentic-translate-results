# ERRORS.md — Phase C error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. This library has **no error enum, no
`errno`, no asserts and no `RETURN_ERROR` macro**. Every way it "rejects" input is
one of:

* a `return` that leaves the output `c2Manifold` at `count = 0` (no contact),
* a `return 0` from a `static int` clipping predicate (`c2Clip`,
  `c2SidePlanes`, `c2SidePlanesFromPoly`) which the caller turns into "no contact",
* a `switch` with **no `default:` label**, so an unhandled `C2_TYPE` (including
  every out-of-range `int` the enum accepts across FFI) is silently ignored,
* a null-pointer test that substitutes a default instead of failing,
* a numeric degeneracy (`0/0`, `x/0`) that produces `NaN` / `±inf` and is then
  propagated instead of being rejected.

The "expected C result" column is what the *compiled C `.so`* does; each row's
differential test asserts the Rust `.so` produces the identical bytes / value.

Legend: `m` = the caller's `c2Manifold`. "untouched" = the C code never writes
that field, so it retains whatever the caller pre-loaded (tests pre-fill `m` with
a poison pattern and assert both libraries leave the same bytes).

## Status

All 96 rows are covered by a passing differential test. 95 rows compare C and Rust
directly; rows 1–2 (`ptr_from_parts` falling off the end of a non-`void` function) are
the single documented divergence, and are unobservable through any caller.

| where | tests |
|---|---|
| `tests/phase_c_errors.rs` | 34 tests, one or more per row |
| `tests/probe_uninit.rs` | rows 19–20, the genuine-UB characterisation |
| `tests/phase_d_symbols.rs` | row 27, the iteration-cap search |

Run with `cargo test --test phase_c_errors` (and `--release`).

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `ptr_from_parts` | `typ == C2_TYPE_POLY` (3) — `switch` has no `case`/`default` | falls off the end of a non-`void` function: returns an indeterminate register (observed: `0x7fd757fec39f`, a stale stack address). **Not a function of its inputs**; Rust returns `NULL`. Unobservable: `c2Collide` has no POLY case either, so the pointer is never dereferenced — which rows 5–9 verify byte-for-byte. | [x] |
| 2 | `ptr_from_parts` | `typ` out of range (4, 5, 255, `0x7fffffff`, `0xffffffff`) | same as row 1 | [x] |
| 3 | `c2MakeProxy` | `type == C2_TYPE_POLY` (3) | writes **nothing**; `*p` keeps the caller's bytes verbatim (`radius`, `count`, all 8 `verts`) | [x] |
| 4 | `c2MakeProxy` | `type` out of range (4 … `0xffffffff`) | writes nothing (row 3) | [x] |
| 5 | `c2Collide` | `typeA == C2_TYPE_POLY` (any `typeB`) | `m->count = 0`; `depths`, `contact_points`, `n` untouched | [x] |
| 6 | `c2Collide` | `typeB == C2_TYPE_POLY` (any valid `typeA`) | `m->count = 0`; rest untouched | [x] |
| 7 | `c2Collide` | `typeA` out of range (4 … `0xffffffff`) | `m->count = 0`; rest untouched | [x] |
| 8 | `c2Collide` | `typeB` out of range, `typeA` valid | `m->count = 0`; rest untouched | [x] |
| 9 | `omni_manifold` | `type_a` and/or `type_b` == POLY / out of range | `m->count = 0`; rest untouched (via rows 1 + 5–8) | [x] |
| 10 | `c2GJK` | `ax_ptr == NULL` | substitutes `c2xIdentity()` (`p=(0,0)`, `r=(c=1,s=0)`); no failure | [x] |
| 11 | `c2GJK` | `bx_ptr == NULL` | substitutes `c2xIdentity()`; no failure | [x] |
| 12 | `c2GJK` | `outA == NULL` | skips `*outA = a`; returns the distance normally | [x] |
| 13 | `c2GJK` | `outB == NULL` | skips `*outB = b` | [x] |
| 14 | `c2GJK` | `iterations == NULL` | skips `*iterations = iter` | [x] |
| 15 | `c2GJK` | `cache == NULL` | skips both the cache read and the cache write-back | [x] |
| 16 | `c2GJK` | `cache->count == 0` (`cache_was_good` false) | cache read skipped entirely, simplex re-seeded from vertex 0; cache still written back | [x] |
| 17 | `c2GJK` | `cache->count != 0` but metric test `!(min < max*2 && metric < -1.0e8f)` — true for essentially all inputs since `metric >= 0` for `count <= 2` | `cache_was_read = 1`, i.e. the cached simplex **is** reused (the `-1.0e8f` guard is dead in the original; reproduced verbatim) | [x] |
| 18 | `c2GJK` | `cache->count < 0` | `!!count` is true → the read loop body never runs, `s.count` set negative, `s.div` from cache; `c2GJKSimplexMetric` `default`→`case 1` returns 0; main loop `switch(s.count)` no match; `c2L` `default` → `(0,0)`; `c2Witness` `default` → `a=b=(0,0)`; returns `0` | [x] |
| 19 | `c2GJK` | `typeA`/`typeB == C2_TYPE_POLY` → proxy never written (row 3), so `pA`/`pB` is an *uninitialised stack local* in C | **Genuine UB — the C is not a function of its inputs here.** Measured: from a debug harness `pB.verts[0]` reads back a stack address (`0x00007f89_3affe180`), and from a fresh C driver / a release harness the garbage `pB.count` makes `c2Support` walk off the array and the process dies with SIGSEGV (exit 139). Rust zero-initialises, reproducing the virgin-zero-page case: a POLY operand acts as a point at the origin with radius 0. `tests/probe_uninit.rs` characterises the UB (running the crashing call in a child process); `common::zero_stack()` forces the C side into the zero state so the path is then compared byte-for-byte. | [x] |
| 20 | `c2GJK` | `typeA`/`typeB` out of range (4 …) | identical to row 19 (no `case` matches) | [x] |
| 21 | `c2GJK` | `use_radius != 0` and `dist <= rA + rB` (or `dist <= FLT_EPSILON`) | takes the `else`: `a = b = midpoint(a,b)`, `dist = 0` | [x] |
| 22 | `c2GJK` | `use_radius != 0`, radii shrink `a` and `b` onto the same point | `dist = 0` via `if (a.x==b.x && a.y==b.y)` | [x] |
| 23 | `c2GJK` | shapes overlap ⇒ `s.count == 3` | `hit = 1`, `a = b`, returns exactly `0` (radius block skipped even when `use_radius != 0`) | [x] |
| 24 | `c2GJK` | solver stalls: `d1 > d0` | `break` out of the iteration loop with the current simplex | [x] |
| 25 | `c2GJK` | search direction degenerates: `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` | `break` | [x] |
| 26 | `c2GJK` | new support vertex duplicates a saved one (`iA==saveA[i] && iB==saveB[i]`) | `break` **without** incrementing `s.count` | [x] |
| 27 | `c2GJK` | no termination condition ever fires | hard cap `while (iter < 20)`. `*iterations` is compared on **every** `c2GJK` call in the suite; a dedicated search (`tests/phase_d_symbols.rs::row27_iteration_cap_search`, ~108 K calls over all type pairs, transforms and hand-built caches) found the histogram `{0: 58101, 1: 33550, 2: 13455, 3: 2852, 4: 38, 5: 4}` — **max 5, so the cap is never reached** and the uninitialised `s.b.u`/`s.c.u` read is unreachable. Rust zero-initialises the simplex, matching the virgin-stack model, in case it ever were. | [x] |
| 28 | `c2GJK` | `s.div == 0` via a poisoned `cache->div` | `den = 1.0f/0.0f = +inf`; propagated, no rejection | [x] |
| 29 | `c2Support` | `count <= 0` (0, negative) | reads `verts[0]` **before** the loop guard, returns `0` | [x] |
| 30 | `c2Norms` | `count <= 0` | loop never runs, `norms` untouched | [x] |
| 31 | `c2Norms` | `count == 1` | `b = 0 == a`, `e = (0,0)`, `c2Norm((0,0))` = `(NaN, NaN)` | [x] |
| 32 | `c2Norm` | `a == (0,0)` (zero length) | `1.0f/0.0f = inf`; `0*inf = NaN` ⇒ `(NaN, NaN)`. No guard. | [x] |
| 33 | `c2Div` | `b == 0` | `1.0f/0.0f = +inf` (or `-inf` for `-0.0`) then multiplied through | [x] |
| 34 | `c2Intersect` | `da == db` (parallel / coincident) | `da/(da-db)` = `x/0` = `±inf`, or `NaN` when `da == db == 0` | [x] |
| 35 | `c2Clip` (static) | both distances `>= 0` and `d0*d1 > 0` ⇒ `sp == 0` | returns `0`; caller's `< 2` test rejects | [x] |
| 36 | `c2Clip` (static) | exactly one point behind the plane and `d0*d1 > 0` ⇒ `sp == 1` | returns `1` ⇒ caller's `< 2` test rejects | [x] |
| 37 | `c2Clip` (static) | `d0 == 0 && d1 == 0` | pushes **both** endpoints in the equality branch on top of anything already pushed | [x] |
| 38 | `c2Clip` (static) | `d0 < 0 && d1 < 0` and `d0*d1` underflows to `0` | pushes 3 entries into `c2v out[2]` — stack overwrite in C. Only `out[0..1]` is read back, so behaviour matches. | [x] |
| 39 | `c2SidePlanes` (static) | first `c2Clip(seg,left) < 2` | `return 0` | [x] |
| 40 | `c2SidePlanes` (static) | second `c2Clip(seg,right) < 2` | `return 0` | [x] |
| 41 | `c2SidePlanes` (static) | `h == NULL` | skips writing the reference plane, still `return 1` (never `NULL` from the two in-tree call sites) | [x] |
| 42 | `c2SidePlanes` (static) | `ra == rb` ⇒ `c2Norm((0,0))` = `(NaN,NaN)` | every `c2Dist` is `NaN`, no `< 0` test passes, `d0*d1 <= 0` false ⇒ `c2Clip` returns `0` ⇒ `return 0` | [x] |
| 43 | `c2AABBtoAABBManifold` | `dx < 0` (separated on x) | early `return`; `m->count = 0`, `depths`/`contact_points`/`n` untouched | [x] |
| 44 | `c2AABBtoAABBManifold` | `dy < 0` (separated on y) | early `return`; same as row 43 | [x] |
| 45 | `c2AABBtoAABBManifold` | any coordinate `NaN` ⇒ `dx`/`dy` `NaN` | both `< 0` tests false, `dx < dy` false ⇒ takes the **y** branch and reports a `NaN` depth with `count = 1` | [x] |
| 46 | `c2AABBtoAABBManifold` | inverted AABB (`min > max`) | `c2Absv` on the half-extent makes it positive again — *accepted*, no rejection | [x] |
| 47 | `c2CircletoCircleManifold` | `d2 >= r*r` (separated, incl. exact touch) | `m->count = 0`; `depths`/`contact_points`/`n` untouched | [x] |
| 48 | `c2CircletoCircleManifold` | concentric (`l == 0`) | fallback normal `(0, 1)`; `count = 1` | [x] |
| 49 | `c2CircletoCircleManifold` | negative radii summing `<= 0` | `d2 < r*r` compares against the *square*, so a negative `r` can still report contact — reproduced | [x] |
| 50 | `c2CircletoAABBManifold` | `d2 >= r2` (circle outside, incl. exact touch) | `m->count = 0`; rest untouched | [x] |
| 51 | `c2CircletoAABBManifold` | centre strictly inside the box (`d2 == 0`) | deep-contact branch: axis of least penetration, `depth = A.r + overlap` | [x] |
| 52 | `c2CircletoAABBManifold` | `d2 == 0` and `x_overlap == y_overlap` | `<` is false ⇒ **y** axis chosen | [x] |
| 53 | `c2CircletoCapsuleManifold` | GJK `d >= r` | `m->count = 0`; rest untouched | [x] |
| 54 | `c2CircletoCapsuleManifold` | `d == 0` (overlapping) | normal from `c2Norm(c2Skew(B.b - B.a))`; degenerate capsule (`B.a == B.b`) ⇒ `(NaN, NaN)` normal | [x] |
| 55 | `c2CapsuletoCapsuleManifold` | GJK `d >= r` | `m->count = 0`; rest untouched | [x] |
| 56 | `c2CapsuletoCapsuleManifold` | `d == 0` | normal from `c2Norm(c2Skew(A.b - A.a))`; degenerate `A` ⇒ `(NaN, NaN)` | [x] |
| 57 | `c2CapsuletoPolyManifold` | `1.0e-6f <= d` and `A.r <= d` | neither branch taken: `m->count = 0`, rest untouched | [x] |
| 58 | `c2CapsuletoPolyManifold` | `1.0e-6f <= d < A.r` (shallow) | single-point branch, `count = 1`, `n = normalize(b-a)` | [x] |
| 59 | `c2CapsuletoPolyManifold` | `code == 0` and `c2SidePlanesFromPoly` returns 0 | early `return`; `m->count` stays `0`, `n` untouched, and the `+= A.r` depth loop is skipped | [x] |
| 60 | `c2CapsuletoPolyManifold` | `code == 1` and `c2SidePlanes` returns 0 | early `return` (same as row 59) | [x] |
| 61 | `c2CapsuletoPolyManifold` | `code == 2` and `c2SidePlanes` returns 0 | early `return` (same as row 59) | [x] |
| 62 | `c2CapsuletoPolyManifold` | `B->count <= 0` (empty polygon) | face loop never runs, `index` stays `~0 == -1`, `sep` stays `-FLT_MAX`; `c2Support` returns 0 so `s0`/`s1` come from `verts[0]`; if `code` ends 0 then `verts[-1]` is read — out-of-bounds, 4 bytes before the struct. Reproduced by raw-offset indexing. | [x] |
| 63 | `c2CapsuletoPolyManifold` | every face distance is `NaN` | `d > sep` never true ⇒ `index` stays `-1` ⇒ `verts[-1]` read (row 62) | [x] |
| 64 | `c2CapsuletoPolyManifold` | `A.a == A.b` (degenerate capsule) | `c2Norm((0,0))` ⇒ `ab = (NaN,NaN)` ⇒ all planes `NaN` (row 63) | [x] |
| 65 | `c2CapsuletoPolyManifold` | `bx_ptr == NULL` | substitutes `c2xIdentity()` | [x] |
| 66 | `c2KeepDeep` (static) | neither clipped point has `d <= 0` | `m->count = 0` but `m->n = h.n` **is still written** — distinguishes it from the early-`return` rows | [x] |
| 67 | `c2AABBtoCapsuleManifold` | `c2CapsuletoPolyManifold` bails out early (rows 57, 59–61) | `m->n` is negated *anyway*: `m->n = c2Neg(m->n)` sign-flips the caller's pre-existing `n` | [x] |
| 68 | `c2AABBtoCapsuleManifold` | degenerate AABB (`min == max`) | `c2Norms` produces 4 `NaN` normals (row 31) ⇒ row 63 | [x] |
| 69 | `c2PlaneAt` | `i` out of range (`i >= 8`, `i < 0`) | unchecked `p->norms[i]` / `p->verts[i]`; reads past/before the arrays. Rust uses raw offsets so the same bytes are read. | [x] |
| 70 | `c22` | `v <= 0` | collapse to vertex `a`, `count = 1`, `div = 1` | [x] |
| 71 | `c22` | `u <= 0` (and `v > 0`) | collapse to vertex `b` (`s->a = s->b`), `count = 1`, `div = 1` | [x] |
| 72 | `c22` | `u > 0 && v > 0` | keep 2, `div = u + v` (can be `0` if both underflow) | [x] |
| 73 | `c23` | `vAB <= 0 && uCA <= 0` | vertex region A | [x] |
| 74 | `c23` | `uAB <= 0 && vBC <= 0` | vertex region B | [x] |
| 75 | `c23` | `uBC <= 0 && vCA <= 0` | vertex region C | [x] |
| 76 | `c23` | `uAB>0 && vAB>0 && wABC<=0` | edge AB | [x] |
| 77 | `c23` | `uBC>0 && vBC>0 && uABC<=0` | edge BC | [x] |
| 78 | `c23` | `uCA>0 && vCA>0 && vABC<=0` | edge CA | [x] |
| 79 | `c23` | all `NaN` (every comparison false) | falls through to the interior `else`, `count = 3`, `div = NaN` | [x] |
| 80 | `c2GJKSimplexMetric` | `count` not in {2,3} (0, 1, negative, > 3) | `default:` falls into `case 1:` ⇒ returns `0` | [x] |
| 81 | `c2D` | `count == 3` or any other value | `case 3: default:` ⇒ `(0, 0)` | [x] |
| 82 | `c2D` | `count == 2` and `c2Det2(ab, -a.p) <= 0` (incl. `NaN`) | `c2CCW90(ab)` rather than `c2Skew(ab)` | [x] |
| 83 | `c2Witness` | `count` not in {1,2,3} | `default:` ⇒ `*a = *b = (0, 0)` | [x] |
| 84 | `c2Witness` | `div == 0` | `den = +inf`; `count==1` ignores it, `count>=2` yields `inf`/`NaN` | [x] |
| 85 | `c2L` | `count == 3` or any other value | `default:` ⇒ `(0, 0)` (note: **differs** from `c2Witness`, which handles 3) | [x] |
| 86 | `c2L` | `div == 0` | `den = +inf`, propagated | [x] |
| 87 | `c2Maxv` / `c2Minv` | either component `NaN` | C's ternary is false for `NaN` ⇒ returns the **second** operand (`b`), unlike `f32::max`/`min` | [x] |
| 88 | `c2Absv` | `-0.0` component | `(-0.0 < 0)` is false ⇒ returns `-0.0` **unchanged**, unlike `f32::abs` | [x] |
| 89 | `c2Absv` | `NaN` component | `(NaN < 0)` false ⇒ returns the `NaN` with its sign bit **intact** | [x] |
| 90 | `c2Clampv` | `lo > hi` (inverted range) | no validation: `c2Maxv(lo, c2Minv(a, hi))` returns `lo` | [x] |
| 91 | `c2Clampv` | `NaN` in `a`, `lo` or `hi` | row 87 twice; result is whichever operand the ternaries select | [x] |
| 92 | `c2Dist` / `c2Dot` | `inf * 0` operand pair | `NaN`, propagated with no check | [x] |
| 93 | `c2Len` | `d2 == inf` / `d2 == NaN` | `sqrtf(inf) = inf`; `sqrtf(NaN) = NaN` | [x] |
| 94 | all `c2v` entry points | signalling `NaN` argument | quieted by the first arithmetic instruction; the **destination** operand's payload survives (see `src/fp.rs`) | [x] |
| 95 | `c2BBVerts` | inverted AABB (`min > max`) | no validation; emits the 4 corners in the same (now clockwise) order | [x] |
| 96 | `omni_manifold` / all `*Manifold` | `m == NULL` | C dereferences it → SIGSEGV. Identical UB in both; **not** tested (would abort the harness). | n/a |
