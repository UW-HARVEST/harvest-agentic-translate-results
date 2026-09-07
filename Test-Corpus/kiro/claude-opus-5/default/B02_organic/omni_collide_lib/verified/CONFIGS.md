# CONFIGS.md — configuration surface table (Phase B gate)

Derived mechanically from `c_src/include/lib.h` + every `if` / `switch` in
`c_src/src/lib.c`. Every row is exercised through **both** `.so` files via
`libloading` with **many randomized inputs at a fixed seed**, and asserted
byte-identical (`f32::to_bits`, not `==`, so `NaN` and `-0.0` are compared
exactly).

## Axes the C actually branches on

| axis | values the C distinguishes | where |
|---|---|---|
| `C2_TYPE` (a) | `CAPSULE=0`, `CIRCLE=1`, `AABB=2`, out-of-range | `c2MakeProxy`, `c2Collided`, `ptr_from_parts` |
| `C2_TYPE` (b) | same 4 | same |
| proxy vertex count | `1` (circle), `2` (capsule), `4` (AABB) | `c2MakeProxy` |
| proxy radius | `0` (AABB) vs `r` (circle/capsule) | `c2MakeProxy`, `use_radius` block |
| `use_radius` | `0`, `1` | `c2GJK` L478 |
| `ax_ptr` / `bx_ptr` | `NULL` (→identity), identity, pure translation, pure rotation, rotation+translation, non-unit `c2r` | `c2GJK` L363/367, `c2Mulxv`, `c2MulrvT` |
| `cache` | `NULL`, cold (`count==0`), warm (written by a previous call), hand-forged | `c2GJK` L378/495 |
| `outA`/`outB`/`iterations` | `NULL`, non-`NULL` | `c2GJK` L505-509 |
| simplex `count` | `0`,`1`,`2`,`3`,`4`,negative,large | `c22`,`c23`,`c2D`,`c2L`,`c2Witness`,`c2GJKSimplexMetric` |
| `c22` branch | `v<=0`, `u<=0`, else | `c22` |
| `c23` branch | 7 distinct branches | `c23` |
| `c2Support` count | `0`,`1`,`2`,`4`,`8` | `c2Support` |
| shape relation | deep overlap, contained, exactly touching, just-apart, far apart | all `c2*to*` |
| degeneracy | zero radius, capsule `a==b`, AABB `min==max`, AABB inverted, zero-length vectors | all |
| float class | normal, `±0.0`, subnormal, huge (`1e30`), `±inf`, `NaN` | all |

## Rows

### Level 0 — pure vector/rotation primitives (randomized over all float classes)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random `(x,y)` incl. `±0`, subnormal, `±inf`, `NaN` | [x] |
| 2 | `c2Sub`, `c2Add` | random pairs, incl. `inf-inf` → `NaN` and `±0` sign rules | [x] |
| 3 | `c2Mulvs` | random vec × random scalar, incl. `0*inf` → `NaN` | [x] |
| 4 | `c2Dot` | random pairs; both terms `inf` with opposite sign → `NaN` | [x] |
| 5 | `c2Det2` | random pairs; cancellation and `NaN` propagation | [x] |
| 6 | `c2Maxv`, `c2Minv` | random pairs; ties, `±0` (C uses `>`/`<` so `-0` vs `+0` picks the *second* arg), `NaN` (comparison false → picks `b`) | [x] |
| 7 | `c2Clampv` | `lo<=hi` normal, `lo>hi` **inverted** (C does not validate), `NaN` in each of the 3 args | [x] |
| 8 | `c2Len` | random vec; zero vec → `0`; huge vec → `inf` overflow in `c2Dot`; `NaN` | [x] |
| 9 | `c2Div` | random vec ÷ random scalar incl. `0` (→`±inf`/`NaN`), `inf`, `NaN` | [x] |
| 10 | `c2Norm` | random vec; zero vec (`0/0`→`NaN`); huge vec (`inf` len → `0`); `NaN` | [x] |
| 11 | `c2Neg`, `c2Skew`, `c2CCW90` | random vec incl. `±0` sign flip and `NaN` | [x] |
| 12 | `c2RotIdentity`, `c2xIdentity` | no inputs — bit-exact constant check | [x] |
| 13 | `c2Mulrv`, `c2MulrvT` | random `c2r`: unit `(cos,sin)`, non-unit, zero, `NaN`; × random vec. `c2MulrvT` must be the transpose | [x] |
| 14 | `c2Mulxv` | random `c2x`: identity, pure translation, pure rotation, rotation+translation, non-unit rot | [x] |

### Level 1 — proxy construction (`c2MakeProxy` / `c2BBVerts`), buffer pre-zeroed

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 15 | `c2BBVerts` | normal AABB (`min<max`); writes 4 corners CCW | [x] |
| 16 | `c2BBVerts` | degenerate AABB `min==max`; and **inverted** `min>max` | [x] |
| 17 | `c2MakeProxy` | `type=CIRCLE` → `radius=r`, `count=1`, `verts[0]=p` (random, incl. `r=0`, `r<0`, `r=NaN`) | [x] |
| 18 | `c2MakeProxy` | `type=CAPSULE` → `radius=r`, `count=2` (random, incl. degenerate `a==b`) | [x] |
| 19 | `c2MakeProxy` | `type=AABB` → `radius=0`, `count=4` (random, incl. inverted) | [x] |
| 20 | `c2MakeProxy` | `type` out of range (`3`, `-1`, `INT_MAX`, `INT_MIN`) → proxy left untouched (pre-zeroed buffer makes this comparable) | [x] |

### Level 2 — simplex primitives, driven with hand-forged `c2Simplex` structs

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 21 | `c2GJKSimplexMetric` | `count=1` → 0 | [x] |
| 22 | `c2GJKSimplexMetric` | `count=2` → `c2Len(b.p-a.p)` (randomized `p`s) | [x] |
| 23 | `c2GJKSimplexMetric` | `count=3` → `c2Det2(b-a, c-a)` (randomized) | [x] |
| 24 | `c2GJKSimplexMetric` | `count` ∈ {`0`,`4`,`-1`,`999`} → `default:` = 0 | [x] |
| 25 | `c22` | branch `v<=0` (origin beyond `a`) — asserts full 152-byte struct after the call | [x] |
| 26 | `c22` | branch `u<=0` (origin beyond `b`) — `a` overwritten by `b` | [x] |
| 27 | `c22` | branch else (origin inside the segment) → `count=2`, `div=u+v` | [x] |
| 28 | `c22` | randomized `a.p`/`b.p` over all float classes, whichever branch results | [x] |
| 29 | `c22` | degenerate `a.p == b.p` (u=v=0 → first branch) | [x] |
| 30 | `c23` | branch 1: `vAB<=0 && uCA<=0` (vertex A region) | [x] |
| 31 | `c23` | branch 2: `uAB<=0 && vBC<=0` (vertex B region) | [x] |
| 32 | `c23` | branch 3: `uBC<=0 && vCA<=0` (vertex C region) | [x] |
| 33 | `c23` | branch 4: `uAB>0 && vAB>0 && wABC<=0` (edge AB) | [x] |
| 34 | `c23` | branch 5: `uBC>0 && vBC>0 && uABC<=0` (edge BC, shifts a←b, b←c) | [x] |
| 35 | `c23` | branch 6: `uCA>0 && vCA>0 && vABC<=0` (edge CA, b←a, a←c) | [x] |
| 36 | `c23` | branch 7 (else): interior → `count=3`, barycentric `u`s | [x] |
| 37 | `c23` | randomized triangles (incl. degenerate/collinear, zero `area`, duplicated vertices) — hits the branches by chance in the C's own order | [x] |
| 38 | `c2D` | `count=1` → `-a.p` | [x] |
| 39 | `c2D` | `count=2`, `c2Det2(ab, -a.p) > 0` → `c2Skew(ab)` | [x] |
| 40 | `c2D` | `count=2`, `c2Det2(ab, -a.p) <= 0` → `c2CCW90(ab)` | [x] |
| 41 | `c2D` | `count` ∈ {`3`,`0`,`4`,`-1`} → `c2V(0,0)` | [x] |
| 42 | `c2L` | `count=1` → `a.p`; `count=2` → weighted; `div=0` → `inf`/`NaN`; `count` other → `(0,0)` | [x] |
| 43 | `c2Witness` | `count=1`, `2`, `3`, and `default`; `div` normal / `0` / `NaN` | [x] |
| 44 | `c2Support` | `count=1` (circle proxy shape) | [x] |
| 45 | `c2Support` | `count=2` (capsule proxy shape), randomized `d` | [x] |
| 46 | `c2Support` | `count=4` (AABB proxy shape), randomized `d` incl. axis-aligned ties | [x] |
| 47 | `c2Support` | `count=8` (full `verts[8]`), randomized | [x] |
| 48 | `c2Support` | `count=0` (still reads `verts[0]`) → `0`; `count` negative → `0` | [x] |
| 49 | `c2Support` | `d = (0,0)` and `d` containing `NaN` → all comparisons false → `0` | [x] |

### Level 3 — `c2GJK` (the low-level entry point) — full option cross-product

Each row runs 200+ randomized shape pairs at a fixed seed and compares the
returned `f32` bits **plus** `*outA`, `*outB`, `*iterations`, and the whole
`c2GJKCache` when supplied.

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 50 | `c2GJK` | all 9 `(typeA,typeB)` pairs × `use_radius=0` × null transforms × null cache | [x] |
| 51 | `c2GJK` | all 9 `(typeA,typeB)` pairs × `use_radius=1` × null transforms × null cache | [x] |
| 52 | `c2GJK` | all 9 pairs × `use_radius=1` × **identity** transforms passed explicitly (not `NULL`) | [x] |
| 53 | `c2GJK` | all 9 pairs × pure **translation** `ax`, `bx` | [x] |
| 54 | `c2GJK` | all 9 pairs × pure **rotation** `ax`, `bx` (unit `c2r` from a random angle) | [x] |
| 55 | `c2GJK` | all 9 pairs × rotation **+** translation on both | [x] |
| 56 | `c2GJK` | all 9 pairs × **non-unit / zero / NaN** `c2r` (C never normalizes) | [x] |
| 57 | `c2GJK` | asymmetric: `ax_ptr = NULL` but `bx_ptr` non-null, and vice versa | [x] |
| 58 | `c2GJK` | `outA=NULL, outB=NULL, iterations=NULL` (return value only) | [x] |
| 59 | `c2GJK` | `outA` non-null, `outB=NULL`; and the mirror | [x] |
| 60 | `c2GJK` | `iterations` non-null — compare the iteration count, incl. the `iter==20` cap | [x] |
| 61 | `c2GJK` | `cache` non-null, **cold** (`count=0`) — compare the written-back cache | [x] |
| 62 | `c2GJK` | `cache` non-null, **warm**: call twice with the same cache struct, same shapes | [x] |
| 63 | `c2GJK` | `cache` **warm across moving shapes**: 10-step sweep reusing one cache (the real consumer pattern) | [x] |
| 64 | `c2GJK` | hand-forged cache: `count` ∈ {1,2,3}, `iA`/`iB` in `[0,8)`, random `metric`/`div` | [x] |
| 65 | `c2GJK` | shape relation: **deep overlap** (hit path, `s.count==3` → `dist=0`, `a=b`) | [x] |
| 66 | `c2GJK` | shape relation: **exactly touching** | [x] |
| 67 | `c2GJK` | shape relation: **just apart** (`dist` slightly > `rA+rB`) — exercises the radius-shrink branch | [x] |
| 68 | `c2GJK` | shape relation: **far apart** (early `d1>d0` / epsilon breaks) | [x] |
| 69 | `c2GJK` | shape relation: one shape **fully contained** in the other | [x] |
| 70 | `c2GJK` | shape relation: **coincident** shapes (identical A and B) | [x] |
| 71 | `c2GJK` | degenerate shapes: circle `r=0`, capsule `a==b`, capsule `r=0`, AABB `min==max`, AABB inverted | [x] |
| 72 | `c2GJK` | huge coordinates (`±1e30`) → `inf` inside `c2Dot`/`c2Len` | [x] |
| 73 | `c2GJK` | `NaN` / `±inf` coordinates and radii | [x] |
| 74 | `c2GJK` | out-of-range `typeA`/`typeB` passed **directly** to `c2GJK`. `c2MakeProxy` writes nothing, so the C's `c2Proxy pA;` keeps an uninitialised `count` and `c2Support` loops that many times over an 8-element array — arbitrary reads that segfault nondeterministically. **UB: characterised in `probe_ub_isolated.rs::invalid_type_straight_into_c2gjk` (each library in a forked child; the Rust, which zero-fills its proxy, survives all 144 configurations and never crashes where the C survives).** The defined public behaviour for an unknown tag is rows 86-87/94. | [x] |
| 75 | `c2GJK` | negative radii (`r < 0`) — `rA+rB` negative flips the `dist > rA+rB` test | [x] |

### Level 4 — pairwise boolean predicates

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 76 | `c2AABBtoAABB` | randomized pairs: overlap, touch, all 4 separation axes, inverted boxes, `NaN` | [x] |
| 77 | `c2CircletoCircle` | randomized: overlap, exact touch (strict `<` → miss), apart, `r=0`, `r<0`, `NaN` | [x] |
| 78 | `c2CircletoAABB` | randomized: center inside, on edge, on corner, outside; `r=0`; inverted AABB | [x] |
| 79 | `c2CircletoCapsule` | branch `da < 0` (before endpoint `a`) | [x] |
| 80 | `c2CircletoCapsule` | branch `da>=0, db<0` (projects onto the segment; divides by `c2Dot(n,n)`) | [x] |
| 81 | `c2CircletoCapsule` | branch `da>=0, db>=0` (past endpoint `b`) | [x] |
| 82 | `c2CircletoCapsule` | degenerate capsule `a==b` → `n=(0,0)` → `0/0` `NaN` on the mid branch | [x] |
| 83 | `c2AABBtoCapsule` | randomized: overlap / touch / apart, degenerate capsule, inverted AABB (goes through `c2GJK` with `use_radius=1`) | [x] |
| 84 | `c2CapsuletoCapsule` | randomized: crossing, parallel, collinear, coincident, degenerate (point capsules) | [x] |

### Level 5-6 — dispatchers and the public one-shot API

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 85 | `c2Collided` | all 9 valid `(typeA,typeB)` combinations, randomized shapes — verifies the C's **argument-swapping** in the mixed cases | [x] |
| 86 | `c2Collided` | `typeA` valid × `typeB` out of range (all 3 × several bad ints) → `0` | [x] |
| 87 | `c2Collided` | `typeA` out of range (incl. `INT_MIN`/`INT_MAX`) × any `typeB` → `0`, `B` never read | [x] |
| 88 | `ptr_from_parts` | `typ=CIRCLE` → heap `c2Circle{p,r}`; contents compared field-by-field | [x] |
| 89 | `ptr_from_parts` | `typ=AABB` → heap `c2AABB{min,max}` | [x] |
| 90 | `ptr_from_parts` | `typ=CAPSULE` → heap `c2Capsule{a,b,r}` | [x] |
| 91 | `ptr_from_parts` | `typ` out of range → C is UB (no return stmt); only the non-crash + `omni_collide` agreement is asserted (see ERRORS.md rows 41-42) | [x] |
| 92 | `omni_collide` | all 9 valid `(type_a,type_b)` × 2000 randomized 5-float parameter tuples per pair | [x] |
| 93 | `omni_collide` | all 9 valid pairs × structured shapes (overlap / touch / apart / contained / coincident) | [x] |
| 94 | `omni_collide` | out-of-range `type_a` and/or `type_b` (`3`,`4`,`-1`,`INT_MAX`,`INT_MIN`) | [x] |
| 95 | `omni_collide` | all-`0.0` parameters, all-`NaN`, all-`±inf`, subnormal, `±0.0` mixes | [x] |
| 96 | `omni_collide` | unused trailing parameters vary (e.g. `a4`,`a5` for a circle) — must not affect the result | [x] |

### Binary / driver

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — there
is **no executable target**, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. The "compare C and Rust stdout"
item is therefore **not applicable** to this project.

### Feature combinations

`translation/Cargo.toml` has no `[features]` section → one configuration only.
All rows above are the default (and only) combination.

## Row → test-file mapping (Phase B gate)

All 96 rows are checked off above. Run `cargo test --release` (or
`translation/scripts/phase_d.sh` for the full Phase D sweep) to verify.

| rows | test file |
|---|---|
| 1-14 | `tests/level0_vectors.rs` — one test per row |
| 15-20 | `tests/level1_proxy.rs` |
| 21-49 | `tests/level2_simplex.rs` |
| 50-75 | `tests/level3_gjk.rs` (row 74's UB part in `tests/probe_ub_isolated.rs`) |
| 76-84 | `tests/level4_pairwise.rs` |
| 85-96 | `tests/level5_dispatch.rs` |

Supporting suites:

| suite | purpose |
|---|---|
| `tests/common/mod.rs` | loads both `.so`s via `libloading`, exposes all 39 exports as raw `extern "C"` pointers, bit-exact comparators, seeded PCG32 generators |
| `tests/nan_payload.rs` | pins down the single tolerated difference (NaN payload bits) and proves it cannot affect any `int` result |
| `tests/errors_phase_c.rs` | Phase C — one test per `ERRORS.md` row |
| `tests/iter_cap_search.rs` | establishes that the `iter < 20` cap is unreachable (max 4 over 300 000 calls) |
| `tests/probe_ub_isolated.rs` | fork-isolated characterization of the C's two crashing UB paths |

## Branch-coverage assertions

Rather than hoping randomized inputs reach every branch, several tests classify
each input by replicating the C's branch conditions and then **fail if any branch
was never exercised**:

* `c23` — all **7** branches (3 vertex regions, 3 edge regions, interior);
  `level2_simplex.rs::row30_36_c23_all_seven_branches`.
* `c22` — all 3 branches, each with its own test (rows 25-27).
* `c2D` — both sides of the `c2Det2(ab, -a.p) > 0` test (rows 39-40).
* `c2CircletoCapsule` — all 3 branches;
  `level4_pairwise.rs::rows79_82_circle_to_capsule`.
* `c2GJK`'s `use_radius` block — both the midpoint-collapse and the
  radius-shrink branch; `errors_phase_c.rs::err_rows22_24_use_radius_block`.
* `c2AABBtoCapsule` / `c2CapsuletoCapsule` — both the accept and reject outcome;
  `errors_phase_c.rs::err_rows26_27_*`.

## Randomization

Every row uses a fixed-seed PCG32 generator (`common::SEED`, per-test salted), so
failures reproduce exactly. Input generators deliberately span the whole float
domain: normals, `±0.0`, subnormals, `f32::MIN`/`MAX`, `±inf`, canonical NaN, and
raw random bit patterns — plus shape-specific degeneracies (zero and negative
radii, point capsules, empty and inverted AABBs) and non-unit / zero / NaN
rotations, since the C never normalizes a `c2r`.
