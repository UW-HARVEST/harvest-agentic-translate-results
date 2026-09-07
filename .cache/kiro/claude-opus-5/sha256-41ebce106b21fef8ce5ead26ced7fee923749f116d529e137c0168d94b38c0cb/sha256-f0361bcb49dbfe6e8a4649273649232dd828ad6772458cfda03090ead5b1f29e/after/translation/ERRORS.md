# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. The library has **no** `assert`,
**no** error enum, and **no** null-pointer validation. Every rejection is a
`return 0` (or a sentinel float produced by unguarded division). Each `return 0`
branch and each unguarded arithmetic hazard gets one row.

Grep basis:

```sh
grep -n 'return 0\|return 1\|return -\|assert\|NULL\|!(' c_src/src/lib.c
grep -n '1.0f /\|/ d\|/ den\|sqrtf' c_src/src/lib.c
```

Sentinel convention: predicate/raycast functions return `int` 0 = reject,
1 = accept. `out` is written only on some paths (see notes).

| #  | function | trigger (the exact invalid input/condition) | expected C result | done |
|----|----------|----------------------------------------------|-------------------|------|
| 1  | `c2RaytoCircle` | `disc = b*b - c < 0` (ray line misses circle) | returns `0`, `*out` untouched | [x] |
| 2  | `c2RaytoCircle` | `t = -b - sqrtf(disc) < 0` (circle fully behind ray origin) | returns `0`, `*out` untouched | [x] |
| 3  | `c2RaytoCircle` | `t > A.t` (hit beyond ray length) | returns `0`, `*out` untouched | [x] |
| 4  | `c2RaytoCircle` | `B.r == 0` (degenerate circle) | `c = dot(m,m)`, almost always `disc < 0` → `0` | [x] |
| 5  | `c2RaytoCircle` | `B.r < 0` (negative radius; `B.r*B.r` still positive) | same as `+B.r`; hit normal from `c2Norm` | [x] |
| 6  | `c2RaytoCircle` | `A.t < 0` (negative ray length) | `t <= A.t` false for `t>=0` → `0` | [x] |
| 7  | `c2RaytoCircle` | `A.p == B.p` and `B.r > 0` (origin at center) → `c2Norm(0)` | returns `1` with `out->n = {NaN, NaN}` | [x] |
| 8  | `c2RaytoCircle` | any NaN component in `A`/`B` | `disc < 0` false, `t>=0` false → `0` | [x] |
| 9  | `c2AABBtoAABB` | `B.max.x < A.min.x` (d0 separating) | returns `0` | [x] |
| 10 | `c2AABBtoAABB` | `A.max.x < B.min.x` (d1 separating) | returns `0` | [x] |
| 11 | `c2AABBtoAABB` | `B.max.y < A.min.y` (d2 separating) | returns `0` | [x] |
| 12 | `c2AABBtoAABB` | `A.max.y < B.min.y` (d3 separating) | returns `0` | [x] |
| 13 | `c2AABBtoAABB` | inverted box (`min > max`) — no validation | evaluated literally, usually `0` | [x] |
| 14 | `c2AABBtoAABB` | NaN in any coordinate (all `<` false) | returns `1` (NaN compares false ⇒ "not separated") | [x] |
| 15 | `c2RayToPlane_OneDimensional` (static, via `c2RaytoAABB`) | `da < 0` | returns `0.0f` | [x] |
| 16 | `c2RayToPlane_OneDimensional` | `da*db > 0` (both endpoints same side) | returns `1.0f` | [x] |
| 17 | `c2RayToPlane_OneDimensional` | `da == db` so `d = da-db == 0` | returns `0.0f` (no divide) | [x] |
| 18 | `c2RaytoAABB` | swept-ray AABB does not overlap `B` (`!c2AABBtoAABB`) | returns `0`, `*out` untouched | [x] |
| 19 | `c2RaytoAABB` | SAT on ray normal: `d = \|dot(n, p0-center)\| - dot(abs_n, half) > 0` | returns `0`, `*out` untouched | [x] |
| 20 | `c2RaytoAABB` | `hit0\|hit1\|hit2\|hit3 == 0` (every `t_i > 1`) | returns `0`, `*out` untouched | [x] |
| 21 | `c2RaytoAABB` | `A.t == 0` (zero-length ray, `p0 == p1`, `n == 0`) | `d = -dot(0,half) <= 0`; result from `t_i` chain | [x] |
| 22 | `c2RaytoAABB` | `A.t < 0` (negative length: `p1` behind origin) | evaluated literally; `out->t = t_i * A.t` can be negative | [x] |
| 23 | `c2RaytoAABB` | `A.d == {0,0}` (zero direction) | `p1 == p0`; same degenerate path as row 21 | [x] |
| 24 | `c2RaytoAABB` | `B` inverted (`min > max`) → negative half extents | no validation; usually rejected at row 18 | [x] |
| 25 | `c2RaytoAABB` | NaN in `A.p` / `A.d` / `A.t` / `B` | `c2AABBtoAABB` returns `1` (row 14), `d > 0` false, all `t_i <= 1` false → `0` | [x] |
| 26 | `c2AABBtoPoint` | `B.x < A.min.x` | returns `0` | [x] |
| 27 | `c2AABBtoPoint` | `B.y < A.min.y` | returns `0` | [x] |
| 28 | `c2AABBtoPoint` | `B.x > A.max.x` | returns `0` | [x] |
| 29 | `c2AABBtoPoint` | `B.y > A.max.y` | returns `0` | [x] |
| 30 | `c2AABBtoPoint` | NaN in `A` or `B` | returns `1` (all four `<`/`>` false) | [x] |
| 31 | `c2CircleToPoint` | `dot(n,n) >= A.r*A.r` incl. exactly on the boundary (`<` is strict) | returns `0` | [x] |
| 32 | `c2CircleToPoint` | `A.r == 0` | `d2 < 0` impossible → `0` even when point == center | [x] |
| 33 | `c2CircleToPoint` | `A.r < 0` | `A.r*A.r > 0` ⇒ behaves like `+A.r` | [x] |
| 34 | `c2CircleToPoint` | NaN component | `d2 < r*r` false → `0` | [x] |
| 35 | `c2RaytoCapsule` | final fall-through: `!(yAe.x*yAp.x < 0)` and `min(\|yAe.x\|,\|yAp.x\|) >= B.r` | returns `0`, but `out->n`/`out->t` **were already written** (`c2Norm(cap_n)`, `0`) | [x] |
| 36 | `c2RaytoCapsule` | `B.a == B.b` (zero-length axis) → `c2Norm({0,0})` | `M` becomes NaN; `out->n = {NaN,NaN}`, `out->t = 0`; returns `0` unless `c2CircleToPoint` accepts | [x] |
| 37 | `c2RaytoCapsule` | `B.r == 0` | `capsule_bb = {{0,0},{0,yBb.y}}`; `c2CircleToPoint` always `0`; `min(...) < 0` false → `0` | [x] |
| 38 | `c2RaytoCapsule` | `B.r < 0` | `capsule_bb.min.x = -B.r > capsule_bb.max.x` (inverted); no validation | [x] |
| 39 | `c2RaytoCapsule` | delegated reject: `\|yAp.x\| < B.r`, `yAp.y < 0`, `c2RaytoCircle(A,Ca)` misses | returns whatever `c2RaytoCircle` returns (`0`) | [x] |
| 40 | `c2RaytoCapsule` | delegated reject: `\|yAp.x\| < B.r`, `yAp.y >= 0`, `c2RaytoCircle(A,Cb)` misses | returns `0` | [x] |
| 41 | `c2RaytoCapsule` | side-slab path, `y <= 0`, `c2RaytoCircle(A,Ca)` misses | returns `0` | [x] |
| 42 | `c2RaytoCapsule` | side-slab path, `y >= yBb.y`, `c2RaytoCircle(A,Cb)` misses | returns `0` | [x] |
| 43 | `c2RaytoCapsule` | side-slab path with `d = yAe.x - yAp.x == 0` → `t = (c-yAp.x)/0` | ±Inf or NaN `t`; comparisons follow IEEE; no check | [x] |
| 44 | `c2RaytoCapsule` | `A.t == 0` (`yAe == yAp`) | row 43's `d == 0` degenerate | [x] |
| 45 | `c2RaytoCapsule` | NaN in `A` or `B` | all comparisons false → `0` with `out` partially written | [x] |
| 46 | `c2RaytoPoly` | `den == 0 && num < 0` (ray parallel to a plane, origin outside it) | returns `0`, `*out` untouched | [x] |
| 47 | `c2RaytoPoly` | `hi < lo` after clipping (slabs disjoint) | returns `0`, `*out` untouched | [x] |
| 48 | `c2RaytoPoly` | loop completes with `index == ~0` (no back-facing plane clipped `lo`) | returns `0`, `*out` untouched | [x] |
| 49 | `c2RaytoPoly` | `B->count == 0` (empty polygon) | loop body never runs, `index == ~0` → `0` | [x] |
| 50 | `c2RaytoPoly` | `B->count < 0` (negative count) | `i < count` false immediately → `0` | [x] |
| 51 | `c2RaytoPoly` | `B->count > 8` (overruns `verts[8]`/`norms[8]`) | C reads adjacent memory — no bounds check; Rust must read the same bytes via raw pointer, not panic | [x] |
| 52 | `c2RaytoPoly` | `bx_ptr == NULL` | **not** an error: substitutes `c2xIdentity()` | [x] |
| 53 | `c2RaytoPoly` | `A.t < 0` ⇒ `hi = A.t < lo = 0` on first iteration | `hi < lo` → `0` (row 47 special case) | [x] |
| 54 | `c2RaytoPoly` | `A.t == 0` and a `den > 0` plane with `num < 0` | `hi` becomes `num/den < 0 = lo` → `0` | [x] |
| 55 | `c2RaytoPoly` | zero normals `B->norms[i] == {0,0}` ⇒ `num == 0, den == 0` | `den==0 && num<0` false → plane ignored | [x] |
| 56 | `c2RaytoPoly` | NaN in `A`, `B->verts`, `B->norms`, or `bx` | every `<`/`==` false ⇒ plane ignored; `index` stays `~0` → `0` | [x] |
| 57 | `c2RaytoPoly` | non-normalized `bx.r` (`c*c + s*s != 1`) | no validation; transform applied literally | [x] |
| 58 | `c2CastRay` | `typeB` outside `0..3` (e.g. `4`, `-1`, `99`, `INT_MAX`, `INT_MIN`) — C enums accept any `int` | falls off `switch` → returns `0`, `*out` untouched | [x] |
| 59 | `c2Div` | `b == 0` | `1.0f/0 = ±Inf`; components become `±Inf` or `NaN` (from `0*Inf`) | [x] |
| 60 | `c2Div` | `b == NaN` or `b == ±Inf` | `1.0f/b` propagates; `Inf` ⇒ result `±0`/`NaN` | [x] |
| 61 | `c2Norm` | `a == {0,0}` ⇒ `c2Len == 0` ⇒ divide by zero | `{NaN, NaN}` (`0 * Inf`) | [x] |
| 62 | `c2Norm` | `a` containing `±Inf` | `c2Len == Inf` ⇒ `{NaN, NaN}` or `{±0,±0}` | [x] |
| 63 | `c2Len` | `dot(a,a)` overflows to `+Inf` | `sqrtf(Inf) == Inf` | [x] |
| 64 | `c2Len` | NaN component | `sqrtf(NaN) == NaN` | [x] |
| 65 | `c2Absv` / min / max | `-0.0` input: C uses `x < 0 ? -x : x`, so `-0.0` stays `-0.0` (unlike `fabsf`) | sign bit preserved — Rust must **not** use `f32::abs` | [x] |
| 66 | `c2Minv` / `c2Maxv` | NaN operand: C ternary `a < b ? a : b` returns `b` on NaN (unlike `fminf`) | Rust must **not** use `f32::min`/`f32::max` | [x] |

## Not testable (identical UB on both sides, would abort the harness)

| function | trigger | reason not tested |
|----------|---------|-------------------|
| `c2RaytoCircle`, `c2RaytoAABB`, `c2RaytoCapsule`, `c2RaytoPoly`, `c2CastRay`, `poly_ray` | `out == NULL` | C dereferences `out` with no check ⇒ SIGSEGV in both. Verified by inspection only. |
| `c2RaytoPoly`, `c2CastRay` | `B == NULL` | unconditional `*B` deref ⇒ SIGSEGV in both. |

## Phase C results

All 66 rows are covered by `translation/tests/phase_c_errors.rs` (55 test
functions; a few adjacent rows that share one construction are asserted in one
test — rows 9–12, 15–17, 26–29, 39–40, 41–42, 59–60, 61–62, 63–64). Each test
builds the exact invalid input, calls BOTH `.so`s, and compares the sentinel
**and** the full `out` struct bit-for-bit against the `DIRTY` prefill.

Several rows additionally pin the C's sentinel with a hard `assert_eq!`, so the
test fails loudly if the C reference ever changes rather than silently agreeing
with a wrong Rust:

- row 14: `c2AABBtoAABB` with all-NaN coordinates returns **1** (all four `<`
  are false, so C reports "not separated").
- row 30: `c2AABBtoPoint` with a NaN point returns **1**.
- row 31: `c2CircleToPoint` with the point exactly on the rim returns **0**
  (`<` is strict).
- row 32: `c2CircleToPoint` with `r == 0` rejects even the centre.
- row 35: `c2RaytoCapsule`'s fall-through returns **0** but has already written
  `out->n` and `out->t` — asserted with `assert_ne!(out, DIRTY)`.
- row 49: `c2RaytoPoly` with `count == 0` returns 0 and leaves `out` == `DIRTY`.
- row 58: `c2CastRay` with any `typeB` outside `0..=3` returns 0 and leaves
  `out` == `DIRTY`.
- row 65: `c2Absv(-0.0)` keeps the sign bit (`0x80000000`) — the C ternary is
  not `fabsf`.
- row 66: `c2Minv({NaN,1.0},{7.0,NaN})` returns `{7.0, NaN}` — the C ternary is
  not `fminf`.

```
DIFF_ITERS=1000000  debug    55 passed; 0 failed
DIFF_ITERS=1000000  release  55 passed; 0 failed
```

### Generic FFI boundary coverage (beyond the table)

- **Out-of-range enum values** (row 58): the fixed set
  `{-1, -2, 4, 5, 7, 8, 99, 255, 256, 0x10000, i32::MAX, i32::MIN, i32::MIN+1, -0x8000}`
  plus ~1e6 random `int`s outside `0..=3`, each against all four payload shapes
  and with `bx` both null and non-null.
- **Zero / oversized lengths**: `c2Poly.count` ∈ `{-1, 0, 1, 7, 8, 9}` (one step
  past both ends of the valid range and past the array capacity), and `A.t` ∈
  `{0, -0, +1 ULP, -1 ULP, f32::MAX, ±Inf}` against all four raycast entry
  points.
- **Null pointers**: `bx` is the only pointer C null-checks; null `bx` is
  covered for all four `c2CastRay` dispatch types and for `c2RaytoPoly`
  directly, and row 52 asserts null `bx` is bit-identical to an explicit
  `c2xIdentity()`. `out` and `B` are dereferenced unconditionally by the C, so
  passing null is identical UB on both sides (SIGSEGV) and is documented rather
  than tested — see the table above.
- **Array overrun**: `count > 8` is exercised for `count` 9..32 with the polygon
  embedded in an over-allocated `PolyBuf` whose trailing bytes are initialized,
  so both libraries read the *same* out-of-bounds memory and the comparison is
  meaningful. The Rust reads it through `as_ptr().add(i)` rather than indexing,
  so it does not panic where the C does not.

No Rust divergence was found on any error path; the Phase B arithmetic fixes
were sufficient.
