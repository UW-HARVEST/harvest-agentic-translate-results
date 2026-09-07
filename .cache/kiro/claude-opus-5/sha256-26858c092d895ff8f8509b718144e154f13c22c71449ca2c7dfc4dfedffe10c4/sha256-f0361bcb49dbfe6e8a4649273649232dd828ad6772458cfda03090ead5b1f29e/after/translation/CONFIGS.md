# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`, plus the
compiled C `.so` disassembly (the C is built with **no** optimisation flags —
`c_src/CMakeLists.txt` sets none — so every helper is a real `call` through the
PLT and every FP operation is a discrete `mulss`/`addss`/`subss`/`comiss`; the
operand order in that asm is part of the observable behaviour for NaN payloads).

## Axes the C actually branches on

**Axis 1 — entry point.** There are 10 exported functions, only *one* of which
(`collided`) is declared in the public header. The other nine are the low-level
API and are exercised directly, not only through the `collided` wrapper:
`c2V`, `c2Maxv`, `c2Minv`, `c2Clampv`, `c2Sub`, `c2Dot`, `c2CircletoCircle`,
`c2CircletoAABB`, `c2AABBtoAABB`, `collided`.

**Axis 2 — runtime option/mode.** The only runtime option the public API can set
is the `C2_TYPE` tag pair `(typeA, typeB)` passed to `collided`. Valid states:
`(CIRCLE,CIRCLE)`, `(CIRCLE,AABB)`, `(AABB,CIRCLE)`, `(AABB,AABB)`.
Note `(AABB,CIRCLE)` (lib.c:89-90) dispatches to
`c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A)` — the *swapped* operand order, a
distinct code path from `(CIRCLE,AABB)`. There are no `#ifdef`s and no other
flags (`grep -c '#if' → 0`).

**Axis 3 — control-flow branches inside the math.** 13 branch/ternary sites:
the 2 ternaries in `c2Maxv`, the 2 in `c2Minv`, the 4 `<` comparisons in
`c2AABBtoAABB`, the 2 `d2 < r2` compares, and the 3 `switch`es. Each ternary
takes the *else* arm for NaN (`comiss`+`jbe`), so NaN-vs-finite ordering is a
distinct configuration from finite-vs-finite.

**Axis 4 — input value shape (float bit classes).** The code does no
classification, but the hardware does: `+/-` normal, `+0.0`, `-0.0`, subnormal,
`+inf`, `-inf`, QNaN (payload-carrying), SNaN (quieted by the ALU), and
magnitudes large enough that `r*r` / `d2` overflow to `inf`, or small enough that
`x*x` underflows to `0`/subnormal.

**Axis 5 — geometric shape.** overlapping, exactly touching (`d2 == r2`, the
strict-`<` boundary), disjoint, one shape fully inside the other, circle centre
inside the AABB (clamp is a no-op ⇒ `d2 == 0`), zero radius, negative radius,
inverted AABB (`min > max`), degenerate AABB (`min == max`), and box/circle
struct sizes mismatching the tag (12-byte `c2Circle` vs 16-byte `c2AABB`
reinterpretation).

## Rows

Each row is checked off only after **many randomized inputs** (fixed seed,
property-style) pass byte-for-byte against the C `.so`. Test names refer to
`translation/tests/differential.rs`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random finite `f32` bit patterns; asserts returned struct bits identical (pure struct-return ABI check) | [x] |
| 2 | `c2V` | special values: `±0.0`, `±inf`, subnormals, QNaN & SNaN payloads (must pass through unquieted — no arithmetic) | [x] |
| 3 | `c2Maxv` | both components finite normals, all orderings (`a>b`, `a<b`, `a==b`) | [x] |
| 4 | `c2Maxv` | `±0.0` mixes (`+0.0 > -0.0` is false ⇒ returns `b`) and subnormals | [x] |
| 5 | `c2Maxv` | NaN in `a` only / `b` only / both, with distinct payloads ⇒ else-arm (`b`) taken, payload preserved | [x] |
| 6 | `c2Maxv` | `±inf` operands, mixed with finites and NaN | [x] |
| 7 | `c2Minv` | both components finite normals, all orderings | [x] |
| 8 | `c2Minv` | `±0.0` mixes and subnormals | [x] |
| 9 | `c2Minv` | NaN in `a` / `b` / both with distinct payloads ⇒ else-arm (`b`) | [x] |
| 10 | `c2Minv` | `±inf` operands mixed with finites and NaN | [x] |
| 11 | `c2Clampv` | `lo <= hi`, point below / inside / above the range (composes `c2Maxv(lo, c2Minv(a,hi))`) | [x] |
| 12 | `c2Clampv` | **inverted range** `lo > hi` (unchecked by C; result is `lo`-dominated) | [x] |
| 13 | `c2Clampv` | NaN in `a`, in `lo`, in `hi`, and combinations — exercises the composed ternary chain, where NaN placement changes which operand survives | [x] |
| 14 | `c2Clampv` | `±inf` bounds, `lo == hi` (degenerate), `±0.0` bounds | [x] |
| 15 | `c2Sub` | random finite normals (`subss` per component) | [x] |
| 16 | `c2Sub` | overflow to `±inf` (`FLT_MAX - -FLT_MAX`), underflow to subnormal/`±0.0`, `x - x == +0.0`, `-0.0 - +0.0 == -0.0` | [x] |
| 17 | `c2Sub` | `inf - inf` (same sign) ⇒ QNaN indefinite `0xFFC00000`; `inf - (-inf)` ⇒ `inf` | [x] |
| 18 | `c2Sub` | NaN operands with distinct payloads in `a` and/or `b` ⇒ SSE dst-preference (`a` wins), SNaN quieted | [x] |
| 19 | `c2Dot` | random finite normals (checks the pinned `mulss a.x,b.x` / `mulss b.y,a.y` / `addss q,p` operand order) | [x] |
| 20 | `c2Dot` | products that overflow to `±inf`, and `+inf + -inf` in the sum ⇒ QNaN indefinite | [x] |
| 21 | `c2Dot` | `0 * inf` ⇒ QNaN indefinite; subnormal products underflowing to `0` | [x] |
| 22 | `c2Dot` | NaN in one/both products with distinct payloads ⇒ exercises the exact `addss` dst/src NaN priority (`b.y*a.y` is dst) | [x] |
| 23 | `c2CircletoCircle` | random overlapping circles (finite normals, positive radii) | [x] |
| 24 | `c2CircletoCircle` | random disjoint circles | [x] |
| 25 | `c2CircletoCircle` | exactly touching, `d2 == r2` — the strict-`<` boundary ⇒ `0` | [x] |
| 26 | `c2CircletoCircle` | identical circles, zero radius (`r == 0`, coincident centres ⇒ `d2 == r2 == 0` ⇒ `0`) | [x] |
| 27 | `c2CircletoCircle` | **negative radii** (unchecked; `A.r + B.r` may cancel to `0` or go negative before squaring) | [x] |
| 28 | `c2CircletoCircle` | huge radii/centres so `r2` or `d2` overflows to `inf` (`inf < inf` ⇒ `0`) | [x] |
| 29 | `c2CircletoCircle` | NaN / `±inf` in centres and/or radii (unordered `comiss` ⇒ `0`) | [x] |
| 30 | `c2CircletoAABB` | circle centre **inside** the box (clamp is identity ⇒ `d2 == 0 < r2`) | [x] |
| 31 | `c2CircletoAABB` | centre outside on each of the 8 sides/corners, overlapping | [x] |
| 32 | `c2CircletoAABB` | centre outside, disjoint | [x] |
| 33 | `c2CircletoAABB` | exactly touching (`d2 == r2`) ⇒ `0`; and the corner-touch case | [x] |
| 34 | `c2CircletoAABB` | degenerate box `min == max` (point box) and zero-radius circle | [x] |
| 35 | `c2CircletoAABB` | **inverted box** `min > max` on one or both axes (unchecked by C) | [x] |
| 36 | `c2CircletoAABB` | negative radius; huge coordinates overflowing `d2`/`r2` to `inf` | [x] |
| 37 | `c2CircletoAABB` | NaN / `±inf` in centre, radius, and box corners (all placements) | [x] |
| 38 | `c2AABBtoAABB` | random overlapping boxes | [x] |
| 39 | `c2AABBtoAABB` | disjoint on x only, y only, and both | [x] |
| 40 | `c2AABBtoAABB` | edge-touching (`B.max.x == A.min.x`) — `<` is false ⇒ `1` (touching counts as collision, unlike the circles) | [x] |
| 41 | `c2AABBtoAABB` | degenerate `min == max`, inverted `min > max`, `±0.0` corners | [x] |
| 42 | `c2AABBtoAABB` | `±inf` corners (infinite box contains everything) | [x] |
| 43 | `c2AABBtoAABB` | NaN corners in every one of the 8 slots ⇒ all four `<` false ⇒ `1` | [x] |
| 44 | `collided` | `(CIRCLE, CIRCLE)` — random circles across all shapes of rows 23-29 | [x] |
| 45 | `collided` | `(CIRCLE, AABB)` — `A` read as 12-byte circle, `B` as 16-byte box | [x] |
| 46 | `collided` | `(AABB, CIRCLE)` — the **swapped** dispatch: `B` read as circle, `A` as box | [x] |
| 47 | `collided` | `(AABB, AABB)` | [x] |
| 48 | `collided` | all 4 valid tag pairs where the *same* pointer is passed for `A` and `B` (aliasing) | [x] |
| 49 | `collided` | all 4 valid tag pairs over a shared 16-byte buffer, so the tag decides how many bytes are read and which struct layout is applied to identical bytes | [x] |
| 50 | `collided` | all 4 valid tag pairs with NaN/inf/subnormal payloads in the pointed-to structs | [x] |
| 51 | end-to-end | full random sweep: for each iteration build random `c2Circle`/`c2AABB` from random 32-bit words (all float classes reachable) and compare every one of the 10 exports in one pass | [x] |

## How the rows are exercised

- `tests/differential.rs` — one `rowNN_*` test per row above (51 tests).
- `tests/adversarial.rs` — 8 denser sweeps layered on top of the rows:
  exhaustive 4-slot cross-products over a 24-entry NaN/inf/zero/boundary bit
  pattern pool (331 776 argument sets per function), a 1 000 000-iteration
  uniform-random-bits sweep, a 500 000-iteration special-value-biased sweep, a
  200 000-iteration raw-byte sweep through `collided`, a full 255×255
  exponent grid for `c2Dot`/`c2Sub` rounding, and a ±4-ULP sweep across
  many magnitudes around the `d2 < r2` decision boundary.
- `tests/errors.rs` — Phase C (see `ERRORS.md`).
- `tests/symbols.rs` — Phase D symbol parity, asserted as a test so it cannot
  drift.

All of it runs via `./run-tests.sh`, which builds the C `.so`, builds the Rust
cdylib (**required** — `cargo test` does *not* rebuild a `cdylib`-only lib
target, so without this the tests would `dlopen` a stale `.so`; the harness also
asserts `.so` freshness and refuses to run otherwise), then runs every test in
`debug` and `release` for every feature combination, and finally diffs `nm -D`.

## Mutation testing (test-sensitivity evidence)

Passing tests only prove something if they would fail on a wrong translation. 25
mutations were injected into `src/lib.rs` one at a time, rebuilt, and re-tested.
Every behaviour-changing mutation was caught. The 5 survivors were confirmed by
hand to be **semantically equivalent** to the original, i.e. unkillable:

| survivor | why it is equivalent |
|----------|----------------------|
| `c2CircletoCircle`: `addss(B.r, A.r)` → `addss(A.r, B.r)` | FP addition is commutative except for the NaN payload, and any NaN `r2` makes `comiss` unordered ⇒ `0` either way, so the payload is unobservable through the `int` return |
| `c2AABBtoAABB`: `(d0\|d1\|d2\|d3) == 0` → `(d0+d1+d2+d3) == 0` | each `dN` is `0` or `1`, so bitwise-or and sum are zero on exactly the same inputs |
| `mulss`: drop the explicit `0*inf` → `0xFFC00000` rule | Rust's own `f32 *` lowers to `mulss` on x86-64 and already yields `0xFFC00000` (verified empirically) |
| `subss`: drop the explicit `inf-inf` rule | same reason, for `subss` |
| `addss`: drop the explicit `inf+(-inf)` rule | same reason, for `addss` |

Representative kills: flipping the NaN dst/src priority in `mulss`/`addss`/`subss`
(5 tests each), replacing the `c2Maxv`/`c2Minv` ternaries with `f32::max`/`f32::min`
(30–36 tests), un-swapping the `(AABB, CIRCLE)` dispatch in `collided` (6 tests),
turning a `default: return 0` arm into `1` (4–5 tests), `<` → `<=` at the
`d2 < r2` boundary (15–18 tests), and reverting the `read_unaligned` fix.
