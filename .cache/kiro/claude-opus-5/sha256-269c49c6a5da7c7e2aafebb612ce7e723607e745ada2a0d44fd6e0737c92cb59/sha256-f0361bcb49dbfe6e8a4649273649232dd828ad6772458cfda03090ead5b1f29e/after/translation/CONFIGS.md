# CONFIGS.md — Phase B configuration / valid-input surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C actually branches on

There are **no** compile-time options: no `#ifdef`, no `#if`, no build flags in
`CMakeLists.txt` beyond `-lm`, and `translation/Cargo.toml` declares **no
`[features]` section** — so the only cargo feature combination is the default
(empty) one. All variation is runtime data.

**Axis 1 — entry point.** All 22 exported functions, including the lowest-level
vector primitives, not just `gen_ray`:
* pure vector math: `c2V`, `c2Dot`, `c2Len`, `c2Add`, `c2Sub`, `c2Mulvs`,
  `c2Div`, `c2Norm`, `c2Minv`, `c2Maxv`, `c2Skew`, `c2Absv`, `c2CCW90`,
  `c2MulmvT`
* predicates: `c2AABBtoAABB`, `c2AABBtoPoint`, `c2CircleToPoint`
* raycasts: `c2RaytoCircle`, `c2RaytoAABB`, `c2RaytoCapsule`
* dispatcher: `c2CastRay` (the "mode" selector)
* one-shot wrapper: `gen_ray`

**Axis 2 — the one runtime mode flag: `C2_TYPE typeB` of `c2CastRay`**, with the
three valid variants `C2_TYPE_CIRCLE=0`, `C2_TYPE_AABB=1`, `C2_TYPE_CAPSULE=2`,
each selecting a completely different shape struct read out of `const void *B`.

**Axis 3 — input shape / value class** (the classes the code's comparisons,
`sqrtf`, division and ternary idioms distinguish):
`normal finite` · `zero (0.0)` · `negative zero (-0.0)` · `denormal` ·
`huge (near FLT_MAX)` · `tiny` · `+inf` · `-inf` · `quiet NaN (both sign bits)`

**Axis 4 — geometric configuration** the raycast branches take:
ray length `A.t` `>0` / `==0` / `<0`; ray direction normalized vs not;
origin inside / on / outside the shape; hit at `t==0` / `t==A.t` exactly;
tangent; radius `>0` / `==0` / `<0`; AABB proper / flat (min==max on one axis) /
inverted (min>max); capsule proper / degenerate (`a==b`) / axis-aligned /
diagonal; each of the four AABB face normals as the winner; each capsule branch
(interior box, end-cap circle A, end-cap circle B, side slab).

## Rows

Each row is run with **many randomized inputs (fixed seed)** and compared
bit-for-bit (`to_bits()` on every returned float, plus the `int` return, plus
every field the callee wrote into `*out`).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random finite f32 pairs from the full bit space + all special classes | [x] |
| 2 | `c2Dot` | both vectors random finite; asserts operand-order/rounding parity | [x] |
| 3 | `c2Dot` | one/both operands from the special set (±0, ±inf, NaN both signs, denormal, huge) — cross product | [x] |
| 4 | `c2Len` | random finite; then negative-`dot` impossible, but huge inputs → `+inf` via overflow; NaN/inf input | [x] |
| 5 | `c2Add` | random finite; then `inf + -inf`, `NaN + finite`, `finite + NaN`, `-0.0 + 0.0` | [x] |
| 6 | `c2Sub` | random finite; then `inf - inf`, NaN on either side, `0.0 - 0.0`, `-0.0 - 0.0` | [x] |
| 7 | `c2Mulvs` | random vector × random scalar; then `0 * inf`, `NaN * 0`, denormal × huge (underflow/overflow) | [x] |
| 8 | `c2Div` | random vector ÷ random scalar (reciprocal-then-multiply path); `b == 0`, `b == -0.0`, `b == inf`, `b == NaN`, `b` denormal (`1/b` overflows) | [x] |
| 9 | `c2Norm` | random finite vectors; zero vector; near-zero denormal vector; huge vector (`c2Len` → inf); NaN/inf components | [x] |
| 10 | `c2Minv` / `c2Maxv` | random pairs; equal values; `+0.0` vs `-0.0` (ternary keeps `b`); NaN in a / in b / in both — the ternary idiom differs from `f32::min`/`max` | [x] |
| 11 | `c2Skew` / `c2CCW90` | random finite; `±0.0` (negation flips the sign of zero); NaN (negation flips the NaN sign bit); ±inf | [x] |
| 12 | `c2Absv` | random finite; `-0.0` (C's `a<0?-a:a` returns `-0.0`, unlike `fabsf`); NaN both sign bits; ±inf | [x] |
| 13 | `c2MulmvT` | random `c2m` × random `c2v`; identity/rotation matrices; NaN & inf rows; the all-NaN matrix produced by a degenerate capsule | [x] |
| 14 | `c2AABBtoAABB` | random overlapping boxes | [x] |
| 15 | `c2AABBtoAABB` | each of the four separation directions forced individually (-x, +x, -y, +y) | [x] |
| 16 | `c2AABBtoAABB` | touching exactly (`A.max.x == B.min.x`), flat boxes (`min == max`), inverted boxes (`min > max`) | [x] |
| 17 | `c2AABBtoAABB` | NaN coordinates (C accepts, returns 1) | [x] |
| 18 | `c2AABBtoPoint` | point strictly inside / on each of the four edges / on a corner / outside on each side; flat box; inverted box; NaN point | [x] |
| 19 | `c2CircleToPoint` | point inside / exactly on the rim (rejected, `<` not `<=`) / outside; `r == 0`; `r < 0`; NaN; huge `r` where `r*r` overflows to `inf` | [x] |
| 20 | `c2RaytoCircle` | direct low-level call: random normalized ray, random circle, `A.t` random positive — full mixture of hit/miss | [x] |
| 21 | `c2RaytoCircle` | ray direction NOT normalized (only reachable through the low-level entry point; `gen_ray` always normalizes) — random `d` magnitude in `[1e-3, 1e3]` | [x] |
| 22 | `c2RaytoCircle` | ray origin strictly inside the circle (`t` negative → reject) | [x] |
| 23 | `c2RaytoCircle` | grazing / tangent rays: `disc` within a few ULP of 0 | [x] |
| 24 | `c2RaytoCircle` | hit exactly at the far end (`t == A.t`) and exactly at the origin (`t == 0`) | [x] |
| 25 | `c2RaytoCircle` | `A.t == 0`, `A.t < 0`, `A.t == inf` | [x] |
| 26 | `c2RaytoCircle` | `r == 0` (point circle), `r < 0`, `r == inf`; special-class components | [x] |
| 27 | `c2RaytoAABB` | direct low-level call: random ray (normalized) vs random box, mixed hit/miss | [x] |
| 28 | `c2RaytoAABB` | ray direction not normalized; `A.t` random | [x] |
| 29 | `c2RaytoAABB` | each of the four face normals forced to win: `(-1,0)`, `(1,0)`, `(0,-1)`, `(0,1)` — axis-aligned rays from each side | [x] |
| 30 | `c2RaytoAABB` | ray origin inside the box | [x] |
| 31 | `c2RaytoAABB` | ray exactly parallel to a face (`da == db`, the `d != 0` false branch), and ray along an edge | [x] |
| 32 | `c2RaytoAABB` | `A.t == 0` (degenerate `p1 == p0`, zero skew normal), `A.t < 0`, `A.t` huge | [x] |
| 33 | `c2RaytoAABB` | flat box (`min.x == max.x` and separately `min.y == max.y`), zero-area box, inverted box | [x] |
| 34 | `c2RaytoAABB` | tie-breaking: two or more `tN` exactly equal so the `>=` chain order matters (corner hits) | [x] |
| 35 | `c2RaytoAABB` | special-class components (±0, ±inf, NaN, denormal, huge) in `A.p`, `A.d`, `A.t`, `B.min`, `B.max` | [x] |
| 36 | `c2RaytoCapsule` | direct low-level call: random capsule (a≠b), random normalized ray, mixed | [x] |
| 37 | `c2RaytoCapsule` | branch: ray origin inside the transformed slab box → early `return 1` with `out = (0, norm(b-a))` | [x] |
| 38 | `c2RaytoCapsule` | branch: origin inside end circle A → early `return 1`; and inside end circle B | [x] |
| 39 | `c2RaytoCapsule` | branch: `|yAp.x| < B.r` with `yAp.y < 0` → delegates to circle A | [x] |
| 40 | `c2RaytoCapsule` | branch: `|yAp.x| < B.r` with `yAp.y >= 0` → delegates to circle B | [x] |
| 41 | `c2RaytoCapsule` | branch: side-slab hit, `yAp.x > 0` → `c = +B.r`, `out->n = M.x` | [x] |
| 42 | `c2RaytoCapsule` | branch: side-slab hit, `yAp.x <= 0` → `c = -B.r`, `out->n = c2Skew(M.y)` | [x] |
| 43 | `c2RaytoCapsule` | branch: `y <= 0` → circle A end cap; and `y >= yBb.y` → circle B end cap | [x] |
| 44 | `c2RaytoCapsule` | capsule orientations: axis-aligned +y, -y (so `yBb.y < 0`), +x, diagonal, very long, very short | [x] |
| 45 | `c2RaytoCapsule` | degenerate capsule `a == b` (all-NaN basis) | [x] |
| 46 | `c2RaytoCapsule` | `B.r == 0`, `B.r < 0` (inverted `capsule_bb`), `B.r` huge | [x] |
| 47 | `c2RaytoCapsule` | ray direction not normalized; `A.t == 0` / `< 0` / huge | [x] |
| 48 | `c2RaytoCapsule` | special-class components in every field | [x] |
| 49 | `c2CastRay` | `typeB = C2_TYPE_CIRCLE (0)`, random `c2Circle` behind `B`, random ray | [x] |
| 50 | `c2CastRay` | `typeB = C2_TYPE_AABB (1)`, random `c2AABB` behind `B`, random ray | [x] |
| 51 | `c2CastRay` | `typeB = C2_TYPE_CAPSULE (2)`, random `c2Capsule` behind `B`, random ray | [x] |
| 52 | `c2CastRay` | same shape memory reinterpreted under each of the 3 valid `typeB` values (checks the cast/ABI reads the same bytes) | [x] |
| 53 | `gen_ray` | full end-to-end pipeline, all 16 floats random finite in a moderate range — the "real consumer" shape | [x] |
| 54 | `gen_ray` | configurations engineered so the returned bitmask takes each of the values 0..7 (circle / capsule / AABB hit combinations) | [x] |
| 55 | `gen_ray` | `mp == ray.p` (zero-length ray → NaN direction) | [x] |
| 56 | `gen_ray` | all-zero arguments | [x] |
| 57 | `gen_ray` | wide-magnitude random arguments (exponents spanning `1e-30 .. 1e30`, random signs) | [x] |
| 58 | `gen_ray` | random arguments drawn from the full f32 bit space including NaN/inf/denormal | [x] |
| 59 | `gen_ray` | `cast1`/`cast2`/`cast3` aliasing the SAME `c2Raycast` buffer (write-order dependent) | [x] |
| 60 | `gen_ray` | out buffers pre-filled with a sentinel pattern, so untouched-on-reject fields are compared too | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the complete set of
combinations is:

| combo | command |
|-------|---------|
| default (= empty) | `cargo test --release` |
| `--no-default-features` (identical, no features exist) | `cargo test --release --no-default-features` |

`run_all.sh` derives this list from `Cargo.toml` (so new features are picked up
automatically) and crosses it with both cargo profiles, because `opt-level`
changes LLVM's instruction selection and is therefore an independent axis for a
bit-exactness claim:

| # | profile | features |
|---|---------|----------|
| 1 | release | default |
| 2 | release | `--no-default-features` |
| 3 | debug   | default |
| 4 | debug   | `--no-default-features` |

All four are green, with the C→Rust `nm -D` symbol diff empty in each.

## Cross-cutting stress

`tests/stress_all.rs` additionally drives all 22 entry points over one shared
randomized corpus (4 value classes: moderate, wide-exponent, special, raw-bit),
300 000 cases × 24 calls per case, verified across 5 independent seeds. This
catches configuration *interactions* the per-row generators keep separate.

