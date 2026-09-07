# CONFIGS.md — configuration surface table (valid inputs)

Axes the C code actually branches on, grepped from `c_src/src/lib.c`:

* **`C2_TYPE`** — `C2_TYPE_CIRCLE`(0) / `C2_TYPE_AABB`(1) / `C2_TYPE_CAPSULE`(2).
  Branched on in `c2MakeProxy` (L114) and twice in `c2Collided` (L577, L579/591/603).
  Determines the proxy shape: `count = 1 / 4 / 2`, `radius = c->r / 0 / c->r`.
* **`c2GJK` runtime options** — `ax_ptr` NULL vs. supplied, `bx_ptr` NULL vs.
  supplied, `use_radius` 0 vs. non-zero, `cache` NULL vs. supplied
  (and, when supplied, `count == 0` vs. warm), `outA`/`outB`/`iterations`
  NULL vs. supplied.
* **Simplex `count`** — 1 / 2 / 3 selects the arm in `c22`(no-op for 1),
  `c23`, `c2D`, `c2L`, `c2Witness`, `c2GJKSimplexMetric`.
* **`c22` branch shape** — `v<=0` / `u<=0` / else (3 arms).
* **`c23` branch shape** — 7 arms (3 vertex regions, 3 edge regions, 1 interior).
* **`c2Support` vertex count** — 1 (circle) / 2 (capsule) / 4 (AABB), plus
  the value-dependent `dot > dmax` tie path.
* **Input shape** — separated / touching / overlapping / identical / contained;
  zero radius; large radius; degenerate capsule (`a == b`); zero-extent AABB
  (`min == max`); rotation identity vs. arbitrary angle; translation zero vs.
  large; coordinates near 0 and at large magnitude.

Every row is exercised in `tests/valid_paths.rs` with **many randomized inputs**
(fixed-seed xorshift PRNG, `SEED = 0x2545F4914F6CDD1D`) unless the row is
inherently a single fixed configuration.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `c2V` | random finite / ±0 / ±inf / nan scalars | [x] |
| 2 | `c2Mulvs` | random vector × random scalar, incl. 0, inf, nan | [x] |
| 3 | `c2Maxv`, `c2Minv` | random pairs; equal components; nan operands (both argument orders) | [x] |
| 4 | `c2Clampv` | inside / below / above box; inverted box (`lo > hi`); nan | [x] |
| 5 | `c2Sub`, `c2Add` | random pairs, cancellation, large magnitudes | [x] |
| 6 | `c2Dot` | random pairs; overflow to inf; nan | [x] |
| 7 | `c2Det2` | random pairs; collinear (det 0); overflow | [x] |
| 8 | `c2Len` | random vectors; zero vector; huge vector (dot overflows to inf) | [x] |
| 9 | `c2Div` | random vector / random non-zero scalar | [x] |
| 10 | `c2Norm` | random non-zero vectors; near-subnormal vectors | [x] |
| 11 | `c2Neg`, `c2Skew`, `c2CCW90` | random vectors incl. ±0 (sign of zero) | [x] |
| 12 | `c2RotIdentity`, `c2xIdentity` | no inputs (fixed) | [x] |
| 13 | `c2Mulrv`, `c2MulrvT` | identity rotation; arbitrary `(cos θ, sin θ)`; un-normalised `c2r` | [x] |
| 14 | `c2Mulxv` | identity transform; rotation-only; translation-only; both | [x] |
| 15 | `c2BBVerts` | random AABB; inverted AABB; zero-extent AABB | [x] |
| 16 | `c2MakeProxy` | `type = CIRCLE` → count 1, radius = r (incl. r = 0, r < 0) | [x] |
| 17 | `c2MakeProxy` | `type = AABB` → count 4, radius 0, 4 verts from `c2BBVerts` | [x] |
| 18 | `c2MakeProxy` | `type = CAPSULE` → count 2, radius = r, verts a/b (incl. a == b) | [x] |
| 19 | `c2Support` | count 1 (circle proxy), random direction | [x] |
| 20 | `c2Support` | count 2 (capsule proxy), random direction, incl. direction ⟂ to a-b | [x] |
| 21 | `c2Support` | count 4 (AABB proxy), random direction, incl. axis-aligned ties | [x] |
| 22 | `c2Support` | count 8 (full `c2Proxy::verts`), random verts + direction | [x] |
| 23 | `c2GJKSimplexMetric` | `count = 1` → 0 | [x] |
| 24 | `c2GJKSimplexMetric` | `count = 2` → `c2Len(b.p - a.p)`, random simplex | [x] |
| 25 | `c2GJKSimplexMetric` | `count = 3` → `c2Det2(b-a, c-a)`, random simplex | [x] |
| 26 | `c22` | random simplex hitting arm `v <= 0` (origin beyond a) | [x] |
| 27 | `c22` | random simplex hitting arm `u <= 0` (origin beyond b, copies `b` into `a`) | [x] |
| 28 | `c22` | random simplex hitting the interior arm (count stays 2) | [x] |
| 29 | `c22` | fully random simplices (all arms mixed, incl. `a == b`) | [x] |
| 30 | `c23` | random simplices, all 7 arms covered (coverage asserted by arm histogram) | [x] |
| 31 | `c23` | degenerate simplices: two equal points, three equal points, collinear (`area == 0`) | [x] |
| 32 | `c2D` | `count = 1`; `count = 2` with `det > 0` (skew) and `det <= 0` (ccw90); `count = 3` | [x] |
| 33 | `c2L` | `count = 1`; `count = 2` with random `u`/`div` | [x] |
| 34 | `c2Witness` | `count = 1` / `2` / `3`, random `sA`/`sB`/`u`/`div` | [x] |
| 35 | `c2GJK` | CIRCLE×CIRCLE, `ax=bx=NULL`, `use_radius=1`, no cache — separated / touching / overlapping / concentric | [x] |
| 36 | `c2GJK` | CIRCLE×AABB, NULL transforms, `use_radius=1`, no cache, randomized | [x] |
| 37 | `c2GJK` | CIRCLE×CAPSULE, NULL transforms, `use_radius=1`, no cache, randomized | [x] |
| 38 | `c2GJK` | AABB×CIRCLE, NULL transforms, `use_radius=1`, no cache, randomized | [x] |
| 39 | `c2GJK` | AABB×AABB, NULL transforms, `use_radius=1`, no cache, randomized | [x] |
| 40 | `c2GJK` | AABB×CAPSULE, NULL transforms, `use_radius=1`, no cache, randomized | [x] |
| 41 | `c2GJK` | CAPSULE×CIRCLE, NULL transforms, `use_radius=1`, no cache, randomized | [x] |
| 42 | `c2GJK` | CAPSULE×AABB, NULL transforms, `use_radius=1`, no cache, randomized | [x] |
| 43 | `c2GJK` | CAPSULE×CAPSULE, NULL transforms, `use_radius=1`, no cache, randomized | [x] |
| 44 | `c2GJK` | all 9 type pairs, `use_radius = 0` (radius not subtracted) | [x] |
| 45 | `c2GJK` | all 9 type pairs, `ax_ptr` supplied (random rotation+translation), `bx_ptr` NULL | [x] |
| 46 | `c2GJK` | all 9 type pairs, `ax_ptr` NULL, `bx_ptr` supplied (random transform) | [x] |
| 47 | `c2GJK` | all 9 type pairs, **both** transforms supplied, random, `use_radius=1` | [x] |
| 48 | `c2GJK` | all 9 type pairs, both transforms supplied, `use_radius=0` | [x] |
| 49 | `c2GJK` | cold cache (`count = 0`) supplied → cache written back; verify metric/count/iA/iB/div byte-identical | [x] |
| 50 | `c2GJK` | warm cache: call twice with the same cache object, shapes unchanged (cache hit path `cache_was_read = 1`) | [x] |
| 51 | `c2GJK` | warm cache re-used after the shapes **moved** (cache indices stale but in range) | [x] |
| 52 | `c2GJK` | cache + transforms + `use_radius=1`, 3 successive calls sweeping a shape past another (progressive warm cache) | [x] |
| 53 | `c2GJK` | `outA`/`outB` NULL but `iterations` supplied, and vice versa (all 8 NULL combinations) | [x] |
| 54 | `c2GJK` | overlapping shapes → `hit = 1` path (`a = b`, `dist = 0`) | [x] |
| 55 | `c2GJK` | identical shapes (degenerate search direction guard) | [x] |
| 56 | `c2GJK` | zero-radius circle vs. zero-extent AABB (proxy count 1 vs. 4, all verts equal) | [x] |
| 57 | `c2GJK` | degenerate capsule (`a == b`, proxy count 2 with duplicate verts) vs. each type | [x] |
| 58 | `c2GJK` | large-magnitude coordinates (1e6…1e18) for all 9 type pairs | [x] |
| 59 | `c2AABBtoAABB` | separated on each of the 4 axes; touching exactly; overlapping; contained; randomized | [x] |
| 60 | `c2AABBtoCapsule` | randomized; capsule endpoint inside box; capsule crossing box; far away; degenerate capsule | [x] |
| 61 | `c2CapsuletoCapsule` | randomized; parallel; crossing; collinear-overlapping; both degenerate | [x] |
| 62 | `c2CircletoCircle` | randomized; exactly touching (`d2 == r2` → false); concentric; r = 0 | [x] |
| 63 | `c2CircletoAABB` | randomized; centre inside box; nearest point on face / on corner; r = 0 | [x] |
| 64 | `c2CircletoCapsule` | randomized hitting all three arms (`da < 0`, `db < 0`, else); degenerate capsule | [x] |
| 65 | `c2Collided` | all 9 valid `(typeA, typeB)` combinations, randomized shapes | [x] |
| 66 | `c2Collided` | AABB×CIRCLE and CAPSULE×CIRCLE / CAPSULE×AABB — the argument-**swapping** arms | [x] |
| 67 | `reverse_collide` | grid sweep over x,y ∈ [-160,160] step 2.5, r ∈ {0, 1, 5, 10, 20, 60} — exercises all 8 result bit patterns | [x] |
| 68 | `reverse_collide` | randomized floats incl. large magnitudes, ±0, subnormals | [x] |
| 69 | `reverse_collide` | boundary values that place the circle exactly tangent to each of the 3 fixed shapes | [x] |

| 70 | `c2GJK` | **externally synthesised** warm cache: random `metric` straddling the `-1e8` guard, `count` 0–3, random in-range `iA`/`iB`, random `div` — crossed with all 9 type pairs, random transforms, both `use_radius` values | [x] |
| 71 | `c2GJK`, `c2Collided` | 250,000-sample randomized cross-product of {type pair} × {tame, non-finite shapes} × {transform present/absent} × {`use_radius`} × {no / cold / synthetic warm cache} × {output-pointer NULL masks}, comparing distance, witness points, iterations and the written-back cache; the `iterations` histograms of C and Rust must be identical | [x] |
| 72 | `c22` → `c23` → `c2D` / `c2L` / `c2GJKSimplexMetric` / `c2Witness` | the composed low-level pipeline: 200,000 randomized simplices (count −1…3, tame and non-finite) pushed through `c22`, then `c23`, then read out by every consumer — catches bugs that per-function tests miss | [x] |

Rows 70–72 live in `tests/stress.rs`; rows 1–69 in `tests/valid_paths.rs`.

No binary/driver target exists (`c_src/CMakeLists.txt` builds only
`add_library(... SHARED src/lib.c)`; `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]`), so the "compare stdout of the two binaries" gate is
not applicable.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the only
configuration is the default one (`--no-default-features` is equivalent).
`./run_all.sh` enumerates the feature power set from `Cargo.toml` (empty here),
runs the suite for the default set *and* `--no-default-features`, and does so
against **two** builds of the C reference:

* the default CMake build (no `CMAKE_BUILD_TYPE`, i.e. `-O0`), and
* `-DCMAKE_BUILD_TYPE=Release` (`-O2`, where GCC is free to reassociate and
  contract floating point),

to prove the bit-exact agreement is a property of the translation and not of one
particular C optimisation level.
