# CONFIGS.md — Phase B configuration surface table

Derived mechanically from `c_src/include/lib.h` (public header) and
`c_src/src/lib.c`.

## Axes the C actually branches on

There are **no** compile-time options: `grep -n '#if\|#ifdef\|#ifndef' c_src/src/lib.c
c_src/include/lib.h` returns 0 hits, and `translation/Cargo.toml` declares **no
`[features]` section**, so there is exactly one feature combination (`default`,
which is empty). `CMakeLists.txt` sets no `-D` macros. There is also **no
binary/driver target** — `add_library(... SHARED ...)` only — so there is no
stdout comparison to make.

The runtime axes are therefore:

* **A1 — entry point.** All 22 exported functions. The low-level vector helpers
  (`c2V`, `c2Dot`, `c2Len`, `c2Add`, `c2Sub`, `c2Mulvs`, `c2Div`, `c2Norm`,
  `c2Minv`, `c2Maxv`, `c2Skew`, `c2Absv`, `c2CCW90`, `c2MulmvT`) are called
  directly, not only through `gen_ray`.
* **A2 — shape dispatch tag** for `c2CastRay`: `C2_TYPE_CIRCLE` (0),
  `C2_TYPE_AABB` (1), `C2_TYPE_CAPSULE` (2). Selects three completely different
  `case` bodies.
* **A3 — float value class.** The code has no integer or size parameters at
  all; every input is an IEEE-754 binary32. The classes the arithmetic and the
  comparisons distinguish are: normal finite, `+0.0`, `-0.0`, subnormal,
  near-overflow (`±1e38`, so `x*x` overflows in `c2Dot`/`c2Len`), `±inf`,
  and NaN (**with an arbitrary payload and sign** — `addss`/`mulss`/`subss`
  return the *destination* operand when both operands are NaN, so NaN payload
  and sign are observable through the FFI and pin down the operand order of
  every arithmetic op).
* **A4 — geometric configuration** of the raycast entry points, i.e. which
  branch of the algorithm is taken:
  * circle: line misses (`disc<0`), tangent (`disc==0`), two roots with
    `0 <= t <= A.t` (hit), root behind origin (`t<0`), root beyond ray
    (`t>A.t`), origin inside the circle.
  * AABB: ray bbox misses B, SAT reject (`d>0`), hit on each of the four faces
    (`-x`, `+x`, `-y`, `+y` — the four-way `t0..t3` tie-break chain), all four
    `t>1` (miss), ray parallel to an axis (`d==0` in
    `c2RayToPlane_OneDimensional`), origin inside B, inverted box
    (`min>max`), degenerate box (`min==max`).
  * capsule: origin in the rotated slab (`c2AABBtoPoint` early `return 1`),
    origin in end-cap A, origin in end-cap B, `|yAp.x| < B.r` delegating to
    `c2RaytoCircle` on cap A (`yAp.y<0`) or cap B (`yAp.y>=0`), slab crossing
    with `y<=0` -> cap A, `y>=yBb.y` -> cap B, `0<y<yBb.y` -> the flat side hit
    (`out->n = M.x` when `c>0` else `c2Skew(M.y)`), and the final fall-through
    miss.
* **A5 — degenerate shape** (valid C input, no validation exists): zero radius,
  negative radius, `capsule.a == capsule.b`, inverted AABB, zero-length ray
  direction, zero / negative / infinite `ray.t`, unnormalised `ray.d`.
* **A6 — `gen_ray` hit-bit combination**: the return value is
  `circle | capsule<<1 | aabb<<2`, so all 8 values 0..7 are distinct outcomes,
  and each of the 3 out-structs must match independently.

## Rows (cross-product, pruned to what the C distinguishes)

Every row is driven with **many randomized inputs** from a fixed-seed
`SplitMix64` PRNG (see `tests/common/mod.rs`), and every result is compared
**bit-for-bit** (`f32::to_bits`) between the C `.so` and the Rust `.so`, both
loaded with `libloading`. Counts below are per row.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `c2V` | A3: 20000 fully random `u32` bit patterns per lane (covers normal / zero / -0 / subnormal / inf / NaN-with-payload) | [x] |
| 2  | `c2V` | A3: cross-product of the 24-value special-value set, both lanes (576 cases) | [x] |
| 3  | `c2Dot` | A3: 20000 random bit-pattern pairs of `c2v` — pins the `mulss`/`addss` operand order of both lanes and the final add | [x] |
| 4  | `c2Dot` | A3: special-value cross-product; includes `inf*0`, `inf + -inf`, two distinct NaNs meeting in one op | [x] |
| 5  | `c2Dot` | A3: near-overflow (`±1e38`, `±3.4e38`) so `x*x` overflows to `±inf` | [x] |
| 6  | `c2Len` | A3: random bit patterns; `c2Dot(a,a)` then `sqrtf` — covers `sqrtf(+inf)`, `sqrtf(NaN)`, `sqrtf(-NaN)` | [x] |
| 7  | `c2Len` | A5: zero vector, `-0.0` vector, subnormal vector (`c2Dot` underflows to 0) | [x] |
| 8  | `c2Add` | A3: 20000 random bit-pattern pairs — pins the `addss` destination operand (C uses `b` as destination in the standalone function) | [x] |
| 9  | `c2Add` | A3: special-value cross-product (`inf + -inf`, NaN + NaN with different payloads/signs, `+0 + -0`) | [x] |
| 10 | `c2Sub` | A3: 20000 random bit-pattern pairs — pins the `subss` destination (`a`) | [x] |
| 11 | `c2Sub` | A3: special-value cross-product (`inf - inf`, `-0 - 0`, NaN pairs) | [x] |
| 12 | `c2Mulvs` | A3: random `c2v` x random scalar — pins the `mulss` destination for **both** lanes | [x] |
| 13 | `c2Mulvs` | A3/A5: scalar = `+0`, `-0`, `1`, `-1`, `inf`, `-inf`, NaN, subnormal, `1e38`; vector from the special set | [x] |
| 14 | `c2Div` | A3: random `c2v` x random scalar — exercises the `1.0f/b` reciprocal then `c2Mulvs` (double rounding, observable in the low bit) | [x] |
| 15 | `c2Div` | A5/ERRORS 34,35: scalar `+0.0` (`1/0=+inf`) and `-0.0` (`1/-0=-inf`), plus `±inf` (`1/inf=0`) and subnormals (`1/tiny=inf`) | [x] |
| 16 | `c2Norm` | A3: random bit patterns — full `c2Len`/`c2Div`/`c2Mulvs` chain | [x] |
| 17 | `c2Norm` | A5/ERRORS 34: zero vector (`0/0` -> `0*inf` -> NaN), `-0.0` vector, `inf` component, near-overflow component | [x] |
| 18 | `c2Minv`, `c2Maxv` | A3: random pairs — pins the ternary (not `fminf`/`fmaxf`) semantics: NaN in either operand and `+0.0` vs `-0.0` both select `b` | [x] |
| 19 | `c2Minv`, `c2Maxv` | A3: special-value cross-product, both argument orders | [x] |
| 20 | `c2Skew`, `c2CCW90`, `c2Absv` | A3: random bit patterns — `c2Absv` must keep `-0.0` and `-NaN` **unchanged** (ternary, not `fabsf`) | [x] |
| 21 | `c2MulmvT` | A3: random `c2m` x random `c2v` (6 random floats) — pins the operand order of all four `mulss` and both `addss` | [x] |
| 22 | `c2MulmvT` | A3/A5: rotation matrix from `c2Norm`/`c2CCW90` (the way `c2RaytoCapsule` builds it) plus all-NaN and all-inf matrices | [x] |
| 23 | `c2AABBtoAABB` | A4: overlapping, and each of the four separating cases d0/d1/d2/d3 individually | [x] |
| 24 | `c2AABBtoAABB` | A4/A5: touching edges (`B.max.x == A.min.x`), fully contained, identical, degenerate (`min==max`), inverted (`min>max`) | [x] |
| 25 | `c2AABBtoAABB` | A3: random bit patterns incl. NaN corners (ERRORS 10: NaN makes it *accept*) | [x] |
| 26 | `c2AABBtoPoint` | A4: inside, and each of the four outside cases d0/d1/d2/d3; on-edge (`==min`, `==max`, both accepted) | [x] |
| 27 | `c2AABBtoPoint` | A3/A5: random bit patterns, NaN point, inverted box, `-0.0` vs `+0.0` bounds | [x] |
| 28 | `c2CircleToPoint` | A4: inside, exactly on the rim (**rejected**, strict `<`), outside; `r == 0`; `r < 0` | [x] |
| 29 | `c2CircleToPoint` | A3: random bit patterns incl. NaN centre/point/radius and `r = inf` | [x] |
| 30 | `c2RaytoCircle` | A4: line miss (`disc<0`) — `out` must stay untouched | [x] |
| 31 | `c2RaytoCircle` | A4: genuine hit `0 <= t <= A.t` — `out->t` and `out->n` compared bit-for-bit | [x] |
| 32 | `c2RaytoCircle` | A4: tangent (`disc == 0`, `t == -b`) | [x] |
| 33 | `c2RaytoCircle` | A4: root behind origin (`t < 0`); ray origin **inside** the circle (also `t<0`) | [x] |
| 34 | `c2RaytoCircle` | A4: hit exists but `t > A.t` | [x] |
| 35 | `c2RaytoCircle` | A5: `A.t == 0`, `A.t < 0`, `A.t == inf`; `B.r == 0`, `B.r < 0`; unnormalised and zero `A.d` | [x] |
| 36 | `c2RaytoCircle` | A3: 20000 fully random bit-pattern `c2Ray`/`c2Circle` (unconstrained fuzz) | [x] |
| 37 | `c2RaytoCircle` | A4: geometrically-structured random rays/circles (so the hit branch is reached ~50% of the time) | [x] |
| 38 | `c2RaytoAABB` | A4: `c2AABBtoAABB` reject (ray bbox misses B) | [x] |
| 39 | `c2RaytoAABB` | A4: SAT reject `d > 0` | [x] |
| 40 | `c2RaytoAABB` | A4: hit selecting each face of the `t0>=t1..` / `t1>=..` / `t2>=..` / else chain (normals `(-1,0)`, `(1,0)`, `(0,-1)`, `(0,1)`) | [x] |
| 41 | `c2RaytoAABB` | A4: all four `t > 1` (`hit == 0`) | [x] |
| 42 | `c2RaytoAABB` | A4/A5: axis-parallel ray (`da == db`, the `d != 0` guard in `c2RayToPlane_OneDimensional`), ray origin inside B, degenerate `min==max` box, inverted `min>max` box | [x] |
| 43 | `c2RaytoAABB` | A5: `A.t` = 0 / negative / `inf`; zero `A.d`; unnormalised `A.d` | [x] |
| 44 | `c2RaytoAABB` | A3: 20000 fully random bit-pattern `c2Ray`/`c2AABB` (unconstrained fuzz) | [x] |
| 45 | `c2RaytoAABB` | A4: geometrically-structured random rays/boxes (hit branch reached often, all four faces covered) | [x] |
| 46 | `c2RaytoCapsule` | A4: origin inside the rotated slab -> early `return 1` with `out->n = c2Norm(cap_n)`, `out->t = 0` | [x] |
| 47 | `c2RaytoCapsule` | A4: origin inside end-cap A (`c2CircleToPoint(capsule_a, A.p)`) -> `return 1` | [x] |
| 48 | `c2RaytoCapsule` | A4: origin inside end-cap B -> `return 1` | [x] |
| 49 | `c2RaytoCapsule` | A4: `|yAp.x| < B.r` and `yAp.y < 0` -> delegate `c2RaytoCircle` on cap A | [x] |
| 50 | `c2RaytoCapsule` | A4: `|yAp.x| < B.r` and `yAp.y >= 0` -> delegate `c2RaytoCircle` on cap B | [x] |
| 51 | `c2RaytoCapsule` | A4: slab crossing, `y <= 0` -> delegate `c2RaytoCircle` on cap A | [x] |
| 52 | `c2RaytoCapsule` | A4: slab crossing, `y >= yBb.y` -> delegate `c2RaytoCircle` on cap B | [x] |
| 53 | `c2RaytoCapsule` | A4: slab crossing, `0 < y < yBb.y` -> flat-side hit, `out->n = M.x` (when `c > 0`) | [x] |
| 54 | `c2RaytoCapsule` | A4: slab crossing, `0 < y < yBb.y` -> flat-side hit, `out->n = c2Skew(M.y)` (when `c <= 0`) | [x] |
| 55 | `c2RaytoCapsule` | A4: final fall-through miss (`ret == 0` **with `out` already written**) | [x] |
| 56 | `c2RaytoCapsule` | A5/ERRORS 28: degenerate capsule `B.a == B.b` (`c2Norm` of zero -> NaN matrix) | [x] |
| 57 | `c2RaytoCapsule` | A5/ERRORS 29,30: `B.r == 0`, `B.r < 0` (inverted `capsule_bb`), `yAe.x == yAp.x` (division by zero at L278) | [x] |
| 58 | `c2RaytoCapsule` | A5: `A.t` = 0 / negative / `inf`; zero `A.d`; capsule reversed (`b` before `a`, so `yBb.y < 0`) | [x] |
| 59 | `c2RaytoCapsule` | A3: 20000 fully random bit-pattern `c2Ray`/`c2Capsule` (unconstrained fuzz) | [x] |
| 60 | `c2RaytoCapsule` | A4: geometrically-structured random rays/capsules (all branches above reached; branch histogram asserted non-zero) | [x] |
| 61 | `c2CastRay` | A2=`C2_TYPE_CIRCLE` x A3 random + A4 structured — result must equal a direct `c2RaytoCircle` call and match C | [x] |
| 62 | `c2CastRay` | A2=`C2_TYPE_AABB` x A3 random + A4 structured | [x] |
| 63 | `c2CastRay` | A2=`C2_TYPE_CAPSULE` x A3 random + A4 structured | [x] |
| 64 | `gen_ray` | A6: each of the 8 hit-bit combinations 0..7, hand-constructed, all three `c2Raycast` outputs compared | [x] |
| 65 | `gen_ray` | A3: 20000 fully random bit patterns for all 18 float parameters | [x] |
| 66 | `gen_ray` | A4/A6: geometrically-structured random scenes (mouse point, ray origin, circle, capsule, AABB all in a plausible range) — the realistic consumer path | [x] |
| 67 | `gen_ray` | A5: `mp == ray.p` (zero ray direction -> NaN `ray.d`/`ray.t`, ERRORS 39); degenerate capsule; inverted AABB; zero/negative radii | [x] |
| 68 | `gen_ray` | A3: the special-value set injected into one parameter at a time, all 18 positions x 24 values | [x] |
| 69 | all 22 entry points | A3: `-0.0` vs `+0.0` discrimination sweep (sign of zero must survive every op identically) | [x] |
| 70 | all 22 entry points | A3: NaN-payload sweep — the same NaN payload set fed through every function, asserting the exact returned NaN bits (this is what pins the `mulss`/`addss` destination operand of every op) | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the complete set of
feature combinations is `{default}` = `{}`. Verified by
`cargo metadata --format-version 1 --no-deps | python3 -c ...` (see
`check_features.sh`): 0 declared features. Tests are additionally run under
`--no-default-features` and `--release` to confirm both profiles agree.

## Non-vacuity check (the suite actually detects divergence)

To prove the harness is not vacuously green, a single operand order in
`src/lib.rs` was reverted to its pre-fix form:

```diff
-fn dot(a: c2v, b: c2v) -> f32 { fadd(fmul(b.y, a.y), fmul(a.x, b.x)) }
+fn dot(a: c2v, b: c2v) -> f32 { fadd(fmul(a.x, b.x), fmul(a.y, b.y)) }
```

`cargo test --test vec_ops` then reported **5 failed / 22 passed** (rows 3, 4, 6,
7, 70 — every row whose inputs can put two distinct NaNs into one `mulss`/`addss`).
Restoring the fix returned the suite to 27/27. The same held for the other five
divergences found and fixed during Phase B:

| # | function | C reference codegen | pre-fix Rust (wrong) |
|---|----------|---------------------|----------------------|
| 1 | `c2Dot` | `mulss` dest = `a.x` / `b.y`; `addss` dest = the *y* product | dest = `a.x` / `a.y`; `addss` dest = the *x* product |
| 2 | `c2Add` (incl. every internal call) | `addss` dest = `b` in both lanes | dest = `a` internally, `b` only in the exported wrapper |
| 3 | `c2Mulvs` | `mulss` dest = `a` in **both** lanes | dest = `b` in the `y` lane |
| 4 | `c2MulmvT` | per row: `mulss` dest = matrix entry / `b.y`; `addss` dest = 2nd product | three different wrong orders (and the wrapper disagreed with the internal helper) |
| 5 | `c2RaytoAABB` `out->t = tN * A.t` (x4) | `mulss` dest = `A.t` | dest = `tN` |
| 6 | `c2RaytoCapsule` `y = yAp.y + (yAe.y-yAp.y)*t` and `out->t = t*A.t` | `addss` dest = the product; `mulss` dest = `A.t` | dest = `yAp.y`; dest = `t` |

Because the C is compiled at `-O0` (no `-O` flag in `CMakeLists.txt`), **nothing
is inlined**: every `c2Sub`/`c2Dot`/`c2Add`/`c2Mulvs`/`c2MulmvT` call in the C is
a real `call`, so each helper's operand order is the same for the exported entry
point and for every internal caller. The Rust mirrors that by having exactly one
implementation per helper, with the exported `#[no_mangle]` wrapper delegating to
it (rather than duplicating the arithmetic, which is how divergences 2 and 4 got
in).

## How to re-run everything

```sh
cd translation && ./run_diff_tests.sh
```

This builds the C `.so`, then for each of `--no-default-features`,
`--all-features` and `<default>` x `{debug, release}` rebuilds the Rust cdylib
(required: `cargo test` alone does **not** rebuild a `cdylib`) and runs all 55
differential tests, then gates on `nm -D` symbol parity and on there being no
undefined non-libc symbols. Last run: **6/6 configurations green, 55 tests each,
22/22 symbols, 0 missing, 0 extra, 0 undefined non-libc.**
