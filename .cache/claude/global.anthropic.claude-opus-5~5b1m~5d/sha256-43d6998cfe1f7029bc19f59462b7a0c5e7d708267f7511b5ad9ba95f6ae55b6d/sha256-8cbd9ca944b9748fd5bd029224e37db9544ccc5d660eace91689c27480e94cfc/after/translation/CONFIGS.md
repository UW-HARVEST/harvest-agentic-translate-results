# CONFIGS.md — configuration surface table (valid inputs)

Axes the C code actually branches on:

* **`C2_TYPE`** for A and B: `CAPSULE=0`, `CIRCLE=1`, `AABB=2`, `POLY=3`
  (`c2Collide` / `c2MakeProxy` / `ptr_from_parts` switch on this).
* **`c2GJK` flags**: `use_radius ∈ {0,1}`, `ax_ptr/bx_ptr ∈ {NULL, identity,
  rotated, translated}`, `outA/outB/iterations ∈ {NULL, non-NULL}`,
  `cache ∈ {NULL, cold(count=0), warm(count=1/2/3)}`.
* **Simplex state** for `c22`/`c23`/`c2D`/`c2L`/`c2Witness`/`c2GJKSimplexMetric`:
  `count ∈ {0,1,2,3,4}`, plus the sub-region branches (7 branches in `c23`,
  3 in `c22`).
* **Input shapes**: separated / touching / shallow overlap / deep overlap /
  concentric; degenerate (zero radius, zero-length capsule, zero-area AABB,
  inverted AABB); axis-aligned vs diagonal; `dx<dy` vs `dx>=dy`;
  `x_overlap<y_overlap` vs not; sign of `d.x`/`d.y`.
* **Polygon shapes** for `c2CapsuletoPolyManifold`: `count = 3..8`, CCW quad
  from `c2BBVerts`, rotated `bx`, and each of the three `code` branches
  (0 = poly face reference, 1 = capsule side `ab_h0`, 2 = capsule side `ab_h1`).

Each row is run with **many randomised inputs** (fixed seed, xorshift PRNG) and
compared **bit-for-bit** between the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `c2V`, `c2Mulvs`, `c2Sub`, `c2Add`, `c2Neg`, `c2Skew`, `c2CCW90`, `c2Absv` | random finite floats (incl. ±0, denormals, huge) | [x] |
| 2 | same as #1 | non-finite inputs: `±inf`, `NaN` (quiet, signalling-bit, negative NaN) | [x] |
| 3 | `c2Maxv`, `c2Minv`, `c2Clampv` | random finite; `lo>hi`; equal components; ±0 pairs | [x] |
| 4 | `c2Maxv`, `c2Minv`, `c2Clampv` | NaN in `a`, in `b`, in both (ternary-picks-`b` behaviour) | [x] |
| 5 | `c2Dot`, `c2Det2`, `c2Len` | random finite; overflowing magnitudes (`1e30`); `inf*0` | [x] |
| 6 | `c2Div`, `c2Norm` | non-zero vector; zero vector (`NaN` result); `inf` component | [x] |
| 7 | `c2Dist`, `c2PlaneAt` | random `c2h`/`c2Poly`, `i ∈ [0,8)` | [x] |
| 8 | `c2RotIdentity`, `c2xIdentity` | no inputs (constant) | [x] |
| 9 | `c2Mulrv`, `c2MulrvT` | random `c2r` (unit rotations from `cos/sin θ`, and arbitrary non-unit) | [x] |
| 10 | `c2Mulrv`, `c2MulrvT` | `c2r` with NaN `c` or `s` (sign-flip-of-NaN in `-a.s`) | [x] |
| 11 | `c2Mulxv`, `c2MulxvT` | identity `c2x`; translation only; rotation only; both | [x] |
| 12 | `c2Intersect` | `da*db < 0`; `da == db` (`0/0` → NaN); `da == 0`; huge values | [x] |
| 13 | `c2BBVerts` | proper AABB; `min == max`; inverted (`min > max`); NaN corners | [x] |
| 14 | `c2Norms` | `count = 0,1,2,3,4,8`; CCW quad; zero-length edge (NaN normal) | [x] |
| 15 | `c2Support` | `count = 1,2,4,8`; ties (equal dots); NaN `d`; `count = 0` | [x] |
| 16 | `c2MakeProxy` | `type = CIRCLE` (radius/count 1) | [x] |
| 17 | `c2MakeProxy` | `type = AABB` (radius 0/count 4 + `c2BBVerts`) | [x] |
| 18 | `c2MakeProxy` | `type = CAPSULE` (radius/count 2) | [x] |
| 19 | `c2GJKSimplexMetric` | `count = 1` (→0), `2` (length), `3` (det) | [x] |
| 20 | `c22` | branch `v<=0`; branch `u<=0`; branch interior (2-simplex) | [x] |
| 21 | `c23` | all 7 branches: 3× vertex-region, 3× edge-region, 1× interior | [x] |
| 22 | `c2D` | `count = 1`; `count = 2` with `det>0`; `count = 2` with `det<=0`; `count = 3` | [x] |
| 23 | `c2L` | `count = 1`; `count = 2`; `count = 3` (default) | [x] |
| 24 | `c2Witness` | `count = 1, 2, 3`; random `div`; `div` near 0 | [x] |
| 25 | `c2GJK` | CIRCLE vs CIRCLE, `use_radius=0`, all transforms NULL, no cache | [x] |
| 26 | `c2GJK` | CIRCLE vs CIRCLE, `use_radius=1` (both radius-shrink and midpoint-collapse branches) | [x] |
| 27 | `c2GJK` | CIRCLE vs CAPSULE / CAPSULE vs CIRCLE, `use_radius ∈ {0,1}` | [x] |
| 28 | `c2GJK` | CAPSULE vs CAPSULE (parallel, crossing, collinear), `use_radius ∈ {0,1}` | [x] |
| 29 | `c2GJK` | AABB vs AABB (separated, touching, overlapping → `hit`), `use_radius ∈ {0,1}` | [x] |
| 30 | `c2GJK` | AABB vs CIRCLE / AABB vs CAPSULE, `use_radius ∈ {0,1}` | [x] |
| 31 | `c2GJK` | non-NULL identity `ax_ptr`/`bx_ptr` (must equal the NULL case) | [x] |
| 32 | `c2GJK` | rotated + translated `ax`, `bx` (unit `c2r` from `cos/sin`) | [x] |
| 33 | `c2GJK` | `outA = NULL`, `outB = NULL`, `iterations = NULL` (any subset) | [x] |
| 34 | `c2GJK` | `cache` non-NULL, cold (`count = 0`) — cache written on exit | [x] |
| 35 | `c2GJK` | `cache` non-NULL, warm — cache round-tripped from a previous call, same shapes | [x] |
| 36 | `c2GJK` | `cache` non-NULL, warm — cache reused after moving the shapes (metric mismatch path) | [x] |
| 37 | `c2GJK` | `cache` warm with hand-built `count = 1/2/3` and arbitrary `metric`/`div` | [x] |
| 38 | `c2CircletoCircleManifold` | separated; touching (`d2 == r*r`); overlapping; concentric (`l == 0`); zero radii | [x] |
| 39 | `c2CircletoAABBManifold` | outside; touching; shallow overlap (`d2 != 0`); center inside (`d2 == 0`, both `x_overlap<y_overlap` and not, all 4 sign combinations of `d.x`/`d.y`) | [x] |
| 40 | `c2CircletoAABBManifold` | degenerate AABB (`min == max`), inverted AABB | [x] |
| 41 | `c2CircletoCapsuleManifold` | separated; overlapping (`d != 0`); `d == 0` (center on segment); zero-length capsule (`B.a == B.b` → NaN normal) | [x] |
| 42 | `c2AABBtoAABBManifold` | separated in x (`dx<0`); separated in y (`dy<0`); overlap with `dx<dy` and `d.x<0`; `dx<dy` and `d.x>=0`; `dx>=dy` and `d.y<0`; `dx>=dy` and `d.y>=0`; identical boxes; inverted boxes | [x] |
| 43 | `c2CapsuletoCapsuleManifold` | separated; parallel overlap; crossing (`d == 0`); collinear; zero-length A; zero-length B | [x] |
| 44 | `c2CapsuletoPolyManifold` | `bx_ptr = NULL`, CCW triangle, `code = 0` branch | [x] |
| 45 | `c2CapsuletoPolyManifold` | `bx_ptr = NULL`, CCW quad, `code = 1` branch | [x] |
| 46 | `c2CapsuletoPolyManifold` | `bx_ptr = NULL`, CCW quad, `code = 2` branch | [x] |
| 47 | `c2CapsuletoPolyManifold` | non-NULL identity `bx`; rotated `bx`; translated `bx` | [x] |
| 48 | `c2CapsuletoPolyManifold` | poly `count = 3,4,5,6,7,8` (regular CCW n-gons, random radius/rotation) | [x] |
| 49 | `c2CapsuletoPolyManifold` | far away (`d >= 1e-6` and `d >= A.r`) → `count = 0` | [x] |
| 50 | `c2CapsuletoPolyManifold` | shallow (`1e-6 <= d < A.r`) → single-point radius branch | [x] |
| 51 | `c2CapsuletoPolyManifold` | degenerate capsule (`A.a == A.b`) — NaN `ab`, `index = -1` OOB path | [x] |
| 52 | `c2AABBtoCapsuleManifold` | proper AABB, capsule separated / overlapping / crossing | [x] |
| 53 | `c2AABBtoCapsuleManifold` | degenerate AABB (`min == max`; zero width; zero height) — NaN-normal `c2Incident` path | [x] |
| 54 | `c2Collide` | `typeA × typeB` full 3×3 cross product of {CAPSULE, CIRCLE, AABB}, random shapes | [x] |
| 55 | `omni_manifold` | full 3×3 cross product of {CAPSULE, CIRCLE, AABB}, random parameter tuples | [x] |
| 56 | `omni_manifold` | grid sweep: shapes at increasing separation from deep overlap to fully separated, for every type pair | [x] |
| 57 | `omni_manifold` | degenerate parameter tuples: zero radius, zero-length capsule, zero-area AABB, inverted AABB, negative radius | [x] |

## Results

Every row above is checked off. Test files:

| rows | test file |
|------|-----------|
| 1–24 | `tests/phase_b_low.rs` (17 tests) |
| 25–37 | `tests/phase_b_gjk.rs` (8 tests) |
| 38–57 | `tests/phase_b_manifold.rs` (13 tests) |

Each row is driven with many randomised inputs from a fixed-seed xorshift64\*
PRNG (per-row seeds, so a failure is reproducible), and every output — return
value, `c2v`/`c2h`/`c2x` fields, all four `c2Manifold` fields, the mutated
`c2GJKCache`, and the `iterations` out-parameter — is compared **bit-for-bit**
(`f32::to_bits`), so NaN payloads and the sign of zero are part of the contract.

Rows 44–57 route through `c2GJK(..., C2_TYPE_POLY, ...)`, which reads
uninitialised memory in the C source; see the row-53 discussion in `ERRORS.md`
for how the harness makes that comparison well defined.

The project builds **no binary executable** (`Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]`, and `c_src/CMakeLists.txt` only
`add_library(... SHARED ...)`), so there is no stdout comparison to make.

The crate declares **no cargo features**, so the feature matrix is
`{default} == {--no-default-features}`; `run_tests.sh` iterates it generically
and additionally re-runs the whole suite against the **dev-profile** cdylib,
because `opt-level` changes how the NaN-propagation helpers in `src/lib.rs` are
compiled.
