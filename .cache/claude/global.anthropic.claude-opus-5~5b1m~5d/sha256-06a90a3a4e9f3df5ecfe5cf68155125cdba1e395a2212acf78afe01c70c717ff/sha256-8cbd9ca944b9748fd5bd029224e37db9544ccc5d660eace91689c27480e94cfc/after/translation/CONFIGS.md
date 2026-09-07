# CONFIGS.md — Phase B configuration surface table

## Axes the C code actually branches on

Derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

**A1 — runtime option / mode.** The library has exactly **one** runtime
"option": the `C2_TYPE typeB` shape selector consumed by `c2CastRay`'s `switch`
(`C2_TYPE_CIRCLE=0`, `C2_TYPE_AABB=1`, `C2_TYPE_CAPSULE=2`). There are no
`#ifdef`s, no global state, no init/config functions, no byte-order handling.
`spec_ray` hard-codes `C2_TYPE_CIRCLE`.

**A2 — float value class (per scalar).** Every branch in the library is a float
comparison, so the value class of each input is a real axis:
`normal`, `+0.0`, `-0.0`, `denormal`, `+inf`, `-inf`, `NaN`, `huge (1e30±)`,
`tiny (1e-30±)`, `exactly-representable-integer`, and `equal operands`
(needed for the `d != 0`, `t0 >= t1`, `y >= yBb.y`, `t <= A.t`,
`d2 < r*r` ties).

**A3 — argument-passing shape (ABI).** Struct sizes select different SysV
classes, and every public function must be driven through the `.so` for the
class to be exercised: `c2v` 8 B (one SSE eightbyte), `c2Circle`/`c2Raycast`
12 B (SSE,SSE), `c2AABB`/`c2m` 16 B (SSE,SSE), `c2Ray`/`c2Capsule` 20 B →
**MEMORY** (passed on the stack). `c2CastRay` mixes a stack-passed `c2Ray` with
three register args.

**A4 — geometric configuration** (the shape-specific branch structure):
sign of `disc`, sign of `t` vs `A.t`, which of the 4 AABB separating tests
fires, which of `t0..t3` wins the 4-way `>=` chain, which capsule region the
ray origin lies in, which end cap the delegation picks, degenerate
(zero-length / zero-radius / inverted / coincident) shapes.

**A5 — out-parameter state.** `*out` is pre-filled with a sentinel before each
call so that "did the callee write it?" is itself part of the compared output
(the C code leaves `*out` untouched on the miss paths of `c2RaytoCircle` /
`c2RaytoAABB` but *always* writes it in `c2RaytoCapsule`).

Every row below is compared **bit-for-bit** (`f32::to_bits`, `c_int`) between
the C `.so` and the Rust `.so`, both loaded with `libloading`, over **many
randomized inputs** from a fixed-seed xorshift64\* PRNG (seed `0x2545F4914F6CDD1D`).

---

## Rows

### Level 0 — leaf vector helpers (12-symbol group, `c2v`/scalar ABI)

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|-------------------------------------------|----|
| 1 | `c2V` | random `f32` bit-patterns (all classes incl. NaN payloads, ±0, denormals) | ✅ |
| 2 | `c2V` | the exact constant set the library itself uses: `(-1,0) (1,0) (0,-1) (0,1) (-r,0) (r,yBb.y)` | ✅ |
| 3 | `c2Dot` | both vectors random normals (finite, moderate magnitude) | ✅ |
| 4 | `c2Dot` | mixed classes: one component ±0 / denormal / ±inf, cancellation cases (`a.x*b.x == -(a.y*b.y)`) | ✅ |
| 5 | `c2Dot` | overflow to ±inf (`1e30 · 1e30`) and `inf·0` → NaN | ✅ |
| 6 | `c2Len` | random finite vectors; perfect squares (3,4)→5; ±0 → 0 | ✅ |
| 7 | `c2Len` | `(0,0)` → 0; components ±inf → inf; denormal-only vector (underflow of the dot) | ✅ |
| 8 | `c2Add` / `c2Sub` | random finite pairs, plus ±0 combinations (`+0 + -0`, `-0 - -0` — sign-of-zero matters) | ✅ |
| 9 | `c2Add` / `c2Sub` | `inf - inf` → NaN, `inf + inf` → inf, overflow to inf | ✅ |
| 10 | `c2Mulvs` | random vector × random scalar; scalar = `±0`, `±1`, `0.5f`, denormal, `±inf`, NaN | ✅ |
| 11 | `c2Div` | random vector, scalar random; **`b == ±0`** (→ `a*±inf`), `b == ±inf` (→ `a*0`), `b` denormal (→ `1/b` overflows to inf) | ✅ |
| 12 | `c2Div` | `b` such that `1.0f/b` rounds (e.g. `b = 3.0f`) — verifies reciprocal-multiply, **not** division | ✅ |
| 13 | `c2Norm` | random finite vectors (unit, huge, tiny) | ✅ |
| 14 | `c2Norm` | `(0,0)` → `(NaN,NaN)`; components ±inf; vector whose dot overflows (`1e30,1e30`); denormal vector | ✅ |
| 15 | `c2Minv` / `c2Maxv` | random pairs; **equal components** (ternary picks the `else` branch); `+0` vs `-0` (ternary keeps `b`, unlike `fminf`) | ✅ |
| 16 | `c2Minv` / `c2Maxv` | one/both components NaN — ternary result differs from `fminf`/`fmaxf`, so this row pins the exact C semantics | ✅ |
| 17 | `c2Skew` / `c2CCW90` | random vectors; ±0 (negation of zero flips the sign bit); NaN (sign-bit flip of a NaN) | ✅ |
| 18 | `c2Absv` | random vectors; `-0.0` (ternary: `-0 < 0` is false ⇒ **`-0.0` is returned unchanged**, unlike `fabsf`); `-NaN`; `-inf` | ✅ |
| 19 | `c2MulmvT` | random `c2m` × random `c2v` (16-B SSE,SSE struct arg) | ✅ |
| 20 | `c2MulmvT` | `c2m` = the NaN matrix produced by a degenerate `c2Norm`, and the rotation matrix `{CCW90(y), y}` actually built by `c2RaytoCapsule` | ✅ |

### Level 1 — boolean overlap predicates

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|-------------------------------------------|----|
| 21 | `c2AABBtoAABB` | random well-formed boxes, overlapping | ✅ |
| 22 | `c2AABBtoAABB` | random boxes, disjoint along each of the 4 axes individually and in combination | ✅ |
| 23 | `c2AABBtoAABB` | edge-touching (`A.max.x == B.min.x`) and corner-touching — the `<` is strict | ✅ |
| 24 | `c2AABBtoAABB` | one box fully inside the other; zero-area box (`min == max`); inverted box (`min > max`) | ✅ |
| 25 | `c2AABBtoAABB` | NaN / ±inf coordinates (all-`<`-false ⇒ returns 1) | ✅ |
| 26 | `c2AABBtoPoint` | random point vs random box: inside, outside on each of the 4 sides, exactly on each edge/corner | ✅ |
| 27 | `c2AABBtoPoint` | zero-area box + point equal to it; inverted box; the `{(-r,0),(r,yBb.y)}` capsule slab shape incl. `yBb.y < 0`; NaN/±inf point | ✅ |
| 28 | `c2CircleToPoint` | random point vs random circle: inside, outside, **exactly on the rim** (`d2 == r*r`, strict `<` ⇒ 0) | ✅ |
| 29 | `c2CircleToPoint` | `r == 0`, `r < 0`, `r == inf`, `r` NaN, `r` denormal (so `r*r` underflows to 0); point == centre | ✅ |

### Level 2 — raycasts (each called directly through the `.so`, `c2Ray` on the stack)

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|-------------------------------------------|----|
| 30 | `c2RaytoCircle` | random normalized-`d` rays vs random circles — mixed hit/miss (broad property sweep) | ✅ |
| 31 | `c2RaytoCircle` | guaranteed **hit**: ray aimed at the centre, `A.t` long enough (exercises the `out->t`/`out->n` write and `c2Norm` of the impact normal) | ✅ |
| 32 | `c2RaytoCircle` | guaranteed **miss** via `disc < 0`; via `t < 0` (origin past the circle); via `t > A.t` | ✅ |
| 33 | `c2RaytoCircle` | tangent (`disc == 0`), origin exactly on the rim (`t == 0`), impact exactly at `t == A.t` (boundary of `<=`) | ✅ |
| 34 | `c2RaytoCircle` | origin **inside** the circle (`c < 0` ⇒ `t < 0` ⇒ miss) | ✅ |
| 35 | `c2RaytoCircle` | non-unit `A.d` (the function never re-normalizes — arbitrary `d` is a valid input), `A.d == (0,0)`, `A.t == 0`, `A.t == inf` | ✅ |
| 36 | `c2RaytoCircle` | `r == 0` / `r < 0` / huge `r` / degenerate: impact `== B.p` ⇒ `c2Norm((0,0))` = `(NaN,NaN)` written to `out->n` while returning **1** | ✅ |
| 37 | `c2RaytoAABB` | random rays vs random boxes — broad property sweep, mixed hit/miss | ✅ |
| 38 | `c2RaytoAABB` | hit resolved by each of the 4 branches of the `t0/t1/t2/t3 >=` chain: `-x` face, `+x` face, `-y` face, `+y` face | ✅ |
| 39 | `c2RaytoAABB` | hit where **several `t` are tied** (e.g. all zero, or corner-exact) — pins the first-wins order of the `>=` chain | ✅ |
| 40 | `c2RaytoAABB` | miss via the swept-box `c2AABBtoAABB` reject; miss via `d > 0` (SAT); miss via `hit == 0` | ✅ |
| 41 | `c2RaytoAABB` | axis-aligned rays (`ab.x == 0` ⇒ `n = (0, 0)`-ish, `da == db` ⇒ the `d != 0` guard fires) | ✅ |
| 42 | `c2RaytoAABB` | ray origin **inside** the box; ray exactly along a face; `A.t == 0`; `A.t == inf`; `A.d == (0,0)` | ✅ |
| 43 | `c2RaytoAABB` | zero-area box (`min == max`), inverted box, huge box (`±1e30` ⇒ half-extents overflow), NaN in `A.d`/`A.t`/box | ✅ |
| 44 | `c2RaytoCapsule` | random rays vs random capsules — broad property sweep | ✅ |
| 45 | `c2RaytoCapsule` | origin **inside the slab** (`c2AABBtoPoint(capsule_bb, yAp)` ⇒ early `return 1`, `t = 0`) | ✅ |
| 46 | `c2RaytoCapsule` | origin inside end cap `a`; origin inside end cap `b` (the two `c2CircleToPoint` early returns) | ✅ |
| 47 | `c2RaytoCapsule` | side-wall hit: `|yAp.x| >= B.r`, `0 < y < yBb.y` ⇒ `out->n = M.x` (`c > 0`) | ✅ |
| 48 | `c2RaytoCapsule` | side-wall hit on the other side ⇒ `out->n = c2Skew(M.y)` (`c < 0`) | ✅ |
| 49 | `c2RaytoCapsule` | cap delegation `y <= 0` ⇒ `c2RaytoCircle(A, Ca, out)`; `y >= yBb.y` ⇒ `c2RaytoCircle(A, Cb, out)` (both the returns-1 and the returns-0 sub-cases) | ✅ |
| 50 | `c2RaytoCapsule` | `|yAp.x| < B.r` branch: `yAp.y < 0` ⇒ cap `a`, else cap `b` | ✅ |
| 51 | `c2RaytoCapsule` | full miss (`return 0`) — asserts `*out` **was still overwritten** with `n=normalize(b-a)`, `t=0` | ✅ |
| 52 | `c2RaytoCapsule` | capsule with `b` "below" `a` (`yBb.y < 0` ⇒ inverted slab), horizontal capsule, unit capsule | ✅ |
| 53 | `c2RaytoCapsule` | degenerate `B.a == B.b` (NaN matrix), `B.r == 0`, `B.r < 0`, `B.r == inf`, `yAe.x == yAp.x` (division by zero in the `t` formula) | ✅ |

### Level 3 — the dispatcher (option axis A1 × every shape)

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|-------------------------------------------|----|
| 54 | `c2CastRay` | `typeB = C2_TYPE_CIRCLE (0)`, random rays/circles — must equal a direct `c2RaytoCircle` call | ✅ |
| 55 | `c2CastRay` | `typeB = C2_TYPE_AABB (1)`, random rays/boxes — must equal a direct `c2RaytoAABB` call | ✅ |
| 56 | `c2CastRay` | `typeB = C2_TYPE_CAPSULE (2)`, random rays/capsules — must equal a direct `c2RaytoCapsule` call | ✅ |
| 57 | `c2CastRay` | each `typeB` on the **miss** path (verifies `*out` untouched propagates through the dispatcher) | ✅ |
| 58 | `c2CastRay` | mixed 20-B-stack `c2Ray` + register args across many randomized calls (ABI stress, incl. NaN/inf rays) | ✅ |

### Level 4 — the public header entry point

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|-------------------------------------------|----|
| 59 | `spec_ray` | random finite `(mp, c.p, c.r, ray.p)` in a moderate range — mixed hit/miss property sweep | ✅ |
| 60 | `spec_ray` | guaranteed hit: `mp` = circle centre, `ray.p` outside, `c_r` large enough | ✅ |
| 61 | `spec_ray` | guaranteed miss: `mp` aimed away from the circle; circle behind the ray origin | ✅ |
| 62 | `spec_ray` | `ray.p` **inside** the circle (`t < 0` ⇒ miss, `*cast` untouched) | ✅ |
| 63 | `spec_ray` | impact exactly at `t == ray.t` (`mp` on the far rim — boundary of `t <= A.t`) | ✅ |
| 64 | `spec_ray` | `mp == ray.p` ⇒ `c2Norm((0,0))` ⇒ NaN direction and NaN `ray.t` | ✅ |
| 65 | `spec_ray` | `c_r == 0`, `c_r < 0`, `c_r == inf`, `c_r` denormal | ✅ |
| 66 | `spec_ray` | any argument ±inf / NaN / ±0 / denormal / huge (random bit-pattern sweep over all 7 floats) | ✅ |
| 67 | `spec_ray` | integer-valued coordinates (the typical mouse-picking use case: `mp`, `ray.p` on a pixel grid) | ✅ |
| 68 | `spec_ray` | must equal the manual composition `c2Norm`+`c2Dot`+`c2CastRay(…,CIRCLE,…)` driven through the `.so` — verifies the composed pipeline, not just the wrapper | ✅ |

---

## Not applicable

* **Binary/driver executable:** `c_src/CMakeLists.txt` only does
  `add_library(${project_name} SHARED src/lib.c)`. No executable is produced,
  so there is no stdout to compare.
* **Feature combinations:** `translation/Cargo.toml` has no `[features]`
  section, so the default build is the only configuration
  (`--no-default-features` is equivalent and is exercised by
  `verify_all_features.sh`).
