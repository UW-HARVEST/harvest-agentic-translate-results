# ERRORS.md — error / rejection surface table

The library has **no** error enum, no `RETURN_ERROR` macro, no `assert`, and no
`errno`. Rejection is expressed in exactly four ways:

* `m->count == 0` on the output `c2Manifold` (the "no collision" sentinel),
* an early bare `return;` out of a void manifold function (leaving `count == 0`),
* `return 0;` from a static predicate (`c2Clip` / `c2SidePlanes`),
* a `switch` with **no matching case and no `default:`**, which silently falls
  through and leaves the output at whatever the function pre-set
  (`c2Collide`, `c2MakeProxy`) or leaves the return value indeterminate
  (`ptr_from_parts`).

Every row below was derived from a specific line of `c_src/src/lib.c`.

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `c2Collide` | `typeA == C2_TYPE_POLY (3)` — no `case` in the outer `switch` | `m->count = 0`, rest of `*m` untouched; no read of `A`/`B` |
| 2 | `c2Collide` | `typeA` out of enum range (e.g. `4`, `99`, `-1`, `INT_MIN`) | `m->count = 0`, rest untouched |
| 3 | `c2Collide` | `typeA == CIRCLE`, `typeB == POLY (3)` — inner `switch` has no poly case | `m->count = 0` |
| 4 | `c2Collide` | `typeA == CIRCLE`, `typeB` out of range | `m->count = 0` |
| 5 | `c2Collide` | `typeA == AABB`, `typeB == POLY`/out of range | `m->count = 0` |
| 6 | `c2Collide` | `typeA == CAPSULE`, `typeB == POLY`/out of range | `m->count = 0` |
| 7 | `ptr_from_parts` | `typ == C2_TYPE_POLY (3)` or any out-of-range int — control falls off the end with no `return` | no allocation; return value indeterminate. Never dereferenced because `c2Collide` has no poly arm. Rust returns `NULL`. |
| 8 | `c2MakeProxy` | `type == C2_TYPE_POLY (3)` or out of range — `switch` has no such case | `*p` left completely unmodified (radius/count/verts keep prior contents) |
| 9 | `c2GJK` | `ax_ptr == NULL` (line 427 `if (!ax_ptr)`) | substitutes `c2xIdentity()`; must NOT crash |
| 10 | `c2GJK` | `bx_ptr == NULL` (line 431) | substitutes `c2xIdentity()` |
| 11 | `c2GJK` | `outA == NULL` / `outB == NULL` | writes skipped, distance still returned |
| 12 | `c2GJK` | `iterations == NULL` | write skipped |
| 13 | `c2GJK` | `cache == NULL` | no cache read, no cache write |
| 14 | `c2GJK` | `cache != NULL` with `cache->count == 0` (`cache_was_good` false) | cache ignored for input, still overwritten on output |
| 15 | `c2GJK` | `cache->count != 0` and `!(min_metric < max_metric*2 && metric < -1e8f)` (line 464 — always true for finite metrics, so the stale cache IS accepted) | simplex seeded from `cache->iA/iB` without validation |
| 16 | `c2GJK` | `cache->count != 0` with **out-of-range** `iA`/`iB` (e.g. 7, or `cache->count == 3` while the proxy has `count == 1`) | reads `pA.verts[iA]` out of bounds inside the 8-element proxy array — no check; garbage-but-deterministic result. Rust must reproduce the same read. |
| 17 | `c2GJK` | non-convergence: loop counter reaches `iter == 20` | loop exits on the `while (iter < 20)` bound, `hit == 0`, distance from last witness |
| 18 | `c2GJK` | duplicate support point (`iA==saveA[i] && iB==saveB[i]`) | `break` out of the loop before `++s.count` |
| 19 | `c2GJK` | `d1 > d0` (distance grew) | `break` |
| 20 | `c2GJK` | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (degenerate direction) | `break` |
| 21 | `c2GJK` | `use_radius != 0` and `dist <= rA+rB` **or** `dist <= FLT_EPSILON` | midpoint collapse: `a = b = (a+b)*0.5`, `dist = 0` |
| 22 | `c2GJK` | `use_radius != 0`, shrink makes `a == b` exactly | `dist = 0` |
| 23 | `c2GJK` | `hit == 1` (simplex reached count 3, i.e. origin enclosed) | `a = b`, `dist = 0`, radius branch skipped entirely |
| 24 | `c2GJKSimplexMetric` | `s->count == 1`, or `0`, or `4`+ (the shared `default:`/`case 1:` at line 174) | returns `0` |
| 25 | `c2D` | `s->count == 3` or any other value (`default:` line 403) | returns `c2V(0,0)` |
| 26 | `c2Witness` | `s->count` not in `{1,2,3}` (`default:` line 364) | writes `*a = *b = c2V(0,0)` |
| 27 | `c2L` | `s->count` not in `{1,2}` (`default:` line 417) | returns `c2V(0,0)` |
| 28 | `c2L` / `c2Witness` | `s->div == 0` | `den = 1/0 = +inf`, products become `inf`/`NaN` — no guard |
| 29 | `c2Support` | `count <= 0` | still reads `verts[0]` and returns `0` |
| 30 | `c2Norm` | zero-length input `c2V(0,0)` | `c2Div(a, 0)` → `1/0 = inf`, `0*inf = NaN` → `c2V(NaN, NaN)` |
| 31 | `c2Div` | `b == 0` | multiplies by `+inf` |
| 32 | `c2Clip` (static) | both endpoints on the positive side (`d0 >= 0 && d1 >= 0`, not both `0`) | `sp == 0`; **`seg[0]`/`seg[1]` are assigned from the uninitialized `out[]`** |
| 33 | `c2Clip` (static) | exactly one endpoint inside | `sp == 1` (or 2 after the `d0*d1 <= 0` intersect) |
| 34 | `c2Clip` (static) | `d0 == 0 && d1 == 0` | pushes both endpoints, `sp == 2` |
| 35 | `c2SidePlanes` (static) | `c2Clip(seg,left) < 2` (line 252) | `return 0` → caller bails |
| 36 | `c2SidePlanes` (static) | `c2Clip(seg,right) < 2` (line 254) | `return 0` → caller bails |
| 37 | `c2SidePlanes` (static) | `ra == rb` so `c2Norm(rb-ra)` is `NaN` | all `c2Dist` comparisons false → `c2Clip` returns 0 → `return 0` |
| 38 | `c2SidePlanes` (static) | `h == NULL` | plane output skipped, still `return 1` |
| 39 | `c2CapsuletoPolyManifold` | `code == 0` and `c2SidePlanesFromPoly` fails (line 781) | bare `return;` — `m->count` stays `0`, `m->n`/depths untouched |
| 40 | `c2CapsuletoPolyManifold` | `code == 1` and `c2SidePlanes` fails (line 790) | bare `return;`, `m->count == 0` |
| 41 | `c2CapsuletoPolyManifold` | `code == 2` and `c2SidePlanes` fails (line 798) | bare `return;`, `m->count == 0` |
| 42 | `c2CapsuletoPolyManifold` | `switch (code)` `default:` (line 802) — unreachable, `code ∈ {0,1,2}` | bare `return;` |
| 43 | `c2CapsuletoPolyManifold` | `d >= 1e-6f` **and** `d >= A.r` (both branch conditions false) | `m->count == 0`, no contact |
| 44 | `c2CapsuletoPolyManifold` | `B->count == 0` | plane loop never runs, `index` stays `~0 == -1`, `sep` stays `-FLT_MAX`; `s0`/`s1` from `c2Support(...,0,...)` win; out-of-range vert reads |
| 45 | `c2CapsuletoPolyManifold` | `A.a == A.b` (degenerate capsule) → `c2Norm(0,0)` = NaN | NaN propagates; `d > sep` false everywhere |
| 46 | `c2AABBtoAABBManifold` | `dx < 0` (line 665) | bare `return;`, `m->count == 0` |
| 47 | `c2AABBtoAABBManifold` | `dy < 0` (line 668) | bare `return;`, `m->count == 0` |
| 48 | `c2AABBtoAABBManifold` | inverted AABB (`min > max`) | `c2Absv` on the half-extents makes it behave like the normalized box |
| 49 | `c2CircletoCircleManifold` | `d2 >= r*r` (no overlap) | `m->count == 0` |
| 50 | `c2CircletoCircleManifold` | concentric circles, `l == 0` | normal forced to `c2V(0, 1)` |
| 51 | `c2CircletoAABBManifold` | `d2 >= r2` | `m->count == 0` |
| 52 | `c2CircletoAABBManifold` | circle centre strictly inside the box, `d2 == 0` | deep branch: axis of least overlap, `depth = A.r + overlap` |
| 53 | `c2CircletoAABBManifold` | `A.r == 0` (`r2 == 0`, so `d2 < 0` impossible) | `m->count == 0` always |
| 54 | `c2CircletoCapsuleManifold` | `d >= A.r + B.r` | `m->count == 0` |
| 55 | `c2CircletoCapsuleManifold` | `d == 0` and `B.a == B.b` | `c2Norm(c2Skew(0,0))` → `NaN` normal, `count == 1` |
| 56 | `c2CapsuletoCapsuleManifold` | `d >= A.r + B.r` | `m->count == 0` |
| 57 | `c2CapsuletoCapsuleManifold` | `d == 0` and `A.a == A.b` | `NaN` normal, `count == 1` |
| 58 | `c2AABBtoCapsuleManifold` | any early `return` inside `c2CapsuletoPolyManifold` | `m->count == 0` but `m->n` is still negated afterwards (`m->n = c2Neg(m->n)` runs unconditionally) — negating the *stale/zero* normal |
| 59 | `c2Norms` | `count == 0` | writes nothing |
| 60 | `c2Norms` | duplicate consecutive verts | `c2Norm` of zero vector → `NaN` normals |
| 61 | `c2PlaneAt` | `i < 0` or `i >= 8` | out-of-bounds read, no check |
| 62 | `omni_manifold` | `type_a`/`type_b == POLY` or out of range | `ptr_from_parts` returns without a value; `c2Collide` leaves `m->count = 0` |
| 63 | `omni_manifold` | `m == NULL` | immediate null deref in `c2Collide` (`m->count = 0`) — NOT tested (crashes both) |
| 64 | all float entry points | `NaN` / `+inf` / `-inf` coordinates | no validation anywhere; must propagate bit-identically |

Rows found during Phase C that were not visible from a first reading of the
source, added here so the table stays derived-from-the-C rather than
derived-from-what-passed:

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 65 | `c2Clip` (static) | `d0 < 0` **and** `d1 < 0` (so `sp == 2`) while `d0 * d1` **underflows to `+0.0`** (both distances subnormal-small), which also satisfies `d0 * d1 <= 0` | executes `out[sp++]` with `sp == 2`, i.e. writes 8 bytes past `c2v out[2]`. gcc `-O0` puts `out` at `-0x30(%rbp)` and the (already dead) `float d1` at `-0x1c(%rbp)`, so the stray store lands on `d1` and the only observable effect is that `c2Clip` returns **3**. `c2SidePlanes` then continues, because it only tests `< 2`. |
| 66 | `c2GJK` | `cache != NULL` with `cache->count >= 4` | `verts + i` writes past the 4-slot `c2Simplex`, clobbering `s.div`/`s.count`; the loop then writes further out on every iteration until the return address is destroyed. **Both libraries crash identically.** Documented, deliberately NOT executed. |
| 67 | `c2Incident` (static), `c2CapsuletoPolyManifold` | all candidate dot products are `NaN` (reachable with a non-finite AABB, whose `c2Norms` normals are `NaN`), so `index` keeps its initial `~0 == -1` | evaluates `p->verts[-1]`. `c2Poly` is `{int count; c2v verts[8]; c2v norms[8]}`, so `verts` is at offset 4 and `verts[-1]` spans offset −4..+4: **4 bytes before the struct**, then `count`. See "Reproduced UB" below. |

## Rows documented but deliberately not executed

| # | why |
|---|-----|
| 63 | `m == NULL` segfaults both libraries identically; running it would abort the test process. |
| 66 | `cache->count >= 4` destroys `c2GJK`'s own return address in both libraries. |
| 42 | the `default:` arm of `switch (code)` is unreachable — `code` is only ever assigned 0, 1 or 2. |
| 38 | `c2SidePlanes` with `h == NULL` is unreachable: both call sites pass `&h`. |

Everything else has a passing differential test in `tests/phase_c_errors.rs`;
each test name carries its row numbers and each assertion is tagged `ROWn`.

## Bugs found in the Rust translation and fixed

1. **`c2Clip` aborted instead of returning 3** (row 65). The Rust used a
   2-element `out` array and a checked index, so `out[sp]` with `sp == 2`
   panicked — and with `panic = "abort"` in `[profile.release]` that killed the
   process where the C merely returned 3. Fixed by giving `out` a third,
   write-only scratch slot, matching the dead `d1` slot the C store lands on.
   Found by `rows68_70_omni_manifold_all_pairs`.
2. **`c2GJK` bounds-checked the caller's cache indices** (row 16). `cache->iA` /
   `cache->iB` are completely unvalidated by the C, but the Rust indexed
   `pA.verts[iA as usize]` on a `[c2v; 8]`, so any index outside `0..8` aborted
   the process. Replaced with the unchecked `proxy_vert` helper. Found by
   auditing every runtime-indexed access against the C after fixing bug 1.
3. **`c2AABBtoCapsuleManifold` read the wrong bytes for `verts[-1]`** (row 67).
   See below.

## Reproduced UB: `p->verts[-1]` in `c2AABBtoCapsuleManifold`

The C builds a local `c2Poly p` and hands it to `c2CapsuletoPolyManifold`, which
can evaluate `p->verts[-1]` (row 67). In the gcc `-O0` frame, `p` is at
`-0xa0(%rbp)` and the by-value `c2AABB A` parameter is spilled to `-0xb0(%rbp)`,
which places `A.max.y` at `-0xa4(%rbp)` — exactly where `verts[-1].x` reads.
So the C deterministically reads

```
p->verts[-1] == (A.max.y, bitcast<float>(p.count) /* == 4 */)
```

The Rust reproduces this with a `#[repr(C)]` wrapper whose first field occupies
the 4 bytes preceding the `c2Poly`, initialised to `A.max.y`. Verified over
~100 000 randomized inputs including non-finite AABBs.

## Irreproducible UB: `c2GJK`'s uninitialized `c2Proxy`

`c2MakeProxy` has **no** `C2_TYPE_POLY` case, so for a poly it leaves the
caller's `c2Proxy` completely unwritten — and `c2CapsuletoPolyManifold` calls
`c2GJK(&A, C2_TYPE_CAPSULE, 0, B, C2_TYPE_POLY, ...)`. `c2GJK` declares
`c2Proxy pA, pB;` as plain locals, so the C then **reads uninitialized stack**.
This is on the live path of `c2AABBtoCapsuleManifold`, `c2Collide(AABB,CAPSULE)`
and `omni_manifold(AABB,CAPSULE)`.

Measured behaviour of the C library:

* stack region zeroed → returns exactly what an all-zero proxy produces;
* stack region dirty → returns garbage, and **segfaults** when the leftover
  `pB.count` is large (`c2Support` then walks off the array). Reproduced with a
  standalone C driver.

There is therefore no Rust implementation that can match the C for arbitrary
call histories. The Rust zero-initializes both proxies, which is the only
reproducible behaviour, and the differential tests establish that precondition
by zeroing the stack region before every call (`common::scrub_stack`).
`tests/phase_c_errors.rs::row64_non_finite_inputs` additionally proves that any
residual divergence on this path is the C's own inconsistency: it re-runs the
same shapes through `c2AABBtoCapsuleManifold` (no `malloc` between the scrub and
the read) and requires the C's two answers to differ before tolerating anything.

## NaN payload

x86 `addss` / `mulss` return the **destination** operand, quieted, when an
operand is NaN. In `c2Dot`'s `a.x*b.x + a.y*b.y`, gcc `-O0` makes the *y* term
the destination of the final `addss`; LLVM `-O3` makes the *x* term the
destination so it can return in `%xmm0`. Reordering the Rust source does not
change this — LLVM canonicalises commutative `fadd` operands (verified by
disassembly).

Consequence: when a caller passes in **two NaNs with different payloads**, a NaN
result can carry a different sign bit than the C's. Everything else is
bit-identical, including `±inf`, `±0.0`, subnormals and all internally generated
NaNs (x86 produces `0xffc00000` for every invalid operation, so both libraries
agree on those). The tests enforce exactly this: `feq` (strict bit equality)
everywhere except the NaN-input tests, which use `feq_nan` — strict bit equality
unless *both* values are NaN.

## Status

- [x] Every row above except 38, 42, 63 and 66 has a passing differential test.
- [x] Rows 38, 42, 63, 66 are unreachable or crash both libraries identically;
      each is justified in the table above.
- [x] All tests pass under `--default`, `--no-default-features` and
      `--all-features` (see `verify.sh`).
