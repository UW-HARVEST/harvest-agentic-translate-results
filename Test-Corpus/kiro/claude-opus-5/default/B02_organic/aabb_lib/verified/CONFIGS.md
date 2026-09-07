# CONFIGS.md — configuration surface table

## Axes derived from the C source

**Runtime options / modes the public API can set** (grepped from the `c2GJK`
signature and its branches):

| axis | values the C branches on | source |
|------|--------------------------|--------|
| `typeA`, `typeB` (`C2_TYPE`) | `C2_TYPE_CIRCLE`, `C2_TYPE_AABB`, `C2_TYPE_CAPSULE` | `c2MakeProxy` switch (line 106), `c2Collided` nested switches (lines 578–616) |
| `ax_ptr` / `bx_ptr` | `NULL` (→ `c2xIdentity`) vs. an explicit `c2x` (identity, pure translation, pure rotation, rotation+translation, non-unit `c2r`) | `if (!ax_ptr)` / `if (!bx_ptr)` (lines 367, 371), used in `c2Mulxv` and `c2MulrvT` |
| `use_radius` | `0` vs. non-zero | `else if (use_radius)` (line 495) |
| `cache` | `NULL`, zeroed (`count == 0`), warm (`count` 1/2/3 from a previous call) | `if (cache)` (line 383), `cache_was_good` |
| `outA` / `outB` / `iterations` | `NULL` vs. non-`NULL` | lines 528–535 region guards |

**Input shapes the code special-cases:**

| axis | values |
|------|--------|
| proxy vertex count | 1 (circle), 2 (capsule), 4 (AABB) — drives `c2Support`'s loop and the simplex growth |
| proxy radius | `0` (AABB) vs. non-zero (circle/capsule) — drives the `use_radius` shrink |
| separation | deeply overlapping, exactly touching, marginally separated, far separated |
| degeneracy | zero-radius circle, zero-extent AABB (`min == max`), degenerate capsule (`a == b`), inverted AABB (`min > max`), negative radius |
| magnitude | ~1, ~1e-30 (subnormal), ~1e30 (near `FLT_MAX`), mixed |
| special values | `-0.0`, `+Inf`, `-Inf`, `NaN` |
| simplex `count` (low-level entry points) | 0, 1, 2, 3, 4, out-of-range |
| `c2Support` `count` | 0, 1, 2, 4, 8, negative |

**Full public entry-point set, including the lowest level.** All 38 exported
symbols are driven directly through the `.so`, not just the `aabb` /
`c2Collided` convenience wrappers:

* level 0 scalar/vector: `c2V`, `c2Mulvs`, `c2Maxv`, `c2Minv`, `c2Clampv`,
  `c2Sub`, `c2Add`, `c2Dot`, `c2Det2`, `c2Len`, `c2Div`, `c2Norm`, `c2Neg`,
  `c2Skew`, `c2CCW90`
* level 0 rotation/transform: `c2RotIdentity`, `c2xIdentity`, `c2Mulrv`,
  `c2MulrvT`, `c2Mulxv`
* level 1 proxy: `c2BBVerts`, `c2MakeProxy`
* level 2 simplex: `c2GJKSimplexMetric`, `c22`, `c23`, `c2D`, `c2L`,
  `c2Support`, `c2Witness`
* level 3: `c2GJK`
* level 4 boolean: `c2AABBtoAABB`, `c2CircletoCircle`, `c2CircletoAABB`,
  `c2CircletoCapsule`, `c2AABBtoCapsule`, `c2CapsuletoCapsule`, `c2Collided`
* level 5 entry point: `aabb`

## Rows

One row per combination the C treats differently. Every row is driven with many
randomized inputs (fixed seed, deterministic xorshift PRNG) via both `.so`s and
compared bit-for-bit on every returned float, every out-parameter, and every
mutated struct byte.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V`, `c2Neg`, `c2Skew`, `c2CCW90` | random finite `f32` bit patterns | [x] |
| 2 | `c2V`, `c2Neg`, `c2Skew`, `c2CCW90` | `-0.0`, `±Inf`, `NaN`, subnormal, `±FLT_MAX` | [x] |
| 3 | `c2Add`, `c2Sub`, `c2Mulvs` | random finite pairs, ~1 magnitude | [x] |
| 4 | `c2Add`, `c2Sub`, `c2Mulvs` | overflow/underflow magnitudes (1e30 × 1e30, 1e-30 × 1e-30) and `NaN`/`Inf` | [x] |
| 5 | `c2Dot`, `c2Det2` | random finite pairs (checks no FMA contraction) | [x] |
| 6 | `c2Dot`, `c2Det2` | cancellation cases (`a·b` where terms nearly cancel), `Inf*0` → NaN | [x] |
| 7 | `c2Len` | random finite vectors | [x] |
| 8 | `c2Len` | zero vector, subnormal, huge (`dot` overflows to `Inf`), `NaN` | [x] |
| 9 | `c2Div`, `c2Norm` | random finite vectors, non-zero divisor | [x] |
| 10 | `c2Div`, `c2Norm` | divisor `0`/`-0`, zero-length vector, `Inf`, `NaN` | [x] |
| 11 | `c2Maxv`, `c2Minv`, `c2Clampv` | random finite, ordered `lo <= hi` | [x] |
| 12 | `c2Maxv`, `c2Minv`, `c2Clampv` | `NaN` operands (ternary, not `fmaxf`), `-0.0` vs `+0.0`, inverted `lo > hi` | [x] |
| 13 | `c2RotIdentity`, `c2xIdentity` | no inputs — exact constant returns | [x] |
| 14 | `c2Mulrv`, `c2MulrvT` | unit `c2r` from random angles × random vector | [x] |
| 15 | `c2Mulrv`, `c2MulrvT` | non-unit / zero / `NaN` / `Inf` `c2r` (the API never validates `c*c+s*s == 1`) | [x] |
| 16 | `c2Mulxv` | identity `c2x`, pure translation, pure rotation, rotation+translation | [x] |
| 17 | `c2Mulxv` | non-unit rotation, non-finite translation | [x] |
| 18 | `c2BBVerts` | well-formed AABB (`min < max`), random | [x] |
| 19 | `c2BBVerts` | zero-extent (`min == max`), inverted (`min > max`), `NaN`/`Inf` corners | [x] |
| 20 | `c2MakeProxy` | `type = C2_TYPE_CIRCLE` → `radius = r`, `count = 1`, 1 vert; random circles incl. `r = 0`, `r < 0` | [x] |
| 21 | `c2MakeProxy` | `type = C2_TYPE_AABB` → `radius = 0`, `count = 4`, 4 verts; random + degenerate/inverted boxes | [x] |
| 22 | `c2MakeProxy` | `type = C2_TYPE_CAPSULE` → `radius = r`, `count = 2`, 2 verts; random incl. `a == b` | [x] |
| 23 | `c2MakeProxy` | full 40-byte output struct compared, including the 5 `verts` slots the C never writes (pre-seeded with a known pattern) | [x] |
| 24 | `c2Support` | `count = 1` (circle proxy), random `d` | [x] |
| 25 | `c2Support` | `count = 2` (capsule proxy), random `d`, incl. exact ties | [x] |
| 26 | `c2Support` | `count = 4` (AABB proxy), random `d`, incl. axis-aligned `d` producing ties | [x] |
| 27 | `c2Support` | `count = 8` (full `verts[8]`), random verts and `d` | [x] |
| 28 | `c2Support` | `d = (0,0)`; `verts`/`d` containing `NaN`; duplicate maximal verts | [x] |
| 29 | `c2GJKSimplexMetric` | `count = 1` → `0` | [x] |
| 30 | `c2GJKSimplexMetric` | `count = 2` → `c2Len(b.p - a.p)`, random `p` | [x] |
| 31 | `c2GJKSimplexMetric` | `count = 3` → `c2Det2(b-a, c-a)`, random `p` incl. collinear (`det == 0`) and `NaN` | [x] |
| 32 | `c22` | branch `v <= 0` (a is closest) — random configurations that hit it | [x] |
| 33 | `c22` | branch `u <= 0` (b is closest, `a = b` copy) | [x] |
| 34 | `c22` | branch else (interior, `count = 2`, `div = u+v`) | [x] |
| 35 | `c22` | fully random simplex bytes (all `sA`/`sB`/`p`/`u`/`iA`/`iB` random) — whole 152-byte struct compared | [x] |
| 36 | `c23` | branch 1: `vAB <= 0 && uCA <= 0` | [x] |
| 37 | `c23` | branch 2: `uAB <= 0 && vBC <= 0` | [x] |
| 38 | `c23` | branch 3: `uBC <= 0 && vCA <= 0` | [x] |
| 39 | `c23` | branch 4: `uAB>0 && vAB>0 && wABC<=0` | [x] |
| 40 | `c23` | branch 5: `uBC>0 && vBC>0 && uABC<=0` (`a=b; b=c` shuffle) | [x] |
| 41 | `c23` | branch 6: `uCA>0 && vCA>0 && vABC<=0` (`b=a; a=c` shuffle) | [x] |
| 42 | `c23` | branch 7: else (interior, `count = 3`) | [x] |
| 43 | `c23` | fully random simplex bytes, incl. degenerate `area == 0` and `NaN` | [x] |
| 44 | `c2D` | `count = 1`, `count = 2` with `det > 0` (`c2Skew`), `count = 2` with `det <= 0` (`c2CCW90`), `count = 3` | [x] |
| 45 | `c2L` | `count = 1`, `count = 2` (random `u`/`div`), `count = 3` (→ `(0,0)`) | [x] |
| 46 | `c2Witness` | `count = 1`, `2`, `3` with random `sA`/`sB`/`u`/`div` | [x] |
| 47 | `c2GJK` | circle↔circle, both transforms `NULL`, `use_radius = 0`, `cache = NULL`, all out-params non-`NULL` | [x] |
| 48 | `c2GJK` | circle↔circle, `use_radius = 1` | [x] |
| 49 | `c2GJK` | circle↔AABB, `NULL` transforms, `use_radius` 0 and 1 | [x] |
| 50 | `c2GJK` | circle↔capsule, `NULL` transforms, `use_radius` 0 and 1 | [x] |
| 51 | `c2GJK` | AABB↔AABB, `NULL` transforms, `use_radius` 0 and 1 | [x] |
| 52 | `c2GJK` | AABB↔capsule, `NULL` transforms, `use_radius` 0 and 1 | [x] |
| 53 | `c2GJK` | capsule↔capsule, `NULL` transforms, `use_radius` 0 and 1 | [x] |
| 54 | `c2GJK` | AABB↔circle / capsule↔circle / capsule↔AABB (reversed argument order — different `pA`/`pB` vertex counts) | [x] |
| 55 | `c2GJK` | all 9 type pairs × explicit identity `c2x` for both (vs. `NULL`) | [x] |
| 56 | `c2GJK` | all 9 type pairs × random *translation-only* `c2x` on A only | [x] |
| 57 | `c2GJK` | all 9 type pairs × random *rotation-only* `c2x` on B only | [x] |
| 58 | `c2GJK` | all 9 type pairs × random rotation+translation on **both** (exercises `c2MulrvT` in the support step) | [x] |
| 59 | `c2GJK` | all 9 type pairs × non-unit `c2r` transforms (scaling/degenerate rotation) | [x] |
| 60 | `c2GJK` | deep overlap (hit path, `s.count == 3`), all type pairs, `use_radius` 0 and 1 | [x] |
| 61 | `c2GJK` | exact touching (`dist == rA + rB`), circle↔circle and capsule↔capsule | [x] |
| 62 | `c2GJK` | far separation (`dist >> rA + rB`), all type pairs | [x] |
| 63 | `c2GJK` | coincident shapes (A and B identical), all type pairs | [x] |
| 64 | `c2GJK` | zero-size shapes: zero-radius circles, `min == max` AABB, `a == b` capsule with `r = 0` | [x] |
| 65 | `c2GJK` | inverted AABB (`min > max`) as A and/or B | [x] |
| 66 | `c2GJK` | negative radii on circle/capsule | [x] |
| 67 | `c2GJK` | subnormal-scale geometry (~1e-30) | [x] |
| 68 | `c2GJK` | huge-scale geometry (~1e30, `c2Dot` overflows to `Inf`) | [x] |
| 69 | `c2GJK` | `cache = NULL` vs. zeroed cache — full 36-byte cache struct compared after the call | [x] |
| 70 | `c2GJK` | warm cache: call twice with the same cache, shapes unchanged (second call reads the cache) | [x] |
| 71 | `c2GJK` | warm cache: call twice with the cache carried over and the shapes *moved* between calls | [x] |
| 72 | `c2GJK` | warm cache with `count = 1`, `2`, `3` and hand-built `iA`/`iB` index permutations valid for the proxy | [x] |
| 73 | `c2GJK` | warm cache + non-`NULL` transforms + `use_radius = 1` (full cross-product corner) | [x] |
| 74 | `c2GJK` | `iterations` non-`NULL`, checking the iteration count itself matches (not just the distance) | [x] |
| 75 | `c2GJK` | `outA`/`outB` non-`NULL`, witness points compared bit-for-bit | [x] |
| 76 | `c2GJK` | `use_radius = 1` with `rA = 0, rB > 0`, `rA > 0, rB = 0`, both `> 0`, both `0` | [x] |
| 77 | `c2AABBtoAABB` | random boxes: disjoint on x, disjoint on y, overlapping, edge-touching, zero-extent, inverted | [x] |
| 78 | `c2CircletoCircle` | random: separated, overlapping, exactly touching (`d2 == r2`, strict `<` → 0), concentric, `r = 0` | [x] |
| 79 | `c2CircletoAABB` | centre inside box, outside each of the 8 regions (4 edges + 4 corners), on the boundary, `r = 0`, inverted box | [x] |
| 80 | `c2CircletoCapsule` | `da < 0` branch, `db < 0` branch (perpendicular projection), `db >= 0` branch, degenerate `a == b`, `r = 0` | [x] |
| 81 | `c2AABBtoCapsule` | random overlapping / separated / touching; degenerate box and capsule (goes through `c2GJK`) | [x] |
| 82 | `c2CapsuletoCapsule` | parallel, crossing, collinear, separated, degenerate (`a == b`) capsules | [x] |
| 83 | `c2Collided` | all 9 valid `typeA × typeB` combinations with random shapes | [x] |
| 84 | `c2Collided` | all 9 combinations with degenerate / boundary shapes | [x] |
| 85 | `aabb` | random finite `(min_x, min_y, max_x, max_y)` — checks all 8 possible result bit patterns are reachable | [x] |
| 86 | `aabb` | boxes deliberately placed to hit each of the 3 result bits independently, plus inverted and zero-extent boxes | [x] |
| 87 | `aabb` | non-finite arguments (`NaN`, `±Inf`), subnormals, `±FLT_MAX` | [x] |

No binary/executable target is produced by either build (`c_src/CMakeLists.txt`
declares only `add_library(... SHARED ...)`; `translation/Cargo.toml` declares
only `crate-type = ["cdylib"]` with no `[[bin]]`), so the stdout-comparison
requirement does not apply.
