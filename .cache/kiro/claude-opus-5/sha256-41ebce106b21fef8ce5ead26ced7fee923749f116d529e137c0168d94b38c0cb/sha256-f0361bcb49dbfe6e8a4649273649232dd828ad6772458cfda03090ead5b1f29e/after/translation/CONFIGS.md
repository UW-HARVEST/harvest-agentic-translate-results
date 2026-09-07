# CONFIGS.md — Phase B configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Derived from the branches the C
actually takes, not from guesses about what matters.

## Axis inventory (mechanically derived from `c_src/src/lib.c`)

**A. Runtime option / mode flags — the only two the public API exposes**

| flag | where | states the C branches on |
|------|-------|--------------------------|
| `C2_TYPE typeB` | `c2CastRay` `switch` | `C2_TYPE_CIRCLE=0`, `C2_TYPE_AABB=1`, `C2_TYPE_CAPSULE=2`, `C2_TYPE_POLY=3` (+ out-of-range → `ERRORS.md` row 58) |
| `const c2x *bx` | `c2RaytoPoly` `bx_ptr ? *bx_ptr : c2xIdentity()` | `NULL` (identity substituted) vs non-`NULL`. Sub-states of a non-null `c2x`: identity, pure translation (`r = {1,0}`), pure rotation (`p = {0,0}`), translation+rotation, non-unit `r`. `bx` is **ignored** for the other three `typeB` values. |

**B. Input shapes the code special-cases**

| axis | distinct values the C distinguishes |
|------|--------------------------------------|
| `c2Poly.count` | `0`, `1`, `2`, `3`, `4` (the `poly_ray` case), `5`, `6`, `7`, `8` (array capacity), `>8` (overrun) |
| ray direction `A.d` | `+x`, `-x`, `+y`, `-y` (axis-aligned; makes `den == 0` for half the planes), diagonal, non-normalized, `{0,0}` |
| ray length `A.t` | `0`, small, `1`, large, `+Inf`, negative |
| ray origin `A.p` | outside shape, inside shape, exactly on boundary |
| AABB shape | normal, `min == max` (degenerate point), inverted `min > max`, huge extents |
| circle radius | `0`, small, normal, huge, negative |
| capsule | `a != b` axis-aligned, `a != b` diagonal, `a == b` degenerate, `r` `0`/normal/negative |
| float class per component | normal, `+0.0`, `-0.0`, subnormal, huge, `±Inf`, `NaN` |
| `c2r` rotation | identity `{1,0}`, unit `{cos,sin}`, non-unit |

**C. Full set of public entry points — all 28 (not just the wrappers)**

Level 0 (pure vector math): `c2V` `c2Dot` `c2Len` `c2Add` `c2Sub` `c2Mulvs`
`c2Div` `c2Norm` `c2Minv` `c2Maxv` `c2Skew` `c2Absv` `c2CCW90`
Level 1 (rot/transform): `c2RotIdentity` `c2xIdentity` `c2Mulrv` `c2MulrvT`
`c2MulxvT` `c2MulmvT`
Level 2 (predicates): `c2AABBtoAABB` `c2AABBtoPoint` `c2CircleToPoint`
Level 3 (raycasts): `c2RaytoCircle` `c2RaytoAABB` `c2RaytoCapsule` `c2RaytoPoly`
Level 4 (dispatch): `c2CastRay`
Level 5 (driver): `poly_ray`

## Row table

Every row is run against BOTH `.so`s with **many** randomized inputs
(seeded, deterministic) unless marked "fixed".

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| 1  | `c2V` | random `(x,y)` from the mixed float generator (normal/±0/subnormal/huge/Inf/NaN) | [x] |
| 2  | `c2Dot` | random pairs, normal magnitudes | [x] |
| 3  | `c2Dot` | random pairs from the mixed float generator (Inf/NaN/±0 mix ⇒ `0*Inf`) | [x] |
| 4  | `c2Len` | random normal vectors | [x] |
| 5  | `c2Len` | mixed floats: zero vector, huge (overflow to Inf), Inf, NaN | [x] |
| 6  | `c2Add` | random normal + mixed floats | [x] |
| 7  | `c2Sub` | random normal + mixed floats | [x] |
| 8  | `c2Mulvs` | random vector × random scalar (incl. `0`, `-0`, Inf, NaN) | [x] |
| 9  | `c2Div` | random vector ÷ random nonzero scalar | [x] |
| 10 | `c2Div` | random vector ÷ scalar from mixed set incl. `0`, `-0`, Inf, NaN | [x] |
| 11 | `c2Norm` | random normal vectors (unit-length output) | [x] |
| 12 | `c2Norm` | mixed: zero vector, huge, Inf, NaN components | [x] |
| 13 | `c2Minv` / `c2Maxv` | random normal pairs | [x] |
| 14 | `c2Minv` / `c2Maxv` | pairs containing NaN and ±0.0 (ternary vs `fminf` semantics) | [x] |
| 15 | `c2Skew` / `c2CCW90` | random mixed vectors (sign of `-0.0` matters) | [x] |
| 16 | `c2Absv` | random mixed vectors incl. `-0.0` (must keep sign) and NaN | [x] |
| 17 | `c2RotIdentity` / `c2xIdentity` | no input — fixed, called once each | [x] |
| 18 | `c2Mulrv` / `c2MulrvT` | identity rotation `{1,0}` × random vector | [x] |
| 19 | `c2Mulrv` / `c2MulrvT` | unit rotation `{cosθ,sinθ}`, random θ × random vector | [x] |
| 20 | `c2Mulrv` / `c2MulrvT` | non-unit / mixed-float `c2r` × random vector | [x] |
| 21 | `c2MulxvT` | `c2x` = identity; random vector | [x] |
| 22 | `c2MulxvT` | `c2x` = pure translation (`r = {1,0}`, random `p`); random vector | [x] |
| 23 | `c2MulxvT` | `c2x` = pure rotation (`p = {0,0}`, unit `r`); random vector | [x] |
| 24 | `c2MulxvT` | `c2x` = translation + rotation, both random | [x] |
| 25 | `c2MulmvT` | random `c2m` (incl. the `{CCW90(y), y}` shape `c2RaytoCapsule` builds) × random vector | [x] |
| 26 | `c2AABBtoAABB` | both boxes normal, random overlapping & disjoint | [x] |
| 27 | `c2AABBtoAABB` | one/both degenerate (`min == max`) | [x] |
| 28 | `c2AABBtoAABB` | one/both inverted (`min > max`) | [x] |
| 29 | `c2AABBtoAABB` | mixed-float coordinates (Inf / NaN / ±0) | [x] |
| 30 | `c2AABBtoPoint` | normal box, random point (inside / outside / on each of the 4 edges) | [x] |
| 31 | `c2AABBtoPoint` | degenerate + inverted box; mixed-float point | [x] |
| 32 | `c2CircleToPoint` | random circle (`r > 0`), random point incl. exactly on the rim | [x] |
| 33 | `c2CircleToPoint` | `r == 0`, `r < 0`, mixed-float `r` and point | [x] |
| 34 | `c2RaytoCircle` | axis-aligned `d` (`±x`, `±y`), origin outside, `A.t` large enough to hit | [x] |
| 35 | `c2RaytoCircle` | diagonal normalized `d`, origin outside, random hit/miss | [x] |
| 36 | `c2RaytoCircle` | non-normalized `d` (magnitude ≠ 1) — changes the `t` scale | [x] |
| 37 | `c2RaytoCircle` | origin **inside** the circle (`t < 0` branch) | [x] |
| 38 | `c2RaytoCircle` | origin exactly on the circle boundary (`t == 0`) | [x] |
| 39 | `c2RaytoCircle` | `A.t` sweep: `0`, tiny, exactly the hit distance, huge, `+Inf` | [x] |
| 40 | `c2RaytoCircle` | `d == {0,0}`; and `r` ∈ {0, huge}; mixed-float fields | [x] |
| 41 | `c2RaytoAABB` | axis-aligned `d` (`+x`) hitting through the box; picks the `t0` (`n = {-1,0}`) branch | [x] |
| 42 | `c2RaytoAABB` | axis-aligned `d` (`-x`); `t1` (`n = {1,0}`) branch | [x] |
| 43 | `c2RaytoAABB` | axis-aligned `d` (`+y`); `t2` (`n = {0,-1}`) branch | [x] |
| 44 | `c2RaytoAABB` | axis-aligned `d` (`-y`); `t3` (`n = {0,1}`) branch | [x] |
| 45 | `c2RaytoAABB` | diagonal `d`; the tie/`>=` chain between `t0..t3` decides the normal | [x] |
| 46 | `c2RaytoAABB` | origin inside the box | [x] |
| 47 | `c2RaytoAABB` | origin exactly on a face / at a corner | [x] |
| 48 | `c2RaytoAABB` | `A.t` sweep: `0`, short (stops before the box), exact, long, `+Inf` | [x] |
| 49 | `c2RaytoAABB` | degenerate box (`min == max`) and huge box | [x] |
| 50 | `c2RaytoAABB` | fully random mixed-float ray + box (fuzz over rows 41–49 in one bucket) | [x] |
| 51 | `c2RaytoCapsule` | vertical axis (`a.x == b.x`), origin outside the side slab, ray crosses the side ⇒ `out->n = M.x` branch | [x] |
| 52 | `c2RaytoCapsule` | vertical axis, `c < 0` ⇒ `out->n = c2Skew(M.y)` branch | [x] |
| 53 | `c2RaytoCapsule` | ray origin inside `capsule_bb` ⇒ immediate `return 1` with `out->n = c2Norm(cap_n)` | [x] |
| 54 | `c2RaytoCapsule` | origin inside end-cap circle `a` ⇒ `c2CircleToPoint(capsule_a)` `return 1` | [x] |
| 55 | `c2RaytoCapsule` | origin inside end-cap circle `b` ⇒ `c2CircleToPoint(capsule_b)` `return 1` | [x] |
| 56 | `c2RaytoCapsule` | `\|yAp.x\| < B.r`, `yAp.y < 0` ⇒ delegates to `c2RaytoCircle(Ca)` | [x] |
| 57 | `c2RaytoCapsule` | `\|yAp.x\| < B.r`, `yAp.y >= 0` ⇒ delegates to `c2RaytoCircle(Cb)` | [x] |
| 58 | `c2RaytoCapsule` | side-slab path, `y <= 0` ⇒ delegates to `c2RaytoCircle(Ca)` | [x] |
| 59 | `c2RaytoCapsule` | side-slab path, `y >= yBb.y` ⇒ delegates to `c2RaytoCircle(Cb)` | [x] |
| 60 | `c2RaytoCapsule` | diagonal axis, `a.y > b.y` (axis pointing "down" ⇒ negative `yBb.y`) | [x] |
| 61 | `c2RaytoCapsule` | `A.t` sweep `0`/short/long/`+Inf`; `d` non-normalized and `{0,0}` | [x] |
| 62 | `c2RaytoCapsule` | fully random mixed-float ray + capsule (fuzz across all branches) | [x] |
| 63 | `c2RaytoPoly` | `bx = NULL`, `count = 4` box, `d = +x`, ray hits a side (`index` set) | [x] |
| 64 | `c2RaytoPoly` | `bx = NULL`, `count = 4`, `d = -y` (the second `poly_ray` ray shape) | [x] |
| 65 | `c2RaytoPoly` | `bx = NULL`, `count = 3` triangle, random rays | [x] |
| 66 | `c2RaytoPoly` | `bx = NULL`, `count = 5,6,7,8` regular n-gons, random rays | [x] |
| 67 | `c2RaytoPoly` | `bx = NULL`, `count = 1` and `count = 2` (degenerate half-plane / slab) | [x] |
| 68 | `bx = &identity` | `c2RaytoPoly`, `count = 4`, same rays as row 63 (must match the `NULL` result bit-for-bit) | [x] |
| 69 | `bx = pure translation` | `c2RaytoPoly`, `count = 4..8`, random rays | [x] |
| 70 | `bx = pure rotation` | `c2RaytoPoly`, `count = 4..8`, random θ, random rays | [x] |
| 71 | `bx = translation + rotation` | `c2RaytoPoly`, `count = 3..8`, random rays | [x] |
| 72 | `bx = non-unit c2r` | `c2RaytoPoly`, `count = 4`, random rays (no normalization in C) | [x] |
| 73 | `c2RaytoPoly` | origin **inside** the polygon (all `num < 0`, `index` may stay `~0`) | [x] |
| 74 | `c2RaytoPoly` | `d` exactly parallel to a plane (`den == 0`, `num >= 0`) | [x] |
| 75 | `c2RaytoPoly` | `A.t` sweep `0`/short/exact/long/`+Inf`; non-normalized `d` | [x] |
| 76 | `c2RaytoPoly` | `count = 8` with **fully random** verts/norms (non-convex, unnormalized normals) | [x] |
| 77 | `c2RaytoPoly` | `count = 9..16` reading past `verts[8]`/`norms[8]` from an over-allocated backing buffer | [x] |
| 78 | `c2RaytoPoly` | fully random mixed-float ray + poly + `bx` fuzz | [x] |
| 79 | `c2CastRay` | `typeB = C2_TYPE_CIRCLE (0)`, `bx = NULL`, random circles & rays | [x] |
| 80 | `c2CastRay` | `typeB = C2_TYPE_CIRCLE (0)`, `bx` non-NULL (must be ignored) | [x] |
| 81 | `c2CastRay` | `typeB = C2_TYPE_AABB (1)`, `bx = NULL`, random boxes & rays | [x] |
| 82 | `c2CastRay` | `typeB = C2_TYPE_AABB (1)`, `bx` non-NULL (ignored) | [x] |
| 83 | `c2CastRay` | `typeB = C2_TYPE_CAPSULE (2)`, `bx = NULL`, random capsules & rays | [x] |
| 84 | `c2CastRay` | `typeB = C2_TYPE_CAPSULE (2)`, `bx` non-NULL (ignored) | [x] |
| 85 | `c2CastRay` | `typeB = C2_TYPE_POLY (3)`, `bx = NULL`, `count = 3..8` | [x] |
| 86 | `c2CastRay` | `typeB = C2_TYPE_POLY (3)`, `bx` = rot+trans, `count = 3..8` | [x] |
| 87 | `poly_ray` | no input — fixed. Both `cast1`/`cast2` out-structs + return code compared byte-for-byte | [x] |
| 88 | `poly_ray` | called repeatedly with pre-dirtied `out` buffers (checks which fields C leaves untouched) | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)`; there
is no `add_executable`. `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]`, no `[[bin]]`. **No driver binary exists on either
side**, so the "compare stdout byte-for-byte" gate is not applicable.

## Feature combinations

`translation/Cargo.toml` has **no** `[features]` section, so the only
configuration is the default (empty) feature set. Verified by
`cargo read-manifest | python3 -c 'import json,sys; print(json.load(sys.stdin)["features"])'`
→ `{}`. Phases B and C therefore run once, under `--no-default-features` and
the default, which are identical here (both are checked in `run_all.sh`).

## Phase B results

All 88 rows implemented as `translation/tests/phase_b_valid.rs::rowNN_*`, each
calling BOTH `.so`s through `libloading` and comparing the return code plus the
full `out` struct **by raw f32 bit pattern** (so NaN payloads and the sign of
zero are part of the assertion). The `out` buffer is pre-filled with a `DIRTY`
sentinel so "which fields C leaves untouched" is asserted too.

```
DIFF_ITERS=1000000  debug    88 passed; 0 failed
DIFF_ITERS=1000000  release  88 passed; 0 failed
```

That is ~1e6 randomized inputs per row from a fixed-seed SplitMix64 generator
(one distinct seed per row, so runs are reproducible).

### Divergences found and fixed in Phase B

Every one was a NaN-payload mismatch with the same root cause: x86
`ADDSS`/`SUBSS`/`MULSS`/`DIVSS` return the **destination** operand's NaN
(quieted), and gcc `-O0` and LLVM choose different destination registers for
the same C expression. Confirmed by disassembling both `.so`s. Fixed by adding
explicit `fadd`/`fsub`/`fmul`/`fdiv` helpers in `src/lib.rs` that encode the
SSE NaN-precedence rule in software, with gcc's destination operand passed
first — making the result codegen-independent instead of relying on how
`a op b` happens to be register-allocated. For every non-NaN input these are
bit-identical to the plain operators.

| function | site | gcc destination operand |
|----------|------|--------------------------|
| `c2Dot` | `a.x*b.x + a.y*b.y` | `mulss` #2 dest = `b.y`; `addss` dest = the *second* product |
| `c2Add` | `a.x += b.x` | `addss` dest = `b.x` (the **right** operand) |
| `c2Sub` | `a.x -= b.x` | `subss` dest = `a.x` |
| `c2Mulvs` | `a.x *= b` | `mulss` dest = `a.x` |
| `c2Div` | `1.0f / b` | `divss` dest = the `1.0f` constant |
| `c2MulmvT` | both components | `mulss` #2 dest = `b.y`; `addss` dest = second product |
| `c2Mulrv` | comp 0 | `subss(mulss(b.x, a.c), mulss(b.y, a.s))` |
| `c2Mulrv` | comp 1 | `addss(mulss(a.s, b.x), mulss(b.y, a.c))` |
| `c2MulrvT` | comp 0 | `addss(mulss(a.c, b.x), mulss(b.y, a.s))` |
| `c2MulrvT` | comp 1 | `addss(mulss(-a.s, b.x), mulss(b.y, a.c))` |
| `c2RaytoAABB` | `out->t = t_i * A.t` | `mulss` dest = **`A.t`** |
| `c2RaytoCapsule` | `yAp.y + (yAe.y-yAp.y)*t` | `addss` dest = the **product** |
| `c2RaytoCapsule` | `out->t = t * A.t` | `mulss` dest = **`A.t`** |

## Binary executable — not applicable

Re-confirmed mechanically:

```sh
grep -c add_executable c_src/CMakeLists.txt                  # 0
cargo read-manifest | python3 -c '...["targets"]...'         # [("poly_ray_lib", ["cdylib"])]
```

Neither side builds a driver binary, so there is no stdout to compare.
