# CONFIGS.md — Phase B configuration-surface table

Axes derived mechanically from the branches in `c_src/src/lib.c`:

* **Entry-point level.** Leaf vector math (`c2V`…`c2Absv`, `c2CCW90`, `c2MulmvT`,
  `c2Rot/xIdentity`, `c2Mulrv`, `c2MulrvT`, `c2MulxvT`) → predicates
  (`c2AABBtoAABB`, `c2AABBtoPoint`, `c2CircleToPoint`) → raycasts
  (`c2RaytoCircle`, `c2RaytoAABB`, `c2RaytoCapsule`, `c2RaytoPoly`) → dispatcher
  (`c2CastRay`) → one-shot wrapper (`poly_ray`). All five levels are driven
  directly, not only through `poly_ray`.
* **`c2CastRay` mode flag:** `typeB` ∈ {`C2_TYPE_CIRCLE`=0, `C2_TYPE_AABB`=1,
  `C2_TYPE_CAPSULE`=2, `C2_TYPE_POLY`=3} — the only runtime "option" in the API.
* **`bx` transform state (`c2RaytoPoly` / `c2CastRay`):** `NULL` (⇒ identity),
  explicit identity, pure translation, pure rotation, translation+rotation,
  non-unit "rotation".
* **Input shapes:** poly `count` ∈ {0,1,2,3,4,5..8}; ray direction axis-aligned
  vs oblique vs zero vs unnormalised; ray `t` ∈ {0, small, exact-boundary, large,
  inf}; ray origin inside vs outside vs exactly on a face; AABB/capsule/circle
  degenerate vs normal vs inverted; float classes {normal, ±0.0, denormal, ±inf,
  NaN, ±FLT_MAX/MIN}.

Every row is exercised with **many randomized inputs** from a fixed-seed
xorshift PRNG (`SEED = 0x2545F4914F6CDD1D`), plus a hand-picked set of the
boundary/special values named in the row. All comparisons are **bit-for-bit** on
the returned `int` *and* on the raw `u32` bit patterns of every out-param float.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `c2V`, `c2Skew`, `c2CCW90`, `c2Absv` | pure component shuffles; 4096 random floats × {normal, ±0, denormal, ±inf, NaN, ±FLT_MAX} | `cfg_01_leaf_unary` | [x] |
| 2 | `c2Add`, `c2Sub`, `c2Mulvs` | 4096 random pairs; plus inf−inf, 0×inf, ±0 sign combinations | `cfg_02_leaf_binary` | [x] |
| 3 | `c2Dot`, `c2Len` | 4096 random pairs; overflow-to-inf, underflow-to-0, NaN, and `c2Len` on huge/tiny vectors | `cfg_03_dot_len` | [x] |
| 4 | `c2Div`, `c2Norm` | divisor ∈ {random, +0.0, −0.0, ±inf, NaN, ±FLT_MIN, ±FLT_MAX}; `c2Norm` on zero / unit / huge / NaN vectors (reciprocal-multiply semantics) | `cfg_04_div_norm` | [x] |
| 5 | `c2Minv`, `c2Maxv` | all 4 NaN placements (none/a/b/both) × ±0.0 combinations × 4096 random pairs — **ternary**, not `fminf`/`fmaxf` | `cfg_05_minv_maxv` | [x] |
| 6 | `c2MulmvT`, `c2Mulrv`, `c2MulrvT`, `c2MulxvT` | 4096 random matrices/rotors × vectors; unit rotors, zero rotor, non-unit rotor, inf/NaN entries; note `c2MulrvT` lane 2 is `(-a.s)*b.x + a.c*b.y` | `cfg_06_matrix_ops` | [x] |
| 7 | `c2RotIdentity`, `c2xIdentity` | nullary constructors — exact bit equality of all fields | `cfg_07_identities` | [x] |
| 8 | `c2AABBtoAABB` | overlapping / touching-edge / touching-corner / disjoint on each of 4 axes / one inside other / identical / inverted min>max / NaN — 4096 random boxes | `cfg_08_aabb_aabb` | [x] |
| 9 | `c2AABBtoPoint` | inside / on each of 4 edges / on each corner / outside each side / inverted box / NaN — 4096 random | `cfg_09_aabb_point` | [x] |
| 10 | `c2CircleToPoint` | inside / exactly on boundary (strict `<` ⇒ miss) / outside / r=0 / r<0 / NaN — 4096 random | `cfg_10_circle_point` | [x] |
| 11 | `c2RaytoCircle` | **hit** cases: ray outside pointing in, tangent, through centre; `A.d` unit vs unnormalised vs zero; 8192 random rays × circles | `cfg_11_ray_circle_random` | [x] |
| 12 | `c2RaytoCircle` | `A.t` sweep: 0, just-below `t_hit`, exactly `t_hit`, just-above, large, inf, NaN — boundary of `t <= A.t` | `cfg_12_ray_circle_t_boundary` | [x] |
| 13 | `c2RaytoCircle` | origin **inside** the circle (`c < 0` ⇒ `disc > 0` but `t < 0`) | `cfg_13_ray_circle_inside` | [x] |
| 14 | `c2RaytoAABB` | axis-aligned rays entering through each of the 4 faces (each of the 4 `t0..t3` winner branches) | `cfg_14_ray_aabb_four_faces` | [x] |
| 15 | `c2RaytoAABB` | oblique rays, corner hits, ties in the `t0>=t1&&…` chain (equal `t` ⇒ first branch wins) | `cfg_15_ray_aabb_oblique_ties` | [x] |
| 16 | `c2RaytoAABB` | ray origin inside the box; ray fully inside; zero-extent box (`min==max`); zero-direction ray | `cfg_16_ray_aabb_degenerate` | [x] |
| 17 | `c2RaytoAABB` | 16384 random rays × boxes (both hit and miss, exercising the bb pre-reject, the SAT `d>0` reject, and the `hit` mask) | `cfg_17_ray_aabb_random` | [x] |
| 18 | `c2RaytoCapsule` | **branch: `c2AABBtoPoint(capsule_bb, yAp)`** — origin inside the capsule's rectangular core ⇒ early `return 1` with `out->n = normalize(b-a)`, `out->t = 0` | `cfg_18_capsule_inside_core` | [x] |
| 19 | `c2RaytoCapsule` | **branch: `c2CircleToPoint(capsule_a, A.p)`** — origin inside the `a` end-cap only | `cfg_19_capsule_inside_cap_a` | [x] |
| 20 | `c2RaytoCapsule` | **branch: `c2CircleToPoint(capsule_b, A.p)`** — origin inside the `b` end-cap only | `cfg_20_capsule_inside_cap_b` | [x] |
| 21 | `c2RaytoCapsule` | **branch: `\|yAp.x\| < B.r` with `yAp.y < 0`** ⇒ delegates to `c2RaytoCircle(A, Ca)` | `cfg_21_capsule_delegate_ca` | [x] |
| 22 | `c2RaytoCapsule` | **branch: `\|yAp.x\| < B.r` with `yAp.y >= 0`** ⇒ delegates to `c2RaytoCircle(A, Cb)` | `cfg_22_capsule_delegate_cb` | [x] |
| 23 | `c2RaytoCapsule` | **else branch, `y <= 0`** ⇒ `c2RaytoCircle(A, Ca)` (crosses the side plane below the capsule) | `cfg_23_capsule_side_below` | [x] |
| 24 | `c2RaytoCapsule` | **else branch, `y >= yBb.y`** ⇒ `c2RaytoCircle(A, Cb)` (crosses above) | `cfg_24_capsule_side_above` | [x] |
| 25 | `c2RaytoCapsule` | **else branch, `0 < y < yBb.y`, `c > 0`** ⇒ `out->n = M.x`, `out->t = t*A.t` | `cfg_25_capsule_side_hit_pos` | [x] |
| 26 | `c2RaytoCapsule` | **else branch, `0 < y < yBb.y`, `c < 0`** ⇒ `out->n = c2Skew(M.y)` | `cfg_26_capsule_side_hit_neg` | [x] |
| 27 | `c2RaytoCapsule` | capsule orientation sweep: vertical (`a.x==b.x`), horizontal, 45°, and 64 random angles × random radii | `cfg_27_capsule_orientations` | [x] |
| 28 | `c2RaytoCapsule` | 16384 random rays × capsules (mixed hits/misses, all branches by chance) | `cfg_28_capsule_random` | [x] |
| 29 | `c2RaytoPoly` | `bx = NULL`, `count = 4` axis-aligned box poly, rays entering each of the 4 faces (each `index` value) | `cfg_29_poly_null_bx_box` | [x] |
| 30 | `c2RaytoPoly` | `bx = &identity` explicitly — must equal row 29 bit-for-bit | `cfg_30_poly_explicit_identity` | [x] |
| 31 | `c2RaytoPoly` | `bx` = pure translation (`r = identity`, `p != 0`) | `cfg_31_poly_translation` | [x] |
| 32 | `c2RaytoPoly` | `bx` = pure rotation (`p = 0`, `r = (cosθ, sinθ)` over 64 angles) | `cfg_32_poly_rotation` | [x] |
| 33 | `c2RaytoPoly` | `bx` = translation **and** rotation combined | `cfg_33_poly_rot_translate` | [x] |
| 34 | `c2RaytoPoly` | `bx.r` **non-unit** (`c*c+s*s != 1`, incl. zero rotor and huge rotor) — C never normalises | `cfg_34_poly_nonunit_rotor` | [x] |
| 35 | `c2RaytoPoly` | `count = 1` (single half-plane) | `cfg_35_poly_count_1` | [x] |
| 36 | `c2RaytoPoly` | `count = 2` (wedge / slab) | `cfg_36_poly_count_2` | [x] |
| 37 | `c2RaytoPoly` | `count = 3` (triangle), including the CCW/CW norm-sign variants | `cfg_37_poly_count_3` | [x] |
| 38 | `c2RaytoPoly` | `count = 5,6,7,8` (regular n-gons, up to the fixed array capacity) | `cfg_38_poly_count_5_to_8` | [x] |
| 39 | `c2RaytoPoly` | ray origin **inside** the poly ⇒ no `den<0` face narrows `lo` ⇒ `index == ~0` ⇒ `0` | `cfg_39_poly_origin_inside` | [x] |
| 40 | `c2RaytoPoly` | ray exactly **parallel** to a face (`den == 0`) with `num > 0` (inside plane ⇒ continue) and `num < 0` (⇒ reject) | `cfg_40_poly_parallel` | [x] |
| 41 | `c2RaytoPoly` | ray origin exactly **on** a face plane (`num == 0`) | `cfg_41_poly_origin_on_face` | [x] |
| 42 | `c2RaytoPoly` | `A.t` sweep: 0, `t_hit`−ε, `t_hit`, `t_hit`+ε, large, inf — the `hi = A.t` initialisation | `cfg_42_poly_t_boundary` | [x] |
| 43 | `c2RaytoPoly` | `A.d` unnormalised (length ≫ 1 and ≪ 1) and zero-direction — C never normalises | `cfg_43_poly_unnormalized_d` | [x] |
| 44 | `c2RaytoPoly` | 16384 random polys (random `count` 1..8, random verts/norms, **not** necessarily convex/consistent) × random rays × random `bx` | `cfg_44_poly_random` | [x] |
| 45 | `c2CastRay` | `typeB = C2_TYPE_CIRCLE` with `bx = NULL` and with `bx != NULL` (ignored) — 4096 random | `cfg_45_castray_circle` | [x] |
| 46 | `c2CastRay` | `typeB = C2_TYPE_AABB`, both `bx` states — 4096 random | `cfg_46_castray_aabb` | [x] |
| 47 | `c2CastRay` | `typeB = C2_TYPE_CAPSULE`, both `bx` states — 4096 random | `cfg_47_castray_capsule` | [x] |
| 48 | `c2CastRay` | `typeB = C2_TYPE_POLY` × {`NULL`, identity, translation, rotation, rot+translate, non-unit} `bx` × `count` 0..8 — 4096 random | `cfg_48_castray_poly_bx_matrix` | [x] |
| 49 | `poly_ray` | the one-shot wrapper: hard-coded slab poly + two hard-coded rays; return value **and** both `c2Raycast` out-params bit-compared | `cfg_49_poly_ray` | [x] |
| 50 | `poly_ray` | called repeatedly / with pre-dirtied out-params (garbage-filled `c2Raycast`) to detect any un-written field divergence | `cfg_50_poly_ray_dirty_out` | [x] |

## No further configuration axes

* No `[features]` in `Cargo.toml`, no `#ifdef`/`#if` in `c_src/` (`grep -c '#if'
  c_src/src/lib.c c_src/include/lib.h` → 0), no byte-order or element-type
  branching, no global/mutable library state, no init/teardown API.
* Therefore the "every feature combination" requirement collapses to the single
  default build, and there is no driver binary to stdout-compare.

---

## Phase B result — every row passes across randomized inputs

```
cargo test --release --test phase_b
test result: ok. 50 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**1,054,605 bit-exact comparisons** in total, every one of them made by calling
BOTH the C `.so` and the Rust `.so` through `libloading` and comparing raw `u32`
bit patterns of the return value and of every out-param field.

Coverage assertions built into the tests (so a row cannot silently pass by never
reaching its own configuration):

* `cfg_14` asserts all **4** distinct `c2RaytoAABB` output normals are produced.
* `cfg_29` asserts all **4** box faces get reported as the entering face.
* `cfg_27` asserts the histogram over all **9** `c2RaytoCapsule` branches is
  non-zero in every bucket; rows 18-26 each additionally assert their own branch
  was reached (branch identified by an oracle built from the C's *own* exported
  helpers, so the classification cannot drift from the C's control flow).
* `cfg_08`/`cfg_17`/`cfg_44` assert a healthy mix of hit and miss outcomes rather
  than a degenerate all-miss run.

### One real bug found and fixed

Rows 2, 3 and 6 initially FAILED. When both operands of a single arithmetic
operator are NaN, x86 propagates the payload of the instruction's *destination*
register, and the reference C is built at `-O0`, so GCC's per-site register
allocation decides which source operand that is — and it is **not** always the
left one (`c2Add` uses `b`, `c2Dot`'s final `addss` uses the *second* product,
`c2MulmvT` likewise). Plain Rust `+`/`*` lets LLVM commute freely and so picked
the other operand. Fixed in `src/lib.rs` by adding explicit
`addss`/`subss`/`mulss`/`divss` helpers and pinning the operand order at each of
the ~30 arithmetic sites to what `objdump -d` of the reference `.so` actually
emits.
