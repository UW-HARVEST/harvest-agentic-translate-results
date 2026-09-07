# CONFIGS.md — configuration / valid-input surface table

Axes the C code actually branches on (derived from `c_src/src/lib.c`):

* **Shape type pair** — `c2Collide`'s nested `switch (typeA) { switch (typeB) }`
  gives 9 handled ordered pairs over `{CIRCLE, AABB, CAPSULE}`. The ordering
  matters: 4 of the 9 pairs re-order the arguments and then apply
  `m->n = c2Neg(m->n)`.
* **`c2GJK` runtime options** — `ax_ptr` NULL vs identity vs a real rotation,
  `bx_ptr` likewise, `use_radius` 0 vs 1, `outA`/`outB` NULL vs non-NULL,
  `iterations` NULL vs non-NULL, `cache` NULL vs zeroed vs warm.
* **Proxy shape** — `c2MakeProxy` produces `count` 1 (circle), 2 (capsule),
  4 (AABB), and leaves the proxy untouched for poly. Proxy `radius` is 0 for
  AABB, `r` otherwise.
* **Simplex `count`** — 1, 2, 3 (and 0/4 for the `default:` arms) select
  entirely different bodies in `c22`, `c23`, `c2D`, `c2L`, `c2Witness`,
  `c2GJKSimplexMetric`.
* **`c23` region** — 7 mutually exclusive Voronoi regions (vertex A, vertex B,
  vertex C, edge AB, edge BC, edge CA, interior).
* **`c22` region** — 3 regions (vertex A, vertex B, edge).
* **`c2CapsuletoPolyManifold` `code`** — 0 (poly face reference), 1 (capsule
  side plane 0), 2 (capsule side plane 1); crossed with the shallow branch
  (`1e-6 <= d < A.r`) and the deep branch (`d < 1e-6`).
* **Separation regime** — deeply overlapping / touching / shallow overlap /
  separated, for each pair.
* **Degenerate shapes** — zero-radius circle, zero-extent AABB, inverted AABB,
  point capsule (`a == b`), axis-aligned vs diagonal capsule.
* **Poly `count`** — 3, 4, 5, …, 8 vertices, plus the transform `bx_ptr`
  NULL / identity / rotated+translated (only reachable via the direct
  `c2CapsuletoPolyManifold` entry point — `c2Collide` never builds a poly with
  more than the 4 AABB verts).
* **Float value classes** — normal, `0.0`/`-0.0`, subnormal, huge, `inf`, `NaN`.

Every row is exercised with many randomized inputs (fixed seed, xorshift PRNG)
through both `.so` files, not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V`, `c2Mulvs`, `c2Sub`, `c2Add`, `c2Neg`, `c2Dot`, `c2Det2`, `c2Len`, `c2Div`, `c2Skew`, `c2CCW90`, `c2Absv` | random finite f32 pairs, full exponent range | [x] |
| 2 | same as row 1 | signed zeros, subnormals, `±inf`, `NaN` mixed in | [x] |
| 3 | `c2Maxv`, `c2Minv`, `c2Clampv` | random values; equal components; `lo > hi` inverted range; NaN operands (C ternary keeps `b` on NaN) | [x] |
| 4 | `c2Norm` | random non-zero vectors; near-zero magnitudes; huge magnitudes (overflow to inf) | [x] |
| 5 | `c2Intersect` | random `a`,`b`,`da`,`db`; `da == db` (division by zero); `da == 0` | [x] |
| 6 | `c2Dot`/`c2Dist`/`c2PlaneAt` | random `c2h` plane + point; index 0..7 into a random poly | [x] |
| 7 | `c2RotIdentity`, `c2xIdentity` | no inputs — constant parity | [x] |
| 8 | `c2Mulrv`, `c2MulrvT` | random unit rotations; non-unit `c2r`; zero `c2r` | [x] |
| 9 | `c2Mulxv`, `c2MulxvT` | identity transform / pure translation / pure rotation / both | [x] |
| 10 | `c2BBVerts` | random AABB; zero-extent; inverted (`min > max`) | [x] |
| 11 | `c2Norms` | poly with count 3,4,5,6,7,8; duplicate consecutive verts (NaN normals); count 0 | [x] |
| 12 | `c2Support` | count 1,2,4,8; random direction; ties in the dot product; count 0 | [x] |
| 13 | `c2MakeProxy` | `type = CIRCLE` (count 1, radius r) | [x] |
| 14 | `c2MakeProxy` | `type = CAPSULE` (count 2, radius r) | [x] |
| 15 | `c2MakeProxy` | `type = AABB` (count 4, radius 0) | [x] |
| 16 | `c2GJKSimplexMetric` | `count = 1` / `2` / `3`, randomized simplex points | [x] |
| 17 | `c22` | simplex `count = 2` covering all 3 Voronoi regions (`v<=0`, `u<=0`, interior) | [x] |
| 18 | `c23` | simplex `count = 3` covering all 7 regions, incl. degenerate/zero area | [x] |
| 19 | `c2D` | `count = 1` / `2` (both `c2Det2` signs) / `3` | [x] |
| 20 | `c2L` | `count = 1` / `2`, random `div` incl. `div = 0` | [x] |
| 21 | `c2Witness` | `count = 1` / `2` / `3`, random `div` and `u` weights | [x] |
| 22 | `c2GJK` | CIRCLE vs CIRCLE, `ax/bx = NULL`, `use_radius = 0`, no cache | [x] |
| 23 | `c2GJK` | CIRCLE vs CIRCLE, `use_radius = 1` (radius shrink branch) | [x] |
| 24 | `c2GJK` | CAPSULE vs CAPSULE, `use_radius = 0` and `1` | [x] |
| 25 | `c2GJK` | AABB vs AABB (proxy count 4 both sides), `use_radius = 0`/`1` | [x] |
| 26 | `c2GJK` | CIRCLE vs AABB and AABB vs CIRCLE (asymmetric proxy counts) | [x] |
| 27 | `c2GJK` | CAPSULE vs AABB and AABB vs CAPSULE | [x] |
| 28 | `c2GJK` | CIRCLE vs CAPSULE and CAPSULE vs CIRCLE | [x] |
| 29 | `c2GJK` | non-NULL `ax_ptr`, identity transform | [x] |
| 30 | `c2GJK` | non-NULL `ax_ptr`, pure translation | [x] |
| 31 | `c2GJK` | non-NULL `ax_ptr` **and** `bx_ptr`, both rotated + translated (`c2r` from `cos/sin` of a random angle) | [x] |
| 32 | `c2GJK` | non-unit / zero `c2r` in the transform | [x] |
| 33 | `c2GJK` | `outA = NULL`, `outB` non-NULL, and vice versa | [x] |
| 34 | `c2GJK` | `iterations` non-NULL — iteration count parity | [x] |
| 35 | `c2GJK` | `cache` non-NULL, zero-initialized (cold cache) — cache written out | [x] |
| 36 | `c2GJK` | `cache` non-NULL, **warm**: run once, feed the resulting cache back with unchanged shapes | [x] |
| 37 | `c2GJK` | `cache` warm, then shapes moved (stale but in-range cache) | [x] |
| 38 | `c2GJK` | separated / touching / shallow / deeply overlapping, for each pair (distance regimes) | [x] |
| 39 | `c2GJK` | POLY type (proxy untouched ⇒ zeroed proxy) as A and as B | [x] |
| 40 | `c2CircletoCircleManifold` | separated, shallow overlap, deep overlap, concentric (`l == 0`), zero radius, huge radius | [x] |
| 41 | `c2CircletoAABBManifold` | circle outside; overlapping a face; overlapping a corner; centre strictly inside (`d2 == 0` deep branch, `x_overlap < y_overlap` and `>=`); zero-radius circle; zero-extent AABB | [x] |
| 42 | `c2CircletoCapsuleManifold` | separated; overlapping near the capsule body; overlapping near an end cap; `d == 0` (centre on the segment); point capsule | [x] |
| 43 | `c2AABBtoAABBManifold` | separated on x; separated on y; overlapping with `dx < dy`; with `dx >= dy`; all four sign combinations of `d.x`/`d.y`; touching exactly; identical boxes; zero-extent; inverted | [x] |
| 44 | `c2CapsuletoCapsuleManifold` | parallel / crossing / collinear / end-to-end capsules; separated; `d == 0`; point capsules | [x] |
| 45 | `c2AABBtoCapsuleManifold` | capsule crossing a face; a corner; fully inside; separated; axis-aligned and diagonal capsules; point capsule | [x] |
| 46 | `c2CapsuletoPolyManifold` | `bx_ptr = NULL`, poly count 4 (AABB-like), deep branch (`d < 1e-6`), `code = 0` | [x] |
| 47 | `c2CapsuletoPolyManifold` | `bx_ptr = NULL`, deep branch, `code = 1` (capsule side plane 0 wins) | [x] |
| 48 | `c2CapsuletoPolyManifold` | `bx_ptr = NULL`, deep branch, `code = 2` (capsule side plane 1 wins) | [x] |
| 49 | `c2CapsuletoPolyManifold` | shallow branch `1e-6 <= d < A.r` | [x] |
| 50 | `c2CapsuletoPolyManifold` | `bx_ptr` = identity `c2x` (non-NULL) | [x] |
| 51 | `c2CapsuletoPolyManifold` | `bx_ptr` = pure translation | [x] |
| 52 | `c2CapsuletoPolyManifold` | `bx_ptr` = rotation + translation | [x] |
| 53 | `c2CapsuletoPolyManifold` | poly vertex count 3 | [x] |
| 54 | `c2CapsuletoPolyManifold` | poly vertex count 5, 6, 7, 8 (regular polygons, CCW, `c2Norms`-derived normals) | [x] |
| 55 | `c2CapsuletoPolyManifold` | randomized capsule × randomized convex poly × randomized transform (broad property sweep) | [x] |
| 56 | `c2Collide` | `CIRCLE`×`CIRCLE` | [x] |
| 57 | `c2Collide` | `CIRCLE`×`AABB` | [x] |
| 58 | `c2Collide` | `CIRCLE`×`CAPSULE` | [x] |
| 59 | `c2Collide` | `AABB`×`CIRCLE` (swapped args + `c2Neg` on the normal) | [x] |
| 60 | `c2Collide` | `AABB`×`AABB` | [x] |
| 61 | `c2Collide` | `AABB`×`CAPSULE` | [x] |
| 62 | `c2Collide` | `CAPSULE`×`CIRCLE` (swapped + `c2Neg`) | [x] |
| 63 | `c2Collide` | `CAPSULE`×`AABB` (swapped + `c2Neg`) | [x] |
| 64 | `c2Collide` | `CAPSULE`×`CAPSULE` | [x] |
| 65 | `ptr_from_parts` | `CIRCLE` — check the 3 floats land in `p.x`,`p.y`,`r` | [x] |
| 66 | `ptr_from_parts` | `AABB` — 4 floats into `min`,`max` | [x] |
| 67 | `ptr_from_parts` | `CAPSULE` — 5 floats into `a`,`b`,`r` | [x] |
| 68 | `omni_manifold` | all 9 handled type pairs × randomized float packs (the public header entry point) | [x] |
| 69 | `omni_manifold` | all 9 pairs, coordinates drawn from a small grid so hits/near-misses/exact ties are frequent | [x] |
| 70 | `omni_manifold` | all 9 pairs with `±0.0`, subnormal, `±inf`, `NaN` in the float packs | [x] |
| 71 | `omni_manifold` | all 16 ordered type pairs including `POLY` and out-of-range ints | [x] |

No binary/driver target exists (`Cargo.toml` declares only `crate-type =
["cdylib"]`; `CMakeLists.txt` declares only `add_library(... SHARED)`), so the
"compare stdout of the C and Rust binaries" clause does not apply. Verified
mechanically: `grep -c 'add_executable' c_src/CMakeLists.txt` is 0 and the crate
has no `src/main.rs` / `[[bin]]` section.

## Where the rows are tested

| rows | file |
|------|------|
| 1..21 | `tests/phase_b_primitives.rs` |
| 22..39 | `tests/phase_b_gjk.rs` |
| 40..55 | `tests/phase_b_manifolds.rs` |
| 56..71 | `tests/phase_b_collide.rs` |

Every test name carries its row numbers, uses a fixed PRNG seed, and asserts
bit-for-bit equality of every output byte (including the manifold slots beyond
`count`, which both libraries are handed pre-seeded with the same pattern).
Several tests also assert *coverage* — e.g. `c22` must reach both of its
reachable region outcomes, `c23` all three simplex counts, and
`c2CapsuletoPolyManifold` all of manifold `count` 0, 1 and 2 — so a row cannot
silently pass by never exercising the branch it is meant to cover.

## Preconditions the tests must establish

Rows that reach `c2GJK` with `C2_TYPE_POLY` (39, 45..55, 61, 63) go through code
where the C reads an **uninitialized** `c2Proxy`, because `c2MakeProxy` has no
poly case. Those tests call `common::scrub_stack()` before every call into either
library, which is the only reproducible precondition. See the "Irreproducible UB"
section of `ERRORS.md` for the measurements behind that decision.

## Bugs this table found

Row 68/70 (`omni_manifold` over all pairs with special float classes) is what
surfaced the `c2Clip` abort and, indirectly, the `p->verts[-1]` and cache-index
defects. All three are written up in `ERRORS.md`. None of them are reachable from
a per-function happy-path test — they only appear when the whole pipeline is
driven end to end with awkward value classes, which is the point of enumerating
the cross-product here rather than testing one call at a time.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only
configuration is the default one. Verified mechanically:

```
$ grep -n '\[features\]' translation/Cargo.toml            # no match (exit 1)
$ cargo metadata --no-deps --format-version 1 | ... features   # {}
```

`verify.sh` still enumerates the feature list out of `cargo metadata` rather than
assuming, and runs the whole suite under `<default>`,
`--no-default-features` and `--all-features`. All three pass with 46/46 symbol
parity.

## Status

- [x] All 71 rows have a passing differential test across randomized inputs.
- [x] No binary target exists in either project, so the stdout-comparison clause
      does not apply (verified mechanically above).
- [x] All rows pass under every feature combination.
- [x] Fixed seeds cannot be hiding a divergence by luck: the whole suite was
      re-run under 79 independent global seeds (`DIFF_SEED=0..80`,
      `SWEEP=n translation/verify.sh`) with zero failures.
