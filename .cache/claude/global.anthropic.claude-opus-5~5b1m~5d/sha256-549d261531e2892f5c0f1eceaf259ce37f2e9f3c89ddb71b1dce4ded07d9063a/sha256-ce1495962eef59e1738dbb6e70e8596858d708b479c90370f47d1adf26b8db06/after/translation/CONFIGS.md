# CONFIGS.md — Phase A: CONFIGURATION-SURFACE TABLE

The mirror of `ERRORS.md`: the **valid**-input axes the C actually branches on,
enumerated mechanically from `c_src/include/lib.h` + `c_src/src/lib.c`.

## Axes the C code actually distinguishes

**A1 — runtime "option" arguments (the only ones the public API exposes):**

| option | set via | states the C branches on |
|--------|---------|--------------------------|
| shape selector | `C2_TYPE typeB` arg of `c2CastRay` (line 368 `switch`) | `C2_TYPE_CIRCLE`(0), `C2_TYPE_AABB`(1), `C2_TYPE_CAPSULE`(2), `C2_TYPE_POLY`(3), + out-of-range (→ `ERRORS.md` row 50) |
| body transform | `const c2x *bx` arg of `c2RaytoPoly` / `c2CastRay` (line 338 ternary) | `NULL` ⇒ `c2xIdentity()`; non-`NULL` ⇒ `*bx` (which itself splits into identity-valued, pure-translation, pure-rotation, translation+rotation, non-unit `c2r`) |
| transform forwarding | `c2CastRay` | `bx` is forwarded **only** for `C2_TYPE_POLY`; ignored for the other three |
| ray length | `c2Ray.t` | `0`, `>0`, `<0`, `+inf` — used as the `hi` initialiser (poly), the `p1` extrapolation (AABB/capsule), and the `t <= A.t` gate (circle) |

There are **no** compile-time options: `grep -c '#ifdef\|#if\|#ifndef' c_src/src/lib.c`
= 0, and `translation/Cargo.toml` has no `[features]` section. Exactly one
feature combination exists (see `SYMBOLS.md`).

**A2 — input shapes the code special-cases:**

* `c2Poly.count`: `0`, `1`, `2`, `3`, `4` (the `poly_ray` shape), `5`, `8` (full),
  `>8` (out-of-bounds read, `ERRORS.md` row 45), `<0`.
* polygon geometry: axis-aligned box vs convex regular n-gon vs random convex hull;
  `norms` consistent-with-`verts` vs deliberately inconsistent (the C never
  recomputes them).
* ray origin: strictly outside / on a face / strictly inside the shape.
* ray direction: unit / non-unit / zero `(0,0)` / axis-aligned / anti-parallel to a normal.
* AABB: proper, degenerate (`min == max`), inverted (`min > max`), zero-width in one axis.
* circle: `r > 0`, `r == 0`, `r < 0`.
* capsule: `a != b`, `a == b` (degenerate), horizontal / vertical / oblique axis,
  `r > 0` / `== 0` / `< 0`.
* scalar float classes fed to the vector math: normal, `±0.0`, subnormal,
  `±FLT_MAX`, `±inf`, quiet NaN (both sign bits), values around `1.0`.
* `c2r`: identity (`1,0`), normalised (`cos,sin`), non-normalised, zero (`0,0`).

**A3 — full set of public entry points.** All 28 exported symbols are driven
directly, *not* only through the `poly_ray` one-shot wrapper. The call hierarchy
is exercised bottom-up: leaf vector math (rows 1–14) → predicates (15–18) →
per-shape raycasts (19–33) → the `c2CastRay` dispatcher (34–39) → the `poly_ray`
convenience wrapper (40).

## Table

One row per combination the C treats differently. Every row is driven with
**many randomized inputs** (fixed seed `0x5EED_C2C2`, splitmix64 + a float
generator that mixes uniform-range, special-class, and bit-pattern draws), and
all outputs are compared **bit-for-bit** (`to_bits()`), so `+0.0` vs `-0.0` and
distinct NaN encodings are caught. `[x]` = passes.

### Leaf vector math (called directly through the `.so`)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `c2V` | 4096 random `(x,y)` bit patterns incl. `±0`, subnormal, `±inf`, NaN | [x] |
| 2  | `c2Dot` | 4096 random pairs, finite range `±1e3`; return is a scalar `float` in `xmm0` | [x] |
| 3  | `c2Dot` | special-class pairs: `±0`, `±inf`, NaN, `±FLT_MAX` (overflow to `±inf`, `inf*0` ⇒ NaN) | [x] |
| 4  | `c2Len` | random finite vectors (exercises `sqrtf` of a non-negative dot) | [x] |
| 5  | `c2Len` | `(0,0)` ⇒ `0`; huge vectors ⇒ `dot` overflows ⇒ `sqrt(inf)`; NaN input | [x] |
| 6  | `c2Add`, `c2Sub` | random finite pairs + `±0` sign-of-zero cases (`0 + -0`, `-0 - 0`) + inf/NaN | [x] |
| 7  | `c2Mulvs` | random vector × random scalar, incl. `0 * inf`, `±0` scalars, NaN scalar | [x] |
| 8  | `c2Div` | random vector ÷ random **non-zero** scalar — must reproduce the C's `a * (1.0f/b)` reciprocal, *not* `a/b` (differs in the last bit for most `b`) | [x] |
| 9  | `c2Div` | `b == +0.0`, `b == -0.0`, `b == ±inf`, `b == NaN`, `b` subnormal (`1/b` overflows) | [x] |
| 10 | `c2Norm` | random finite vectors (composition `c2Div(a, c2Len(a))`) | [x] |
| 11 | `c2Norm` | `(0,0)` ⇒ `(NaN,NaN)`; already-unit vectors; huge vectors; NaN component | [x] |
| 12 | `c2Minv`, `c2Maxv` | random finite pairs + **NaN in each argument position separately** (the ternary is asymmetric, unlike `fminf`) + `(+0,-0)` and `(-0,+0)` | [x] |
| 13 | `c2Skew`, `c2CCW90` | random incl. `±0` (negation of zero flips the sign bit) and NaN (sign bit preserved) | [x] |
| 14 | `c2Absv` | random + `-0.0` (⇒ `-0.0`, unlike `fabsf`) + `-NaN` (⇒ `-NaN`, sign kept) + `-inf` | [x] |

### Rotation / transform math

| #  | entry point(s) | configuration | [x] |
|----|----------------|---------------|-----|
| 15 | `c2RotIdentity`, `c2xIdentity` | no arguments — verifies the 8-byte and 16-byte struct **return** ABI | [x] |
| 16 | `c2Mulrv`, `c2MulrvT` | `c2r` = identity, normalised `(cosθ,sinθ)` over 64 angles, non-normalised, `(0,0)`, NaN/inf components × random `c2v` | [x] |
| 17 | `c2MulmvT` | `c2m` = identity, orthonormal-from-`c2CCW90`, random, singular (both columns equal), NaN × random `c2v` | [x] |
| 18 | `c2MulxvT` | `c2x` = identity, pure translation, pure rotation, translation+rotation, non-unit `c2r`, NaN — verifies the 16-byte struct **argument** ABI | [x] |

### Predicates

| #  | entry point(s) | configuration | [x] |
|----|----------------|---------------|-----|
| 19 | `c2AABBtoAABB` | random proper boxes: overlapping, touching exactly, separated on each of the 4 axes, one containing the other | [x] |
| 20 | `c2AABBtoAABB` | degenerate (`min == max`), inverted (`min > max`), NaN coordinates | [x] |
| 21 | `c2AABBtoPoint` | random point vs random proper box: inside, exactly on each of the 4 edges, outside on each side, corners | [x] |
| 22 | `c2AABBtoPoint` | degenerate / inverted box, NaN point or box | [x] |
| 23 | `c2CircleToPoint` | random point vs circle: strictly inside, **exactly on the rim** (strict `<` ⇒ miss), outside; `r>0`/`r==0`/`r<0`; NaN | [x] |

### Circle raycast

| #  | entry point(s) | configuration | [x] |
|----|----------------|---------------|-----|
| 24 | `c2RaytoCircle` | random ray (unit `d`) vs random circle `r>0`, origin **outside**, `A.t` large enough to reach ⇒ hit path, `out->t`/`out->n` compared | [x] |
| 25 | `c2RaytoCircle` | origin **inside** the circle (`t < 0` ⇒ miss); origin exactly **on** the rim (`t == 0` ⇒ hit, `n` = radial) | [x] |
| 26 | `c2RaytoCircle` | non-unit and zero `d`; `A.t` = `0`, small, huge, negative, `+inf`; tangent rays (`disc ≈ 0`) | [x] |
| 27 | `c2RaytoCircle` | `r == 0` / `r < 0`; NaN & `±inf` in `A.p`, `A.d`, `A.t`, `B.p`, `B.r` | [x] |

### AABB raycast

| #  | entry point(s) | configuration | [x] |
|----|----------------|---------------|-----|
| 28 | `c2RaytoAABB` | random ray vs random proper box, **hit** path — covers all four `out->n` selections `(-1,0)`, `(1,0)`, `(0,-1)`, `(0,1)` (the 4-way `if/else if/else` chain, lines 197–209) | [x] |
| 29 | `c2RaytoAABB` | axis-aligned rays (`d = ±x̂`, `±ŷ`) — makes `n == (0,0)` after `c2Skew` for one axis and drives `c2RayToPlane_OneDimensional`'s `d == 0` branch | [x] |
| 30 | `c2RaytoAABB` | ray origin **inside** the box; ray fully **before**/**after** the box along its own line; grazing a corner exactly | [x] |
| 31 | `c2RaytoAABB` | `A.t` = `0` / negative / `+inf`; `d = (0,0)`; degenerate, inverted, and zero-width boxes; NaN/inf coordinates | [x] |

### Capsule raycast

| #  | entry point(s) | configuration | [x] |
|----|----------------|---------------|-----|
| 32 | `c2RaytoCapsule` | random capsule (`a != b`, `r > 0`) × random ray, covering **each** exit of the branch tree: (a) `c2AABBtoPoint(capsule_bb, yAp)` early `return 1`; (b) `c2CircleToPoint(capsule_a, A.p)`; (c) `c2CircleToPoint(capsule_b, A.p)`; (d) `\|yAp.x\| < r` ∧ `yAp.y < 0` ⇒ delegate to `Ca`; (e) `\|yAp.x\| < r` ∧ `yAp.y >= 0` ⇒ delegate to `Cb`; (f) side-plane `y <= 0` ⇒ `Ca`; (g) side-plane `y >= yBb.y` ⇒ `Cb`; (h) side-plane hit, `c > 0` ⇒ `n = M.x`; (i) side-plane hit, `c <= 0` ⇒ `n = c2Skew(M.y)`; (j) fall-through `return 0` **with `*out` pre-written** | [x] |
| 33 | `c2RaytoCapsule` | axis orientation: horizontal, vertical, oblique, `b` below `a` (negative `yBb.y`); `a == b` degenerate; `r == 0`; `r < 0`; `A.t` = `0`/negative/`+inf`; `d = (0,0)`; NaN/inf | [x] |

### Poly raycast + dispatcher (lowest-level entry points driven directly)

| #  | entry point(s) | configuration | [x] |
|----|----------------|---------------|-----|
| 34 | `c2RaytoPoly` | `bx == NULL`, axis-aligned box poly `count == 4`, random rays — hit and miss | [x] |
| 35 | `c2RaytoPoly` | `bx == NULL`, convex regular n-gon for **every** `count` in `1..=8`, random rays | [x] |
| 36 | `c2RaytoPoly` | `bx != NULL`: identity-valued, pure translation, pure rotation (64 angles), translation+rotation, non-normalised `c2r`, zero `c2r` — × random rays × `count` `3..=8` | [x] |
| 37 | `c2RaytoPoly` | `count` = `0`, `-1`, `INT_MIN` (loop skipped); `count` = `9..=16` with a **padded, deterministically filled backing buffer** so the out-of-bounds reads are identical bytes for both libraries (`ERRORS.md` row 45) | [x] |
| 38 | `c2RaytoPoly` | fully random (non-convex, inconsistent `norms`, NaN/inf verts & norms) `c2Poly` + random `c2Ray` + random `c2x` — pure property fuzz, 20 000 cases | [x] |
| 39 | `c2CastRay` | every valid `typeB` (`0,1,2,3`) × (`bx == NULL`, `bx != NULL`) × the shape configurations of rows 24–38, with the shape written into a **shared byte buffer** so both libraries reinterpret identical bytes; asserts the return value **and** `*out` | [x] |
| 40 | `poly_ray` | the one-shot wrapper: no arguments beyond two `out` pointers; asserts the packed `hit` bitfield (`hit0 + (hit1 << 1)`) and both `c2Raycast` outputs bit-exactly | [x] |
| 41 | `c2CastRay` → `c2RaytoPoly` | `typeB == C2_TYPE_POLY` with the *same* poly bytes but `bx` supplied vs `NULL`, asserting `c2CastRay` forwards `bx` (and that it is *ignored* for types 0/1/2) | [x] |

## Rows 42–48 — the NaN/infinity operand matrix (`tests/phase_b_nan_matrix.rs`)

These rows were **added after mutation-testing the suite** revealed a blind
spot: several arithmetic sites are only distinguishable when *both* operands of
one SSE instruction are NaN **with different bit patterns**. That needs an input
NaN with a distinctive payload to meet a *second* NaN manufactured internally by
an invalid operation (`0*inf`, `inf-inf`, `0/0`) — a combination that uniform
random floats essentially never produce. The rows therefore drive a deliberately
chosen alphabet (`±0.0`, `±1.0`, `±inf`, and four distinguishable NaN encodings
including signalling ones) through **every** argument slot.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 42 | `c2RaytoAABB` | a distinctive NaN pinned into each of the 9 float slots in turn × the 5-value {0, ±1, ±inf} alphabet swept exhaustively over four co-slots (4 × 9 × 625 cases) | [x] |
| 43 | `c2RaytoAABB` | 400 000 random draws with all 9 slots over the full 9-value alphabet, plus 200 000 draws forced to carry **two different** NaNs. This is the row that pins `out->t = mulss(A.t, t3)` — the only `t_k` branch a NaN can reach | [x] |
| 44 | `c2RaytoCapsule` | pinned NaN × 10 slots (4 × 10 × 625), plus an exhaustive degenerate-axis sweep (`a == b`, which manufactures `0xFFC00000` in `M.y` and `0x7FC00000` in `M.x`), plus 600 000 random draws | [x] |
| 45 | `c2RaytoCircle` | pinned NaN × 8 slots (4 × 8 × 625) + 300 000 random full-alphabet draws | [x] |
| 46 | `c2RaytoPoly` | 25 polygons (the fixed box + 24 whose verts/norms are drawn from the alphabet, counts 1..=10) × pinned NaN in each of the 9 ray/transform slots × {`bx == NULL`, `bx != NULL`}, plus 200 000 random draws | [x] |
| 47 | `c2Dot`/`c2Add`/`c2Sub`/`c2Minv`/`c2Maxv`/`c2Mulrv`/`c2MulrvT` | all 4 float slots exhaustive over the 9-value alphabet (6 561 combinations each) | [x] |
| 48 | `c2MulmvT`/`c2Len`/`c2Norm`/`c2Skew`/`c2CCW90`/`c2Absv`/`c2Mulvs`/`c2Div`/`c2MulxvT` | matrix **and** vector arguments both over the full alphabet — `c2MulmvT`'s four `mulss` operand orders are only observable when a matrix lane and a vector lane are simultaneously NaN, so restricting the vector to a NaN-free alphabet left them unpinned | [x] |

## Verification that these rows have teeth

* `operand_order_check.py` swaps the operands of every one of the 65
  `addss`/`mulss`/`subss`/`divss` sites in `src/lib.rs`, one at a time, and
  reports whether the suite notices: **46 CAUGHT, 19 provably unobservable**
  (each justified in the module docs of `src/lib.rs`). No `subss`/`divss` site
  is unobservable, which also proves every arithmetic site is *reachable*.
* `mutation_check.sh` injects 29 whole-behaviour translation mistakes:
  **27 caught, 2 documented as unobservable through the public API**
  (`poly_ray` takes no arguments and both of its hard-coded rays miss, so
  `0 << 1 == 0` and unread vertex data cannot be observed — the C cannot
  distinguish them either).
