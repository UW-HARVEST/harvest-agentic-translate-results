# ERRORS.md — Phase A: ERROR-SURFACE TABLE

Mechanically derived from `c_src/src/lib.c` by grepping **every** `return`,
`if`, `else`, `switch`, `case`, ternary, `assert`, and null check
(`grep -n 'return' ; grep -n 'if *(\|else\|switch\|case\|assert\|NULL\|?'`).

## Inventory of the C's rejection vocabulary

There is **no error enum, no `RETURN_ERROR` macro, no `errno`, no `assert`, and
no `return NULL`** anywhere in `c_src/src/lib.c`. The library's *entire*
rejection vocabulary is:

* `return 0;` from an `int`-returning predicate/raycast — meaning **"no hit" /
  "false"**. This is the only sentinel.
* `return !(d0|d1|d2|d3);` / `return d2 < A.r*A.r;` — boolean 0/1.
* Exactly **one** null check exists in the whole file:
  `c2x bx = bx_ptr ? *bx_ptr : c2xIdentity();` (line 338). `bx_ptr == NULL` is a
  *documented valid* input meaning "identity transform", not an error.
* There is **no** null check on `out`, on `B` in `c2RaytoPoly`, or on `B` in
  `c2CastRay`; there is **no** bounds check on `c2Poly.count` against the
  fixed `verts[8]`/`norms[8]` arrays; there is **no** validation of the
  `C2_TYPE typeB` argument beyond the `switch`'s implicit default.
* The magic constant `~0` (= `-1`) is used as the "no index yet" sentinel in
  `c2RaytoPoly` (lines 343, 359).

Every distinct rejection branch gets one row below. `[x]` = a differential test
exists and passes against both `.so`s.

## Table

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|---------------------------------------------|-------------------|-----|
| 1  | `c2RaytoCircle` | `disc < 0` (line 117): `b*b - c < 0`, i.e. ray line misses the circle entirely | `return 0`, `*out` **left untouched** | [x] |
| 2  | `c2RaytoCircle` | `t < 0` (line 120 first conjunct false): nearest root behind the ray origin (origin strictly inside, or circle behind) | `return 0`, `*out` untouched | [x] |
| 3  | `c2RaytoCircle` | `t > A.t` (line 120 second conjunct false): hit exists but beyond the ray's length | `return 0`, `*out` untouched | [x] |
| 4  | `c2RaytoCircle` | `disc` is NaN (e.g. `A.p`/`B.p`/`B.r` contains NaN or ±inf giving inf−inf) ⇒ `disc < 0` false, then `t` NaN ⇒ `t >= 0` false | `return 0`, `*out` untouched (NaN falls through *both* checks) | [x] |
| 5  | `c2RaytoCircle` | `B.r < 0` (negative radius): `c = dot(m,m) - r*r` uses `r*r ≥ 0`, so a negative radius behaves like `+|r|`; no rejection | may `return 1` — must match bit-exactly | [x] |
| 6  | `c2RaytoCircle` | `B.r == 0` and `A.p == B.p` ⇒ `t == 0`, `impact == p`, `c2Norm(0,0)` = `0 * (1/0)` = NaN | `return 1` with `out->n = (NaN, NaN)`, `out->t = 0` | [x] |
| 7  | `c2AABBtoAABB` | `B.max.x < A.min.x` (d0, line 130) | `return 0` | [x] |
| 8  | `c2AABBtoAABB` | `A.max.x < B.min.x` (d1, line 131) | `return 0` | [x] |
| 9  | `c2AABBtoAABB` | `B.max.y < A.min.y` (d2, line 132) | `return 0` | [x] |
| 10 | `c2AABBtoAABB` | `A.max.y < B.min.y` (d3, line 133) | `return 0` | [x] |
| 11 | `c2AABBtoAABB` | any coordinate NaN ⇒ all four `<` false ⇒ `!(0)` | `return 1` (NaN reports **overlap**) | [x] |
| 12 | `c2AABBtoAABB` | inverted box (`min > max`) — no validation at all | whatever the four `<` yield; must match | [x] |
| 13 | `c2AABBtoPoint` | `B.x < A.min.x` (d0, line 235) | `return 0` | [x] |
| 14 | `c2AABBtoPoint` | `B.y < A.min.y` (d1, line 236) | `return 0` | [x] |
| 15 | `c2AABBtoPoint` | `B.x > A.max.x` (d2, line 237) | `return 0` | [x] |
| 16 | `c2AABBtoPoint` | `B.y > A.max.y` (d3, line 238) | `return 0` | [x] |
| 17 | `c2AABBtoPoint` | `B` contains NaN ⇒ all four comparisons false | `return 1` | [x] |
| 18 | `c2CircleToPoint` | `d2 >= A.r * A.r` (line 245) — point on or outside the circle (note: **strict** `<`, so a point exactly on the rim is a miss) | `return 0` | [x] |
| 19 | `c2CircleToPoint` | `A.r == 0` ⇒ `d2 < 0` impossible | `return 0` always | [x] |
| 20 | `c2CircleToPoint` | `A.r` or a coordinate NaN ⇒ `d2 < NaN` false | `return 0` | [x] |
| 21 | `c2RaytoAABB` | `!c2AABBtoAABB(a_box, B)` (line 162) — the ray's own AABB does not overlap `B` | `return 0`, `*out` untouched | [x] |
| 22 | `c2RaytoAABB` | `d > 0` (line 173) — SAT on the ray's normal separates the segment from the box | `return 0`, `*out` untouched | [x] |
| 23 | `c2RaytoAABB` | `hit == 0` (line 191/211) — all four `t0..t3 > 1.0` | `return 0`, `*out` untouched | [x] |
| 24 | `c2RaytoAABB` | `A.t == 0` (degenerate zero-length ray) ⇒ `p1 == p0`, `ab == (0,0)`, `n == (0,0)`, `d = 0 - dot(0,he)` | no rejection from `d`; behaviour must match exactly | [x] |
| 25 | `c2RaytoAABB` | `A.t < 0` (negative ray length) — never validated | must match exactly | [x] |
| 26 | `c2RaytoAABB` | `A.t == +inf` / `A.d` = `(0,0)` ⇒ `p1` has NaN ⇒ `c2Minv/c2Maxv` NaN ternaries ⇒ propagates through `c2AABBtoAABB` (row 11) | must match exactly | [x] |
| 27 | `c2RaytoAABB` | degenerate box `B.min == B.max` ⇒ `half_extents == (0,0)` | must match exactly | [x] |
| 28 | `c2RaytoAABB` | inverted box `B.min > B.max` ⇒ negative `half_extents` | must match exactly | [x] |
| 29 | `c2RayToPlane_OneDimensional` (static, via `c2RaytoAABB`) | `da < 0` (line 143) | returns `0` (⇒ that axis' `t` is 0, i.e. `hit` is true for it) | [x] |
| 30 | `c2RayToPlane_OneDimensional` (static) | `da * db > 0` (line 145) — both endpoints on the same side | returns `1.0f` (`t <= 1.0f` ⇒ still counted as `hit`) | [x] |
| 31 | `c2RayToPlane_OneDimensional` (static) | `d == da - db == 0` (line 149 false) — parallel/degenerate | returns `0` | [x] |
| 32 | `c2RayToPlane_OneDimensional` (static) | `da` NaN ⇒ `da<0` false, `da*db>0` false, `d` NaN ≠ 0 ⇒ `NaN/NaN` | returns NaN ⇒ `t <= 1.0f` false ⇒ that axis not `hit` | [x] |
| 33 | `c2RaytoCapsule` | falls off the end (line 308): `yAe.x*yAp.x >= 0` **and** `min(|yAe.x|,|yAp.x|) >= B.r` | `return 0`, but `*out` **has already been written** with `n = c2Norm(b-a)`, `t = 0` (lines 260–261). This write-before-reject is essential to replicate. | [x] |
| 34 | `c2RaytoCapsule` | `B.a == B.b` (degenerate zero-length capsule) ⇒ `c2Norm((0,0))` = `(NaN,NaN)` ⇒ `M` all NaN ⇒ every derived value NaN | `out->n = (NaN,NaN)`, `out->t = 0`; return value determined by NaN comparisons | [x] |
| 35 | `c2RaytoCapsule` | `B.r == 0` ⇒ `capsule_bb = {(0,0),(0,yBb.y)}`; `c2CircleToPoint` always false (row 19); `min(...) < 0` false | must match exactly | [x] |
| 36 | `c2RaytoCapsule` | `B.r < 0` ⇒ `capsule_bb.min.x = -r > 0 > r = capsule_bb.max.x` (inverted!) | must match exactly | [x] |
| 37 | `c2RaytoCapsule` | side-plane branch with `d = yAe.x - yAp.x == 0` (line 294) ⇒ `t = (c - yAp.x)/0` = ±inf or NaN | must match exactly (division by zero, no check) | [x] |
| 38 | `c2RaytoCapsule` | delegating branch: `\|yAp.x\| < B.r` and `yAp.y < 0` ⇒ tail-calls `c2RaytoCircle(A, Ca, out)` and inherits **its** rejections (rows 1–4) | `return` whatever `c2RaytoCircle` returns | [x] |
| 39 | `c2RaytoCapsule` | delegating branch: `y <= 0` ⇒ `c2RaytoCircle(A, Ca, out)`; `y >= yBb.y` ⇒ `c2RaytoCircle(A, Cb, out)` | inherited rejection | [x] |
| 40 | `c2RaytoPoly` | `den == 0 && num < 0` (line 347) — ray parallel to a plane and origin outside it | `return 0`, `*out` untouched | [x] |
| 41 | `c2RaytoPoly` | `hi < lo` (line 356) — interval collapsed; checked **every iteration, including i == 0** | `return 0`, `*out` untouched | [x] |
| 42 | `c2RaytoPoly` | `index == ~0` after the loop (line 359) — no back-facing plane ever tightened `lo` (ray origin already inside, or `count <= 0`) | `return 0`, `*out` untouched | [x] |
| 43 | `c2RaytoPoly` | `B->count == 0` ⇒ loop body never runs ⇒ `index == ~0` | `return 0`, `*out` untouched (safe even with `out == NULL`) | [x] |
| 44 | `c2RaytoPoly` | `B->count < 0` (e.g. `-1`, `INT_MIN`) ⇒ `0 < count` false ⇒ loop never runs. **No validation.** | `return 0`, `*out` untouched | [x] |
| 45 | `c2RaytoPoly` | `B->count > 8` (9..16) — reads **past** `verts[8]`/`norms[8]`. No bounds check exists; `verts[8+k]` aliases `norms[k]` and `norms[8+k]` reads past the struct. Out-of-range-index behaviour must match. | reads adjacent bytes; result must be bit-identical for identical backing memory | [x] |
| 46 | `c2RaytoPoly` | `bx_ptr == NULL` (line 338) — the file's only null check; means "use `c2xIdentity()`" | not an error: identity transform | [x] |
| 47 | `c2RaytoPoly` | `A.t < 0` ⇒ `hi = A.t < lo = 0` ⇒ `hi < lo` on the very first iteration (row 41), *unless* `count <= 0` | `return 0` | [x] |
| 48 | `c2RaytoPoly` | `bx.r` non-normalised / zero (`c = s = 0`) ⇒ `p` and `d` collapse to `(0,0)`; degenerate but unvalidated | must match exactly | [x] |
| 49 | `c2RaytoPoly` | NaN in `A`, `B->verts`, `B->norms`, or `bx` ⇒ `den == 0` false, `num < 0` false, `den < 0` false, `den > 0` false, `hi < lo` false ⇒ loop completes with `index == ~0` | `return 0` | [x] |
| 50 | `c2CastRay` | `typeB` out of range — **any** `int` with no matching `case`: `-1`, `4`, `5`, `100`, `INT_MIN`, `INT_MAX`. C enums accept any `int` across the FFI boundary; the `switch` has no `default`, so control reaches line 378. | `return 0`, `*out` untouched, `B` **never dereferenced** (safe with `B == NULL`) | [x] |
| 51 | `c2CastRay` | `typeB == C2_TYPE_CIRCLE/AABB/CAPSULE` with `bx != NULL` — `bx` is silently **ignored** for these three types (only `C2_TYPE_POLY` forwards it) | rejection/hit identical to calling the underlying fn with no transform | [x] |
| 52 | `c2CastRay` | `typeB == C2_TYPE_POLY` inherits every `c2RaytoPoly` rejection (rows 40–49) | inherited | [x] |
| 53 | `c2Div` / `c2Norm` | `b == 0` in `c2Div` (line 83): `1.0f/0` = ±inf, then `a * inf`. `c2Norm` of the zero vector ⇒ `c2Len == 0` ⇒ `0 * inf` = NaN. **No zero check.** | `(±inf,±inf)` or `(NaN,NaN)`, never an error | [x] |
| 54 | `c2Div` / `c2Norm` | `b == -0.0` ⇒ `1.0f/-0.0` = `-inf` (sign matters) | signed infinities must match | [x] |
| 55 | `c2Len` | `c2Dot(a,a) < 0` impossible for finite `a`, but `a` = `(inf, NaN)` ⇒ `sqrtf(NaN)` = NaN; `a` huge ⇒ `dot` overflows to `+inf` ⇒ `sqrtf(inf)` = `inf` | NaN / inf, no error | [x] |
| 56 | `c2Minv`/`c2Maxv`/`c2Absv` | NaN operand: the C uses raw ternaries, **not** `fminf`/`fmaxf`/`fabsf`, so `c2Minv(NaN, 1)` = `1` but `c2Minv(1, NaN)` = `NaN`, and `c2Absv(NaN)` keeps the sign bit. Using Rust's `f32::min/max/abs` here would be a real bug. | asymmetric NaN selection; `-0.0` maps to `-0.0` in `c2Absv` | [x] |
| 57 | `poly_ray` | `cast1 == NULL` or `cast2 == NULL` — no check; both rays *miss* in the hard-coded scenario? (see CONFIGS row 40) | governed by whether the fixed scenario hits; tested with valid pointers | [x] |

## Not differentially testable (C dereferences an invalid pointer ⇒ UB/SIGSEGV in both)

Documented for completeness; deliberately **not** turned into tests, because the
C's behaviour is a crash rather than a value:

| function | invalid input | why untestable |
|----------|---------------|----------------|
| `c2RaytoPoly` | `B == NULL` | line 344 reads `B->count` unconditionally ⇒ SIGSEGV |
| `c2CastRay` | `B == NULL` with a *valid* `typeB` | lines 370/372/374 dereference `B`; line 376 passes it to `c2RaytoPoly` ⇒ SIGSEGV |
| `c2RaytoCapsule` | `out == NULL` | lines 260–261 write `out->n`/`out->t` **unconditionally** ⇒ SIGSEGV for every input |
| `c2RaytoCircle` / `c2RaytoAABB` / `c2RaytoPoly` | `out == NULL` on a **hit** | `out->t` written ⇒ SIGSEGV |
| `c2RaytoPoly` | `bx_ptr` non-null but dangling | line 338 dereferences ⇒ SIGSEGV |

The **safe** subset of null-pointer behaviour *is* tested, because in each of
these cases the C provably returns before dereferencing anything:

| test | null argument | why it is safe |
|------|---------------|----------------|
| `err01` | `out` on `c2RaytoCircle` with `disc < 0` | returns at line 118 |
| `err21` | `out` on `c2RaytoAABB` with a non-overlapping ray AABB | returns at line 163 |
| `err40` | `out` on `c2RaytoPoly` with `den == 0 && num < 0` | returns at line 348 |
| `err43`/`err44` | `out` on `c2RaytoPoly` with `count <= 0` | the loop body never runs |
| `err50` | `B` **and** `out` on `c2CastRay` with an out-of-range `typeB` | no `case` matches, so neither is read |
| `err57` | both `cast1` and `cast2` on `poly_ray` | both hard-coded rays miss, so neither out pointer is written (asserted as a precondition of the test) |

## Where each row is tested

Every row above has a corresponding `#[test]` in `tests/phase_c_errors.rs`,
named `errNN_...`. 44 tests cover the 57 rows (some tests cover a group of
closely-related rows, e.g. `err07_to_err10_aabbtoaabb_each_separating_axis`
covers one row per separating axis and `err13_to_err16_aabbtopoint_each_axis`
likewise).

The tests assert more than "both failed somehow":

* the **exact** sentinel (`0` vs `1`), not just "falsy";
* that `*out` is **byte-for-byte pristine** (pre-poisoned with `0xA5`) on the
  rejection paths where the C returns before writing — and, for `ERRORS.md`
  row 33, that `*out` *has* been written before the rejection, which is the
  opposite expectation and easy to get wrong;
* the documented *shape* of degenerate results where one exists, e.g. row 6
  asserts `out->t == -0.0` (from `-b - sqrt(0)` with `b == +0.0`, so the sign
  bit is set) and `out->n == (NaN, NaN)`.

`mutation_check.sh` confirms these assertions have teeth: flipping
`disc < 0` to `disc <= 0`, `t <= A.t` to `t < A.t`, `hi < lo` to `hi <= lo`,
the `~0` index sentinel to `0`, `c2CircleToPoint`'s strict `<` to `<=`, making
`c2CastRay` accept an out-of-range enum value, clamping `count` to 8, rejecting
a `NULL` `bx`, and dropping `c2RaytoCapsule`'s write-before-reject are ALL
caught.
