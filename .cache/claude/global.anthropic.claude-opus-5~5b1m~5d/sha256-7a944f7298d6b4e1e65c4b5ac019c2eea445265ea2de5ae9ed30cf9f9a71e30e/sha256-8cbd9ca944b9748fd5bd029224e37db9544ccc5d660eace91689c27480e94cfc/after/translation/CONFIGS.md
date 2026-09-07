# CONFIGS.md — Phase B valid-input configuration surface table

Mechanically derived from the branches `c_src/src/lib.c` actually takes on
**valid** input (the mirror of `ERRORS.md`). Every row is exercised by a
differential test that calls both the C `.so` and the Rust `.so` through
`libloading` with **many randomized inputs at a fixed seed** and compares the
raw bit patterns of every output.

## Axes the C code branches on

| axis | values the C distinguishes | grep site |
|------|----------------------------|-----------|
| `C2_TYPE type` (proxy shape) | `C2_TYPE_CIRCLE` (1 vert, radius=r), `C2_TYPE_AABB` (4 verts, radius=0), `C2_TYPE_CAPSULE` (2 verts, radius=r) | `c2MakeProxy` `lib.c:113` |
| `typeA` × `typeB` in `c2Collided` | full 3×3 cross product, with **argument swapping** in the AABB/CAPSULE arms | `lib.c:576-615` |
| `ax_ptr` / `bx_ptr` | `NULL` (→ identity), identity, pure translation, pure rotation, rotation+translation | `lib.c:367-374` |
| `use_radius` | `0`, `1` (and any nonzero) | `lib.c:481` |
| `cache` | `NULL`, zeroed (`count==0`, cold start), warm (`count` 1/2/3 written by a previous call) | `lib.c:382-407`, `lib.c:499-508` |
| out params | `outA`/`outB`/`iterations` each `NULL` or non-`NULL` | `lib.c:509-514` |
| `c2Simplex.count` | `1`, `2`, `3` (and `0`/`4` → `default:`) | `c22`,`c23`,`c2D`,`c2L`,`c2Witness`,`c2GJKSimplexMetric` |
| `c2Support` `count` | `1`, `2`, `4`, `8` (the proxy sizes the library actually produces, plus the array max) | `lib.c:297-308` |
| geometric relation | disjoint-far, disjoint-near, touching, overlapping, one-contained-in-other, identical | all the `to`-predicates + GJK |
| shape degeneracy | zero radius, zero-area AABB (min==max), point capsule (a==b), inverted AABB | `c2BBVerts`, `c2CircletoCapsule` |
| value magnitude | tiny (~1e-30), normal (~±100), large (~1e18), exact-integer, mixed sign | float rounding paths |
| float *class* | normal, ±0.0, denormal, ±inf, quiet NaN, **signalling NaN** | every `addss`/`subss`/`mulss`/`divss` site: the C build is unoptimised, so each expression is ONE SSE instruction with a fixed destination register, and x86 returns `quiet(dst)` if `dst` is NaN else `quiet(src)`. The operand order therefore selects the returned NaN payload/sign, and had to be read off `objdump -d` of the C `.so` and mirrored exactly (see the `addss`/`subss`/`mulss`/`divss` helpers in `src/lib.rs`). |

Rows are the pruned cross-product of the above — only the combinations the C
treats differently.

## Table

### Leaf vector helpers

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random `(x,y)`: normal, ±0.0, tiny denormal, huge, exact integers | [x] |
| 2 | `c2Mulvs` | random vector × random scalar incl. `0.0`, `-0.0`, `1.0`, huge (overflow to inf) | [x] |
| 3 | `c2Add`, `c2Sub` | random vector pairs; cancellation (`a == b`), catastrophic cancellation, overflow | [x] |
| 4 | `c2Dot` | random vector pairs; orthogonal, parallel, magnitudes that overflow the product | [x] |
| 5 | `c2Det2` | random vector pairs; collinear (det == 0), sign-flipped ordering | [x] |
| 6 | `c2Len` | random vector; zero vector, unit vector, huge (sqrt of inf) | [x] |
| 7 | `c2Maxv`, `c2Minv` | random pairs, plus `a.x == b.x` ties and `±0.0` pairs (C's `?:` picks `b` on a tie) | [x] |
| 8 | `c2Clampv` | random `a` with `lo <= hi` (normal box), and `lo > hi` (inverted box) | [x] |
| 9 | `c2Neg`, `c2Skew`, `c2CCW90` | random vector; `±0.0` (sign of zero must match) | [x] |
| 10 | `c2Div` | random vector / random nonzero divisor, incl. tiny divisor (overflow) | [x] |
| 11 | `c2Norm` | random nonzero vector; already-unit vector; very large vector | [x] |
| 12 | `c2RotIdentity`, `c2xIdentity` | no inputs — constant result | [x] |
| 13 | `c2Mulrv`, `c2MulrvT` | random `c2r` normalized rotation × random vector; identity rotation; 90°/180° rotations; **unnormalized** `c2r` | [x] |
| 14 | `c2Mulxv` | random `c2x` (rotation+translation) × random vector; identity transform; translation-only | [x] |

### Proxy construction

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 15 | `c2BBVerts` | normal AABB (`min < max`) | [x] |
| 16 | `c2BBVerts` | degenerate AABB (`min == max`) and inverted AABB (`min > max`) | [x] |
| 17 | `c2MakeProxy` | `type = C2_TYPE_CIRCLE`, random circle (incl. `r == 0`, `r < 0`) → `count=1`, `radius=r` | [x] |
| 18 | `c2MakeProxy` | `type = C2_TYPE_AABB`, random AABB → `count=4`, `radius=0`, 4 corners | [x] |
| 19 | `c2MakeProxy` | `type = C2_TYPE_CAPSULE`, random capsule (incl. `a == b`) → `count=2`, `radius=r` | [x] |

### Simplex primitives (lowest-level GJK entry points, driven directly)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 20 | `c2GJKSimplexMetric` | `count = 1` | [x] |
| 21 | `c2GJKSimplexMetric` | `count = 2`, random `a.p`/`b.p` (incl. `a.p == b.p`) | [x] |
| 22 | `c2GJKSimplexMetric` | `count = 3`, random triangle; degenerate/collinear triangle (`area == 0`) | [x] |
| 23 | `c22` | `count = 2`, random `a.p`,`b.p` hitting the `v <= 0` branch (origin behind A) | [x] |
| 24 | `c22` | `count = 2`, hitting the `u <= 0` branch (origin beyond B) | [x] |
| 25 | `c22` | `count = 2`, hitting the interior branch (`u > 0 && v > 0`) | [x] |
| 26 | `c22` | `count = 2`, `a.p == b.p` (both `u` and `v` are 0 → `v <= 0` branch) | [x] |
| 27 | `c23` | `count = 3`, origin in vertex-A region (`vAB<=0 && uCA<=0`) | [x] |
| 28 | `c23` | `count = 3`, origin in vertex-B region (`uAB<=0 && vBC<=0`) | [x] |
| 29 | `c23` | `count = 3`, origin in vertex-C region (`uBC<=0 && vCA<=0`) | [x] |
| 30 | `c23` | `count = 3`, origin in edge-AB region | [x] |
| 31 | `c23` | `count = 3`, origin in edge-BC region (slot shift B→A, C→B) | [x] |
| 32 | `c23` | `count = 3`, origin in edge-CA region (slot shift A→B, C→A) | [x] |
| 33 | `c23` | `count = 3`, origin **inside** the triangle (final `else`, `count` stays 3) | [x] |
| 34 | `c23` | `count = 3`, collinear/degenerate triangle (`area == 0`, so `uABC = vABC = wABC = 0`). Note: this does **not** reach the final `else` — see `ERRORS.md` row 45; the C always exits via an earlier collapse/edge branch, and the Rust must pick the same one | [x] |
| 35 | `c23` | `count = 3`, both CW and CCW winding (sign of `area` flips all of `uABC/vABC/wABC`) | [x] |
| 36 | `c2D` | `count = 1` (returns `-a.p`) | [x] |
| 37 | `c2D` | `count = 2` with `c2Det2(ab, -a.p) > 0` (→ `c2Skew`) | [x] |
| 38 | `c2D` | `count = 2` with `c2Det2(ab, -a.p) <= 0` (→ `c2CCW90`) | [x] |
| 39 | `c2D` | `count = 3` (→ `(0,0)`) | [x] |
| 40 | `c2L` | `count = 1`, `count = 2` × random `div`/`u` weights (incl. `div == u_a + u_b`) | [x] |
| 41 | `c2Witness` | `count = 1` (copies `sA`/`sB` verbatim) | [x] |
| 42 | `c2Witness` | `count = 2` × random barycentric `u` and `div` | [x] |
| 43 | `c2Witness` | `count = 3` × random barycentric `u` and `div` | [x] |
| 44 | `c2Support` | `count = 1` (single vertex, always returns 0) | [x] |
| 45 | `c2Support` | `count = 2` (capsule proxy) × random direction | [x] |
| 46 | `c2Support` | `count = 4` (AABB proxy) × random direction, incl. axis-aligned directions that tie | [x] |
| 47 | `c2Support` | `count = 8` (full proxy array) × random direction; `d == (0,0)` (all dots 0 → index 0) | [x] |

### `c2GJK` — the low-level general entry point (full option cross-product)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 48 | `c2GJK` | typeA×typeB = CIRCLE×CIRCLE, `ax=bx=NULL`, `use_radius=0`, `cache=NULL`, all outs non-null | [x] |
| 49 | `c2GJK` | CIRCLE×CIRCLE, `use_radius=1` | [x] |
| 50 | `c2GJK` | CIRCLE×AABB, `use_radius` ∈ {0,1} | [x] |
| 51 | `c2GJK` | CIRCLE×CAPSULE, `use_radius` ∈ {0,1} | [x] |
| 52 | `c2GJK` | AABB×CIRCLE, `use_radius` ∈ {0,1} | [x] |
| 53 | `c2GJK` | AABB×AABB, `use_radius` ∈ {0,1} | [x] |
| 54 | `c2GJK` | AABB×CAPSULE, `use_radius` ∈ {0,1} | [x] |
| 55 | `c2GJK` | CAPSULE×CIRCLE, `use_radius` ∈ {0,1} | [x] |
| 56 | `c2GJK` | CAPSULE×AABB, `use_radius` ∈ {0,1} | [x] |
| 57 | `c2GJK` | CAPSULE×CAPSULE, `use_radius` ∈ {0,1} | [x] |
| 58 | `c2GJK` | all 9 type pairs, `ax_ptr = NULL` vs an explicitly-passed identity `c2x` (must be identical) | [x] |
| 59 | `c2GJK` | all 9 type pairs, `ax`/`bx` = pure translation (`r` identity, random `p`) | [x] |
| 60 | `c2GJK` | all 9 type pairs, `ax`/`bx` = pure rotation (random normalized `c2r`, `p == (0,0)`) | [x] |
| 61 | `c2GJK` | all 9 type pairs, `ax`/`bx` = rotation + translation, both different | [x] |
| 62 | `c2GJK` | all 9 type pairs, `ax`/`bx` with **unnormalized** `c2r` (scaling/shearing transform) | [x] |
| 63 | `c2GJK` | `cache = NULL` (no cache read, no cache write) | [x] |
| 64 | `c2GJK` | `cache` zeroed (`count == 0`): cold start, cache written back — assert the full 36-byte cache image matches | [x] |
| 65 | `c2GJK` | `cache` **warm-restarted**: call twice in a row with the same cache, second call reads it (`cache_was_read`) — assert both return values and both cache images | [x] |
| 66 | `c2GJK` | `cache` warm-restarted then the shapes are *moved* between the two calls (stale cache indices, still in range) | [x] |
| 67 | `c2GJK` | `cache` hand-built with `count = 1`, `count = 2`, `count = 3` and in-range `iA`/`iB`, random `div`/`metric` | [x] |
| 68 | `c2GJK` | `outA = NULL`, `outB` non-null (and vice versa), `iterations = NULL` | [x] |
| 69 | `c2GJK` | overlapping shapes (`hit = 1` path: `a = b`, `dist = 0`) for every type pair | [x] |
| 70 | `c2GJK` | shapes exactly touching (`dist == rA + rB`) → the `use_radius` else-branch | [x] |
| 71 | `c2GJK` | one shape fully contained in the other, for every type pair | [x] |
| 72 | `c2GJK` | identical shapes at identical transforms (`A == B`, degenerate zero-distance) | [x] |
| 73 | `c2GJK` | degenerate shapes: zero-radius circle, `min == max` AABB, `a == b` capsule, `r == 0` capsule | [x] |
| 74 | `c2GJK` | far-apart shapes (~1e6 apart) so the loop terminates on `c2Dot(d,d) < eps^2` | [x] |
| 75 | `c2GJK` | shapes arranged to drive many iterations (rotated AABB vs rotated AABB) — check `*iterations` | [x] |
| 76 | `c2GJK` | large-magnitude coordinates (~1e18) so `c2Dot` overflows to `inf` | [x] |
| 77 | `c2GJK` | `use_radius` = a nonzero non-1 value (`2`, `-1`) — must equal the `use_radius=1` result | [x] |

### Boolean shape predicates

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 78 | `c2AABBtoAABB` | random pairs: disjoint on x, disjoint on y, overlapping, touching (`A.max.x == B.min.x`), contained, identical, inverted boxes | [x] |
| 79 | `c2CircletoCircle` | random pairs: disjoint, touching (`d == r1+r2` exactly), overlapping, concentric, zero-radius, negative radius | [x] |
| 80 | `c2CircletoAABB` | circle centre inside the box, outside on a face, outside at a corner, exactly on the boundary; zero-radius; degenerate box | [x] |
| 81 | `c2CircletoCapsule` | `da < 0` branch (before A), `db < 0` branch (mid-segment projection), else branch (past B); point capsule (`a == b`) | [x] |
| 82 | `c2AABBtoCapsule` | disjoint, overlapping, touching, capsule fully inside the box, point capsule, zero-radius capsule | [x] |
| 83 | `c2CapsuletoCapsule` | parallel, perpendicular, crossing (X shape), collinear-overlapping, disjoint, identical, point capsules | [x] |

### Dispatcher and public entry point

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 84 | `c2Collided` | full 3×3 valid `typeA`×`typeB` cross product with random shapes (verifies the C's argument **swapping** in the AABB/CAPSULE arms) | [x] |
| 85 | `c2Collided` | each of the 9 pairs specifically in the disjoint / touching / overlapping / contained relations | [x] |
| 86 | `aabb` | random `(min_x,min_y,max_x,max_y)` in the ±200 range — hits all 8 possible return bit-patterns reachable there | [x] |
| 87 | `aabb` | box that hits the hard-coded circle only (result bit 0) | [x] |
| 88 | `aabb` | box that hits the hard-coded AABB only (result bit 1) | [x] |
| 89 | `aabb` | box that hits the hard-coded capsule only (result bit 2) | [x] |
| 90 | `aabb` | box that hits none (result `0`) and boxes that hit several (results `3`,`5`,`6`,`7` where reachable) | [x] |
| 91 | `aabb` | degenerate / inverted / huge / tiny boxes, ±0.0 coordinates | [x] |

### Float-class / NaN-payload propagation (`tests/fuzz_differential.rs`)

These rows are the reason `src/lib.rs` needs the `addss`/`subss`/`mulss`/`divss`
helpers: with plain Rust `+`/`*` LLVM commutes the operands, which silently
changes which NaN payload survives.  ~7.6 million randomized cases.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| 92 | every leaf helper (`c2V`, `c2Mulvs`, `c2Add`, `c2Sub`, `c2Dot`, `c2Det2`, `c2Len`, `c2Maxv`, `c2Minv`, `c2Clampv`, `c2Neg`, `c2Skew`, `c2CCW90`, `c2Div`, `c2Norm`, `c2Mulrv`, `c2MulrvT`, `c2Mulxv`, `aabb`) | operands drawn from **arbitrary 32-bit patterns**: normals, ±0.0, denormals, ±inf, quiet NaNs, signalling NaNs (`0x7f80_0001..`) — 4,000,000 cases | [x] |
| 93 | `c22`, `c23`, `c2D`, `c2L`, `c2Witness`, `c2GJKSimplexMetric`, `c2Support`, `c2BBVerts`, `c2MakeProxy` | simplex fields and `count` from arbitrary bit patterns (`count` incl. 0/1/2/3/4/-1/random) — 1,600,000 cases | [x] |
| 94 | `c2GJK` | arbitrary-bit shapes × all 9 type pairs × 5 transform kinds × random `use_radius` × cache none/zeroed/hand-made × 1-3 **chained** calls sharing one cache × random out-pointer nullability — 959,052 cases | [x] |
| 95 | `c2Collided` + all six predicates | arbitrary-bit shapes over the full 3×3 grid — 1,050,000 cases | [x] |
