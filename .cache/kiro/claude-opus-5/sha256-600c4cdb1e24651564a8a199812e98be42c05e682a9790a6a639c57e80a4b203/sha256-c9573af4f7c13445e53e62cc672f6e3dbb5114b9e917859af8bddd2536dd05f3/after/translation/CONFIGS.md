# CONFIGS.md — configuration surface table (valid inputs)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

**Runtime options / modes (grepped from the public signatures and the `if`/`switch`
statements that read them):**

| axis | values the C distinguishes | branch site |
|------|---------------------------|-------------|
| `C2_TYPE typeA` | `C2_TYPE_CIRCLE(0)`, `C2_TYPE_AABB(1)`, `C2_TYPE_CAPSULE(2)` | `c2MakeProxy` switch, `c2Collided` outer switch |
| `C2_TYPE typeB` | same 3 values | `c2MakeProxy` switch, `c2Collided` inner switches |
| `c2GJK.use_radius` | `0` (raw simplex distance) vs `!= 0` (radius shrink block, lib.c:483-501) | `else if (use_radius)` |
| `c2GJK.ax_ptr` / `bx_ptr` | `NULL` → `c2xIdentity()`; non-`NULL` → arbitrary `c2x` (translation, rotation, both) | `if (!ax_ptr)` / `if (!bx_ptr)` |
| `c2GJK.cache` | `NULL`; non-`NULL` with `count == 0`; non-`NULL` with `count` 1/2/3 (warm start); re-used across calls | `if (cache)`, `!!cache->count`, metric gate |
| `c2GJK.outA`/`outB`/`iterations` | `NULL` vs non-`NULL` (all 8 combinations of the three) | `if (outA)` / `if (outB)` / `if (iterations)` |
| `c2x.r` | identity (`c=1,s=0`) vs a real rotation — changes `c2Mulrv`/`c2MulrvT` and hence support indices | `c2Mulxv`, `c2MulrvT` |

**Input shapes the code special-cases:**

| axis | values |
|------|--------|
| proxy vertex count | 1 (circle), 2 (capsule), 4 (AABB) — drives `c2Support`'s loop length |
| proxy radius | `0` (AABB, always), `> 0`, `== 0` on circle/capsule, `< 0` |
| relative placement | far apart, near-touching, exactly touching, overlapping, one containing the other, coincident |
| simplex `count` | 1, 2, 3 (each of `c22`'s 3 and `c23`'s 7 reduction branches) |
| degenerate shapes | circle `r == 0`, AABB `min == max` (point) / one-dimensional (line), capsule `a == b` (point) |
| magnitude | subnormal (`1e-40`), small (`1e-6`), unit, large (`1e18`), `FLT_MAX/4` |
| sign | all-positive, all-negative, straddling the origin (the GJK origin test is sign-sensitive) |
| AABB orientation | well-formed (`min <= max`) and inverted (`min > max`, never validated) |

Rows below are the pruned cross-product — one row per combination the C treats
differently. Each row is checked off only after **both** `.so`s agree
byte-for-byte across many seeded-random inputs (`SEED = 0x5DEECE66D`,
`tests/configs_valid.rs`; 512–4096 cases per row depending on cost).

## A. Scalar / vector primitives (lowest level, called directly via `.so`)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random `(x, y)` over mixed magnitudes incl. `±0`, subnormal, `FLT_MAX`, `±inf`, `NaN` | [x] |
| 2 | `c2Mulvs` | random vector × random scalar, incl. `0`, `-0`, `inf`, `NaN`, subnormal (tests flush-to-zero divergence) | [x] |
| 3 | `c2Add`, `c2Sub` | random vector pairs incl. cancellation (`a - a`), `inf - inf` → `NaN` | [x] |
| 4 | `c2Neg` | random vectors incl. `±0` (sign of zero must flip) | [x] |
| 5 | `c2Skew`, `c2CCW90` | random vectors (component swap + sign) | [x] |
| 6 | `c2Dot` | random pairs; magnitudes chosen so `x*x + y*y` overflows / cancels | [x] |
| 7 | `c2Det2` | random pairs incl. parallel (`det == 0`) and anti-parallel vectors | [x] |
| 8 | `c2Len` | random vectors, incl. zero, subnormal, huge (overflow inside `c2Dot`) | [x] |
| 9 | `c2Div` | random vector / random non-zero scalar (both signs, subnormals, huge) | [x] |
| 10 | `c2Norm` | random non-zero vectors of every magnitude class | [x] |
| 11 | `c2Maxv`, `c2Minv` | random pairs; equal components; `NaN` in either operand (C ternary, not `fmaxf`) | [x] |
| 12 | `c2Clampv` | `lo <= hi` (well-formed box) — random `a` inside / outside / on each edge | [x] |
| 13 | `c2Clampv` | `lo > hi` (inverted box) — result must collapse to `lo` | [x] |
| 14 | `c2RotIdentity`, `c2xIdentity` | no inputs — exact bit pattern of the returned struct | [x] |
| 15 | `c2Mulrv`, `c2MulrvT` | identity rotation `(c=1,s=0)` × random vectors | [x] |
| 16 | `c2Mulrv`, `c2MulrvT` | normalised rotation `(cos θ, sin θ)` for random θ × random vectors | [x] |
| 17 | `c2Mulrv`, `c2MulrvT` | un-normalised / scaled `c2r` (the C never checks `c²+s² == 1`) | [x] |
| 18 | `c2Mulxv` | `c2x` = identity rotation + non-zero translation | [x] |
| 19 | `c2Mulxv` | `c2x` = real rotation + non-zero translation | [x] |

## B. Proxy construction

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 20 | `c2BBVerts` | well-formed random AABB → 4 vertices | [x] |
| 21 | `c2BBVerts` | degenerate AABB (`min == max`) and inverted AABB (`min > max`) | [x] |
| 22 | `c2MakeProxy` | `type = C2_TYPE_CIRCLE`, random `c2Circle` (r > 0, r == 0, r < 0) → `count == 1` | [x] |
| 23 | `c2MakeProxy` | `type = C2_TYPE_AABB`, random `c2AABB` (well-formed / degenerate / inverted) → `count == 4`, `radius == 0` | [x] |
| 24 | `c2MakeProxy` | `type = C2_TYPE_CAPSULE`, random `c2Capsule` (a != b, a == b, r >= 0, r < 0) → `count == 2` | [x] |
| 25 | `c2Support` | `count == 1` (circle proxy) — random direction | [x] |
| 26 | `c2Support` | `count == 2` (capsule proxy) — random direction, incl. direction ⟂ to `b-a` (tie) | [x] |
| 27 | `c2Support` | `count == 4` (AABB proxy) — random direction covering all 4 winning indices | [x] |
| 28 | `c2Support` | `count == 8` (full `c2Proxy::verts`) — random vertices + direction | [x] |

## C. Simplex reduction (all C branches, driven directly through the `.so`)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 29 | `c2GJKSimplexMetric` | `count == 1` → `0` | [x] |
| 30 | `c2GJKSimplexMetric` | `count == 2` → `c2Len(b.p - a.p)`, random `p`s | [x] |
| 31 | `c2GJKSimplexMetric` | `count == 3` → `c2Det2`, random `p`s incl. collinear (`det == 0`) | [x] |
| 32 | `c22` | branch `v <= 0` (origin beyond `a`) | [x] |
| 33 | `c22` | branch `u <= 0` (origin beyond `b`; copies `b` over `a`) | [x] |
| 34 | `c22` | branch `else` (origin projects inside the segment) | [x] |
| 35 | `c22` | fully random `a.p`/`b.p` — hits all three branches by chance, plus exact `u == 0` / `v == 0` boundaries | [x] |
| 36 | `c23` | branch 1: `vAB <= 0 && uCA <= 0` (vertex A region) | [x] |
| 37 | `c23` | branch 2: `uAB <= 0 && vBC <= 0` (vertex B region) | [x] |
| 38 | `c23` | branch 3: `uBC <= 0 && vCA <= 0` (vertex C region) | [x] |
| 39 | `c23` | branch 4: `uAB > 0 && vAB > 0 && wABC <= 0` (edge AB) | [x] |
| 40 | `c23` | branch 5: `uBC > 0 && vBC > 0 && uABC <= 0` (edge BC) | [x] |
| 41 | `c23` | branch 6: `uCA > 0 && vCA > 0 && vABC <= 0` (edge CA) | [x] |
| 42 | `c23` | branch 7: `else` (origin inside the triangle → `count = 3`) | [x] |
| 43 | `c23` | fully random `a`/`b`/`c` incl. degenerate (collinear, coincident) triangles and both windings | [x] |
| 44 | `c2D` | `count == 1` → `-a.p`, random | [x] |
| 45 | `c2D` | `count == 2`, `c2Det2(ab, -a.p) > 0` → `c2Skew(ab)` | [x] |
| 46 | `c2D` | `count == 2`, `c2Det2(ab, -a.p) <= 0` → `c2CCW90(ab)` (incl. exactly `0`) | [x] |
| 47 | `c2L` | `count == 1` → `a.p` | [x] |
| 48 | `c2L` | `count == 2`, random `div`/`u` weights (incl. `u == 0`, negative `div`) | [x] |
| 49 | `c2Witness` | `count == 1` → `sA`/`sB` copied verbatim | [x] |
| 50 | `c2Witness` | `count == 2`, random barycentric weights and `div` | [x] |
| 51 | `c2Witness` | `count == 3`, random barycentric weights and `div` | [x] |

## D. `c2GJK` — full option cross-product

Geometry classes per row: `far`, `near`, `touching`, `overlapping`, `contained`,
`coincident`, `degenerate` (each row is run with all of them plus random data).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 52 | `c2GJK` | circle × circle, `use_radius=0`, both transforms `NULL`, cache `NULL` | [x] |
| 53 | `c2GJK` | circle × circle, `use_radius=1`, both transforms `NULL`, cache `NULL` | [x] |
| 54 | `c2GJK` | circle × AABB, `use_radius=0` / `=1`, transforms `NULL` | [x] |
| 55 | `c2GJK` | circle × capsule, `use_radius=0` / `=1`, transforms `NULL` | [x] |
| 56 | `c2GJK` | AABB × circle, `use_radius=0` / `=1`, transforms `NULL` | [x] |
| 57 | `c2GJK` | AABB × AABB, `use_radius=0` / `=1`, transforms `NULL` | [x] |
| 58 | `c2GJK` | AABB × capsule, `use_radius=0` / `=1`, transforms `NULL` | [x] |
| 59 | `c2GJK` | capsule × circle, `use_radius=0` / `=1`, transforms `NULL` | [x] |
| 60 | `c2GJK` | capsule × AABB, `use_radius=0` / `=1`, transforms `NULL` | [x] |
| 61 | `c2GJK` | capsule × capsule, `use_radius=0` / `=1`, transforms `NULL` | [x] |
| 62 | `c2GJK` | all 9 type pairs × `ax` = translation-only, `bx = NULL` | [x] |
| 63 | `c2GJK` | all 9 type pairs × `ax = NULL`, `bx` = translation-only | [x] |
| 64 | `c2GJK` | all 9 type pairs × `ax` = rotation-only, `bx` = rotation-only | [x] |
| 65 | `c2GJK` | all 9 type pairs × `ax` = rotation+translation, `bx` = rotation+translation, `use_radius=0` | [x] |
| 66 | `c2GJK` | all 9 type pairs × `ax`/`bx` = rotation+translation, `use_radius=1` | [x] |
| 67 | `c2GJK` | all 9 type pairs × explicit `c2xIdentity()` structs (must equal the `NULL` result exactly) | [x] |
| 68 | `c2GJK` | all 9 type pairs, un-normalised `c2r` in the transform (never validated by C) | [x] |
| 69 | `c2GJK` | `outA = NULL`, `outB` set — returned distance must still match | [x] |
| 70 | `c2GJK` | `outA` set, `outB = NULL` | [x] |
| 71 | `c2GJK` | `outA = outB = NULL`, `iterations` set | [x] |
| 72 | `c2GJK` | `iterations = NULL`, outputs set | [x] |
| 73 | `c2GJK` | `iterations` set — iteration count must match exactly for every geometry class | [x] |
| 74 | `c2GJK` | `cache` non-`NULL`, zero-initialised (`count == 0`) — cold start, cache written back | [x] |
| 75 | `c2GJK` | `cache` re-used for a second call with the **same** shapes (warm start, `cache_was_read`) | [x] |
| 76 | `c2GJK` | `cache` re-used across a **moving** shape (transform changes between calls) — 8-step sweep | [x] |
| 77 | `c2GJK` | hand-built cache with `count == 1` and valid indices | [x] |
| 78 | `c2GJK` | hand-built cache with `count == 2` and valid indices | [x] |
| 79 | `c2GJK` | hand-built cache with `count == 3` and valid indices | [x] |
| 80 | `c2GJK` | hand-built cache with extreme `metric` (`0`, `-1e9`, `FLT_MAX`, `-FLT_MAX`) exercising the metric gate | [x] |
| 81 | `c2GJK` | hand-built cache with `div` = `0`, `1`, random (feeds `1.0f/div` in `c2Witness`) | [x] |
| 82 | `c2GJK` | degenerate shapes: circle `r == 0`, AABB `min == max`, capsule `a == b` — all 9 pairs, both `use_radius` | [x] |
| 83 | `c2GJK` | coincident shapes (A and B identical, distance 0, `hit` path) — all 9 pairs | [x] |
| 84 | `c2GJK` | exact-touching shapes (`dist == rA + rB`), `use_radius=1` → midpoint-collapse branch | [x] |
| 85 | `c2GJK` | negative radii on circle/capsule, `use_radius=1` | [x] |
| 86 | `c2GJK` | large-magnitude coordinates (`1e18`) and subnormal coordinates (`1e-40`) | [x] |
| 87 | `c2GJK` | inverted AABB (`min > max`) as A and/or B | [x] |
| 88 | `c2GJK` | non-convergent / near-parallel configurations that reach the 20-iteration cap | [x] |

## E. Boolean convenience wrappers

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 89 | `c2CircletoCircle` | random circles: separated, tangent (`d2 == r2`), overlapping, concentric, `r == 0`, `r < 0` | [x] |
| 90 | `c2CircletoAABB` | random circle × well-formed AABB: outside each of the 8 Voronoi regions, inside, tangent | [x] |
| 91 | `c2CircletoAABB` | random circle × degenerate AABB (point / line) and inverted AABB | [x] |
| 92 | `c2CircletoCapsule` | branch `da < 0` (before the segment) | [x] |
| 93 | `c2CircletoCapsule` | branch `da >= 0 && db < 0` (alongside the segment) | [x] |
| 94 | `c2CircletoCapsule` | branch `da >= 0 && db >= 0` (past the segment) | [x] |
| 95 | `c2CircletoCapsule` | degenerate capsule (`a == b`), `r == 0`, `r < 0`, random circles | [x] |
| 96 | `c2AABBtoAABB` | random AABB pairs: disjoint on each axis/side, touching edges, touching corners, nested, identical, inverted | [x] |
| 97 | `c2AABBtoCapsule` | random AABB × capsule: separated, touching, overlapping, capsule inside AABB, degenerate capsule | [x] |
| 98 | `c2CapsuletoCapsule` | random capsule pairs: parallel, crossing, collinear, coincident, degenerate, `r == 0` | [x] |

## F. `c2Collided` dispatcher (all 9 valid type pairs) and the public entry point

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 99 | `c2Collided` | `(CIRCLE, CIRCLE)` random shapes | [x] |
| 100 | `c2Collided` | `(CIRCLE, AABB)` random shapes | [x] |
| 101 | `c2Collided` | `(CIRCLE, CAPSULE)` random shapes | [x] |
| 102 | `c2Collided` | `(AABB, CIRCLE)` random shapes (note: C swaps the operands) | [x] |
| 103 | `c2Collided` | `(AABB, AABB)` random shapes | [x] |
| 104 | `c2Collided` | `(AABB, CAPSULE)` random shapes | [x] |
| 105 | `c2Collided` | `(CAPSULE, CIRCLE)` random shapes (C swaps the operands) | [x] |
| 106 | `c2Collided` | `(CAPSULE, AABB)` random shapes (C swaps the operands) | [x] |
| 107 | `c2Collided` | `(CAPSULE, CAPSULE)` random shapes | [x] |
| 108 | `capsule` | random `(min_x, min_y, max_x, max_y, r)` over the range that makes the three fixed probe shapes hit/miss — all 8 result bitmasks | [x] |
| 109 | `capsule` | boundary args: `r == 0`, `r < 0`, `a == b`, huge/subnormal coordinates, `±0` | [x] |
| 110 | `capsule` | exhaustive grid sweep over the fixed probe geometry (circle @(-70,0) r20, AABB (-40,-40)-(-15,-15), capsule (-40,40)-(-20,100) r10) | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only build
configuration is the default one. `tests/feature_matrix.sh` enumerates the
feature list from `Cargo.toml` and loops over every combination (currently the
single empty combination), rebuilding the cdylib, re-checking `nm -D` symbol
parity and running the whole suite for each. It takes `--release` to repeat the
matrix under the `panic = "abort"` profile.

## Status: all 110 rows verified

```
$ cargo test --release --test configs_valid
test result: ok. 73 passed; 0 failed; 0 ignored
```

Rows map to tests by name (`row052_053_gjk_circle_circle` covers rows 52-53, and
so on), so a failure names its row directly.

### Branch coverage is asserted, not assumed

Rows that exist to reach a specific C branch compute the branch predicate in the
test and assert every branch was actually taken. Measured:

| C function | branches | measured hits |
|------------|----------|---------------|
| `c22` (rows 32-35) | 3 | `[3040, 1238, 3914]` |
| `c23` (rows 36-43) | 7 | `[1520, 1600, 1245, 4095, 2806, 2476, 2642]` |
| `c2D` (rows 44-46) | 3 | `[2731, 2162, 3299]` |
| `c2CircletoCapsule` (rows 92-95) | 3 | `[9025, 10458, 13285]` |
| `capsule` result mask (row 108) | 8 | all 8 masks produced |
| `c2Collided` (rows 99-107) | 9 pairs | each pair produced both a hit and a miss |

Row 110 alone performs 244 205 `capsule()` comparisons over the fixed probe
geometry.

### Cross-compiler / cross-optimisation validation

The suite was additionally run with `C_SO_PATH` pointed at C libraries built at
`-O1`, `-O2`, `-O3` and `-Os` (in addition to the `-O0` build produced by
`c_src/CMakeLists.txt`). All 110 rows pass against every one, so the translation
does not depend on any particular C codegen choice.
