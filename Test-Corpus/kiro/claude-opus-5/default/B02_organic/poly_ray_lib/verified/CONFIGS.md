# CONFIGS.md — Phase A configuration surface table (valid inputs)

Mirror of `ERRORS.md` for VALID inputs. Derived mechanically from
`c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

**A1 — entry point.** All 28 exported symbols (see `SYMBOLS.md`). Four layers:
scalar/vector helpers (`c2V`…`c2Absv`, `c2CCW90`, `c2MulmvT`, `c2Mulrv`,
`c2MulrvT`, `c2MulxvT`, `c2RotIdentity`, `c2xIdentity`), boolean predicates
(`c2AABBtoAABB`, `c2AABBtoPoint`, `c2CircleToPoint`), the four low-level
raycasts (`c2RaytoCircle`, `c2RaytoAABB`, `c2RaytoCapsule`, `c2RaytoPoly`), the
`c2CastRay` dispatcher, and the fixed `poly_ray` driver. Tests drive the
low-level entry points directly, not only `c2CastRay`/`poly_ray`.

**A2 — `C2_TYPE` runtime mode** (the library's only real "option"): the
`typeB` argument selects one of 4 `switch` arms in `c2CastRay` and reinterprets
the `const void *B` payload as a different struct per arm. This is the one
place where a caller-supplied flag changes which code path (and which struct
layout) is used.

**A3 — `bx` transform state** for `c2RaytoPoly` / `c2CastRay`: `NULL`
(→ `c2xIdentity()`), an explicit identity, translation-only, rotation-only,
rotation+translation, and a non-unit/degenerate `c2r`. This toggles
`c2MulxvT`/`c2MulrvT` on the ray and `c2Mulrv` on the output normal.

**A4 — which internal branch of the target raycast is taken.** These are not
caller flags but distinct shapes the code special-cases, and they must each be
hit:
- `c2RaytoAABB`: which of the four `t0..t3` wins the 4-way `>=` chain
  (→ normal `(-1,0)`, `(1,0)`, `(0,-1)`, `(0,1)`), including ties where the
  first matching arm must win.
- `c2RaytoCapsule`: origin inside the capsule's local bb; origin inside end-cap
  A; origin inside end-cap B; `|yAp.x| < B.r` with `yAp.y < 0` (→ cap A) and
  `>= 0` (→ cap B); the side-wall branch with `y <= 0` (→ cap A), `y >= yBb.y`
  (→ cap B), and the true wall hit (`out->n = M.x` vs `c2Skew(M.y)` depending
  on `sign(c)`).
- `c2RaytoPoly`: which `den` sign branch fires per edge (`den<0` lo-clip vs
  `den>0` hi-clip vs `den==0`), and which edge ends up as `index`.

**A5 — input shape / magnitude.** Finite normal, exact zeros, `-0.0`,
denormals, huge (`1e30`), tiny (`1e-30`), `±inf`, NaN, values straddling every
comparison constant in the source (`0`, `1.0f`, `0.5f`, `-1.0f`, `A.t`,
`B.r`, `yBb.y`, `lo`, `hi`).

**A6 — polygon vertex count** (`c2Poly.count`, array capacity 8): `1`, `2`,
`3`, `4`, `5`, `6`, `7`, `8` (and `0` / negative in `ERRORS.md`). Also
`count > 8` reading past the declared arrays — exercised with an oversized
backing buffer so both languages read the *same* bytes.

**A7 — ray parameter `t`**: `0`, small, `1.0`, large, and `t` chosen exactly at
the hit distance (boundary of `t <= A.t`).

**A8 — direction vector `d`**: axis-aligned `±x`/`±y` (many `den == 0` cases),
diagonal, unnormalised, and zero.

## Row table

Every row is exercised against BOTH `.so`s with **many randomized inputs**
(fixed seed, deterministic xorshift PRNG) plus the hand-picked boundary values
named in the row. A row is checked only when all of its randomized cases match
byte-for-byte (`f32::to_bits`, so `-0.0` != `0.0` and NaN payloads compare
exactly).

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `c2V` | random finite pairs + `{0,-0,inf,-inf,NaN,denormal,1e30,1e-30}` cross product | [x] |
| 2  | `c2Dot` | random finite pairs; magnitudes spanning 1e-30…1e30 (catches under/overflow ordering) | [x] |
| 3  | `c2Dot` | non-finite operands: `inf*0`, `inf-inf`, NaN in either slot | [x] |
| 4  | `c2Len` | random finite; plus zero vector, `-0.0` vector, `±inf`, NaN, huge (overflow to inf), denormal | [x] |
| 5  | `c2Add` | random finite; plus `inf + -inf`, `-0.0 + 0.0`, `-0.0 + -0.0` | [x] |
| 6  | `c2Sub` | random finite; plus `inf - inf`, `0.0 - 0.0`, `-0.0 - 0.0` | [x] |
| 7  | `c2Mulvs` | random vector × random scalar; plus `0*inf`, `inf*0`, `-0.0*positive`, `x*NaN` | [x] |
| 8  | `c2Div` | random vector / random scalar; plus `/0`, `/-0`, `/inf`, `/NaN`, `/denormal` (tests the `1/b` then multiply, which is NOT the same as componentwise division) | [x] |
| 9  | `c2Norm` | random finite vectors (unnormalised, all quadrants); plus zero vector, `-0.0` vector, `±inf` components, NaN, denormal (len underflows to 0) | [x] |
| 10 | `c2Minv` | random pairs; plus equal values, `0` vs `-0`, NaN in slot a, NaN in slot b (ternary != `fminf`) | [x] |
| 11 | `c2Maxv` | random pairs; plus equal values, `0` vs `-0`, NaN in slot a, NaN in slot b | [x] |
| 12 | `c2Skew` | random; plus `-0.0` (negation of `-0.0` → `+0.0`), `±inf`, NaN | [x] |
| 13 | `c2Absv` | random; plus `-0.0` (must stay `-0.0`, unlike `fabsf`), `-inf`, NaN, `-NaN` | [x] |
| 14 | `c2CCW90` | random; plus `-0.0` in both slots, `±inf`, NaN | [x] |
| 15 | `c2MulmvT` | random `c2m` × random `c2v`; plus zero matrix, identity, NaN/inf entries | [x] |
| 16 | `c2RotIdentity` | no args — constant result | [x] |
| 17 | `c2xIdentity` | no args — constant result | [x] |
| 18 | `c2Mulrv` | random `c2r` (unit AND non-unit) × random `c2v`; plus zero rot, NaN/inf | [x] |
| 19 | `c2MulrvT` | same as row 18 (transpose path, different sign placement) | [x] |
| 20 | `c2MulxvT` | random `c2x` (translation+rotation) × random `c2v`; plus identity, zero rot, NaN | [x] |
| 21 | `c2AABBtoAABB` | random overlapping boxes | [x] |
| 22 | `c2AABBtoAABB` | random touching boxes (exactly equal edge coords — `<` is strict so touching overlaps) | [x] |
| 23 | `c2AABBtoAABB` | random inverted boxes (`min > max`), plus degenerate zero-area boxes | [x] |
| 24 | `c2AABBtoPoint` | random point inside / on each of the 4 edges / at each of the 4 corners | [x] |
| 25 | `c2CircleToPoint` | random point strictly inside; point exactly on rim (strict `<` → 0); `r` random incl. huge/tiny | [x] |
| 26 | `c2RaytoCircle` | direct call, random ray origin + normalised random direction + random circle, `A.t` random — full hit/miss mix | [x] |
| 27 | `c2RaytoCircle` | ray origin **inside** the circle (`c < 0`, `t < 0` branch) | [x] |
| 28 | `c2RaytoCircle` | `A.t` set exactly to the analytic hit distance (boundary `t <= A.t`) | [x] |
| 29 | `c2RaytoCircle` | unnormalised direction (`|d| != 1`), incl. `d = 0` | [x] |
| 30 | `c2RaytoCircle` | tangent / near-tangent rays (`disc ≈ 0`), grazing a random circle | [x] |
| 31 | `c2RaytoAABB` | direct call, random ray vs random box, full hit/miss mix, `A.t` random | [x] |
| 32 | `c2RaytoAABB` | axis-aligned rays along `+x`, `-x`, `+y`, `-y` through a random box — forces each of the four normal branches and many `da-db == 0` degeneracies | [x] |
| 33 | `c2RaytoAABB` | ray starting **inside** the box | [x] |
| 34 | `c2RaytoAABB` | `A.t == 0` (degenerate zero-length ray) at random origins | [x] |
| 35 | `c2RaytoAABB` | diagonal ray hitting a corner exactly (ties in the 4-way `>=` chain) | [x] |
| 36 | `c2RaytoAABB` | zero-area box (`min == max`) and inverted box, random ray | [x] |
| 37 | `c2RaytoCapsule` | direct call, random capsule (`a`,`b`,`r`) + random ray — full mix | [x] |
| 38 | `c2RaytoCapsule` | ray origin inside the capsule's local bb (`c2AABBtoPoint` early `return 1`) | [x] |
| 39 | `c2RaytoCapsule` | ray origin inside end-cap A, and inside end-cap B (`c2CircleToPoint` early returns) | [x] |
| 40 | `c2RaytoCapsule` | `|yAp.x| < B.r` with `yAp.y < 0` → delegates to circle A; with `yAp.y >= 0` → circle B | [x] |
| 41 | `c2RaytoCapsule` | side-wall branch: `y <= 0` (→ cap A), `y >= yBb.y` (→ cap B), and `0 < y < yBb.y` (true wall hit, `out->n = M.x` for `c>0` / `c2Skew(M.y)` for `c<0`) | [x] |
| 42 | `c2RaytoCapsule` | axis-aligned capsules (`a`,`b` differing in one axis only) crossed by axis-aligned rays | [x] |
| 43 | `c2RaytoCapsule` | degenerate: `a == b` (NaN `M`), `r == 0`, `r < 0`, `A.t == 0` | [x] |
| 44 | `c2RaytoPoly` | direct call, `bx = NULL`, random convex polygon (`count` 3..8, generated as a random convex hull with correctly derived outward normals) + random ray | [x] |
| 45 | `c2RaytoPoly` | direct call, `bx = &identity` — must be bit-identical to row 44's `NULL` result | [x] |
| 46 | `c2RaytoPoly` | `bx` = translation only (`r` identity, `p != 0`) | [x] |
| 47 | `c2RaytoPoly` | `bx` = rotation only (unit `c2r` from a random angle, `p == 0`) | [x] |
| 48 | `c2RaytoPoly` | `bx` = rotation + translation, random | [x] |
| 49 | `c2RaytoPoly` | `bx.r` non-unit / degenerate (`c=s=0`, or `c,s` random unnormalised) | [x] |
| 50 | `c2RaytoPoly` | `count == 1` (single half-plane) with random normal/vert | [x] |
| 51 | `c2RaytoPoly` | `count == 2` (two half-planes, unbounded wedge) | [x] |
| 52 | `c2RaytoPoly` | `count == 3` (triangle) | [x] |
| 53 | `c2RaytoPoly` | `count == 4` (quad — the shape `poly_ray` uses) | [x] |
| 54 | `c2RaytoPoly` | `count == 5,6,7` (mid-range hulls) | [x] |
| 55 | `c2RaytoPoly` | `count == 8` (array capacity boundary) | [x] |
| 56 | `c2RaytoPoly` | `count > 8` (9, 16) with an oversized shared backing buffer — both languages read the same out-of-declared-bounds bytes | [x] |
| 57 | `c2RaytoPoly` | ray origin **inside** the polygon (`lo` stays 0, `index` may stay `~0`) | [x] |
| 58 | `c2RaytoPoly` | ray exactly parallel to an edge (`den == 0`) with `num > 0` (inside) — random polygons | [x] |
| 59 | `c2RaytoPoly` | ray starting exactly on a vertex / on an edge plane (`num == 0`) | [x] |
| 60 | `c2RaytoPoly` | axis-aligned rays (`d = ±x`, `±y`) against axis-aligned quads — many simultaneous `den == 0` | [x] |
| 61 | `c2RaytoPoly` | `A.t` at the exact hit distance (boundary of the `hi` clip) | [x] |
| 62 | `c2RaytoPoly` | non-convex / arbitrary garbage verts+norms (the C never validates convexity) | [x] |
| 63 | `c2CastRay` | `typeB = C2_TYPE_CIRCLE (0)`, `bx = NULL` and `bx = &random`, random circle+ray | [x] |
| 64 | `c2CastRay` | `typeB = C2_TYPE_AABB (1)`, `bx = NULL` and `bx = &random`, random box+ray (note: C **ignores** `bx` for this arm) | [x] |
| 65 | `c2CastRay` | `typeB = C2_TYPE_CAPSULE (2)`, `bx = NULL` and `bx = &random`, random capsule+ray (`bx` ignored) | [x] |
| 66 | `c2CastRay` | `typeB = C2_TYPE_POLY (3)`, `bx = NULL`, random poly+ray | [x] |
| 67 | `c2CastRay` | `typeB = C2_TYPE_POLY (3)`, `bx = &random` transform, random poly+ray | [x] |
| 68 | `c2CastRay` | each `typeB` arm must equal the corresponding direct low-level call bit-for-bit (dispatcher-vs-direct cross-check) | [x] |
| 69 | `poly_ray` | the fixed driver: return value + both `c2Raycast` out-params, bit-exact | [x] |
| 70 | `poly_ray` | called repeatedly and with pre-dirtied out-buffers (checks which fields the C actually writes vs leaves alone) | [x] |

## Status

All 70 rows pass. Every row is exercised in
`translation/tests/phase_b_*.rs`; `tests/nan_storm.rs` and
`tests/targeted_operand_order.rs` add exhaustive pathological-input sweeps on
top. Two branch-coverage guards (`capsule_branch_coverage`,
`poly_branch_coverage`) assert that all 7 exits of `c2RaytoCapsule` and all 4
exits of `c2RaytoPoly` are actually reached, so no row can pass vacuously.

Observed exit histograms (40 000 randomized cases each):

- `c2RaytoCapsule`: `[6758, 6618, 5580, 1080, 7533, 3531, 8900]` — all 7 exits
- `c2RaytoPoly`: `[3938, 18472, 6735, 10855]` — all 4 exits

For the record, `poly_ray` returns **0** and leaves both `c2Raycast`
out-params untouched: both of its hard-coded rays miss the hard-coded
polygon. `tests/phase_b_dispatch.rs` asserts this bit-exactly, including that
pre-existing garbage in the out-buffers survives unmodified.
