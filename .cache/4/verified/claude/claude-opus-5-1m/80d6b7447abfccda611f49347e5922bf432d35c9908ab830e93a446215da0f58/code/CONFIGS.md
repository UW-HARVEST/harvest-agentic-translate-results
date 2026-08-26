# CONFIGS.md — Phase B configuration surface table

## Build-time configuration

`translated_rust/Cargo.toml` has **no `[features]` section**, so the complete set of
feature combinations is the single empty one:

| # | combination | `cargo check` | `cargo test` (debug) | `cargo test --release` |
|---|-------------|---------------|----------------------|------------------------|
| F1 | `--no-default-features` (== default == no features) | PASS | PASS | PASS |

`./run_all_features.sh` enumerates the `[features]` table mechanically and runs the
whole suite once per combination, so adding a feature automatically extends the sweep.
Both profiles are verified because `[profile.release]` turns on optimisation, and
optimisation is exactly what could defeat the NaN-propagation helpers in `src/fp.rs`.

`c_src/CMakeLists.txt` has no `option()`, no `add_definitions()` and no
`target_compile_definitions()`; `lib.c` contains no `#ifdef` at all and the library
is built from one source file. So there is a single C configuration too, and
`cargo build`'s default is the only Rust configuration. Everything below is
therefore **runtime** configuration, which is where this library's real branch
surface lives.

## Runtime configuration axes (derived from the C `if` / `switch` / `?:` sites)

| axis | values the C code distinguishes |
|---|---|
| `C2_TYPE` (shape kind) | `CAPSULE=0`, `CIRCLE=1`, `AABB=2`, `POLY=3` (no `case`), out-of-range `int` |
| transform pointer | `NULL` (⇒ `c2xIdentity`) vs non-`NULL` |
| rotation | identity (`c=1,s=0`) vs arbitrary `(c,s)` vs non-unit / zero `(0,0)` |
| `use_radius` | `0` vs non-zero |
| `cache` | `NULL`, `count == 0`, `count ∈ {1,2,3}`, round-tripped from a previous call |
| out params | `outA` / `outB` / `iterations` each `NULL` vs non-`NULL` (2³ combinations) |
| `c2Simplex.count` | `0`, `1`, `2`, `3`, out-of-range — every `switch` in `gjk.rs` reads it |
| `c2Poly.count` | `0`, `1`, `2`, `3`, `4`, `5`, `6`, `7`, `8` |
| `c2Support` / `c2Norms` `count` | `0`, `1`, `2`, `4`, `8`, negative |
| geometric regime | far apart, near-touching, exactly touching, shallow overlap, deep overlap, coincident/concentric, degenerate (zero-extent / zero-length) |
| float value class | normal, `+0.0`, `-0.0`, denormal, `FLT_MAX`, `±inf`, `±qNaN` (random payload), `±sNaN` |
| `c2CapsuletoPolyManifold` `code` | `0` (face), `1` (`ab_h0` side), `2` (`ab_h1` side) |
| `c22` branch | vertex-A, vertex-B, edge |
| `c23` branch | vertex-A, vertex-B, vertex-C, edge-AB, edge-BC, edge-CA, interior |

Each row below is exercised with **many pseudo-random inputs from a fixed seed**
(`SplitMix64`, see `tests/common/mod.rs`), and both `.so`s' outputs are compared as
**raw bytes**, so `-0.0` and differing NaN payloads count as divergences.

## Status

All 79 rows pass, in both the debug and the release profile.

| rows | test file | tests |
|---|---|---|
| 1–20  | `tests/phase_b_primitives.rs` | 16 |
| 21–29 | `tests/phase_b_shapes.rs`     | 9  |
| 30–38 | `tests/phase_b_simplex.rs`    | 7  |
| 39–56 | `tests/phase_b_gjk.rs`        | 14 |
| 57–72 | `tests/phase_b_manifolds.rs`  | 8  |
| 73–79 | `tests/phase_b_api.rs`        | 3  |

Rows that only *reach* their target through a `switch`/`if` chain assert their own
branch coverage rather than assuming it — e.g. `c22` reports
`A=24000 B=8000 edge=8000`, `c23`'s seven Voronoi regions are each targeted by a
hand-built simplex and then verified to have landed in the intended region, and
`c2CapsuletoPolyManifold` is checked to have produced 0-, 1- **and** 2-point manifolds
(`0=116340 1=43505 2=8155`). A row is not ticked on the strength of "the call
returned".

**Rows 63–79 note.** These reach `c2GJK` with `C2_TYPE_POLY`, where the C library
reads an uninitialised `c2Proxy` and is genuinely not a function of its inputs (see
`ERRORS.md` rows 19–20 and `tests/probe_uninit.rs`). The tests call
`common::zero_stack()` immediately before *both* the C and the Rust invocation, which
forces that region to all-zero — the exact state `src/gjk.rs` models — and makes those
paths comparable byte-for-byte. Without it the C side either returns a stack address
or SIGSEGVs. This is also the only route to the five `static` helpers `c2Clip`,
`c2SidePlanes`, `c2SidePlanesFromPoly`, `c2KeepDeep` and `c2Incident`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random 32-bit patterns for `x`,`y` (all float classes incl. sNaN) | [x] |
| 2 | `c2Neg`, `c2CCW90`, `c2Skew`, `c2Absv` | random 32-bit patterns (signed zeros, NaN sign/payload preservation) | [x] |
| 3 | `c2Sub`, `c2Add` | random normal pairs | [x] |
| 4 | `c2Sub`, `c2Add` | random 32-bit patterns (inf−inf, NaN+NaN destination-operand selection) | [x] |
| 5 | `c2Mulvs` | random normal `a`, scalar in {normal, ±0, ±inf, NaN, denormal} | [x] |
| 6 | `c2Mulvs` | random 32-bit patterns for both (`0*inf`, NaN×NaN) | [x] |
| 7 | `c2Div` | random `a`; `b` in {normal, `+0`, `-0`, `±inf`, NaN, denormal, FLT_MAX} | [x] |
| 8 | `c2Dot` | random normal pairs; then random 32-bit patterns (`inf*0`, two NaNs into one `addss`) | [x] |
| 9 | `c2Det2` | random normal pairs; then random 32-bit patterns | [x] |
| 10 | `c2Len` | random normals; zero vector; `±inf`; NaN; FLT_MAX (overflow to `inf` before `sqrtf`); denormals | [x] |
| 11 | `c2Norm` | random normals; **zero vector** (`0/0` ⇒ NaN); `±inf`; NaN; denormal (`1/tiny` ⇒ `inf`) | [x] |
| 12 | `c2Maxv`, `c2Minv` | random normal pairs; equal components; `±0` pairs; one NaN; both NaN (second-operand selection) | [x] |
| 13 | `c2Clampv` | `lo <= hi` random; `lo > hi` inverted; `a` inside / below / above; NaN in each of `a`,`lo`,`hi` | [x] |
| 14 | `c2Dist` | random `c2h` × `c2v`; `h.n` zero; `h.d` `±inf`; NaN in each field | [x] |
| 15 | `c2Intersect` | random `a`,`b`,`da`,`db`; `da == db` (`x/0`); `da == db == 0` (`0/0`); `da`/`db` `±inf`; NaN | [x] |
| 16 | `c2RotIdentity`, `c2xIdentity` | no inputs — exact bytes of the returned structs | [x] |
| 17 | `c2Mulrv`, `c2MulrvT` | identity rotation; random unit rotations; non-unit `(c,s)`; `(0,0)`; `±inf`/NaN in `c`,`s`,`b` | [x] |
| 18 | `c2Mulxv`, `c2MulxvT` | identity `c2x`; translation-only; rotation-only; both; NaN/inf in `p` and `r` | [x] |
| 19 | `c2BBVerts` | well-formed AABB; inverted (`min > max`); zero-extent (`min == max`); `±inf` corners; NaN corners | [x] |
| 20 | `c2PlaneAt` | random `c2Poly`, `i ∈ 0..7` (all in-range indices) | [x] |
| 21 | `c2Support` | `count ∈ {1,2,3,4,8}`, random verts, random direction `d` | [x] |
| 22 | `c2Support` | `count ∈ {1,2,4,8}`, direction with ties (equal dots ⇒ first index wins, `>` not `>=`) | [x] |
| 23 | `c2Support` | `count ∈ {1..8}`, verts containing NaN (`dot > dmax` false ⇒ index sticks) | [x] |
| 24 | `c2Norms` | `count ∈ {3,4,5,6,7,8}` convex random polygon | [x] |
| 25 | `c2Norms` | `count == 2` (degenerate: wraps, second normal is the negation) | [x] |
| 26 | `c2Norms` | `count ∈ {3..8}` with a duplicated vertex ⇒ one `(NaN,NaN)` normal | [x] |
| 27 | `c2MakeProxy` | `type == C2_TYPE_CIRCLE`, random circle; caller's proxy pre-poisoned | [x] |
| 28 | `c2MakeProxy` | `type == C2_TYPE_AABB`, random / inverted / zero-extent AABB | [x] |
| 29 | `c2MakeProxy` | `type == C2_TYPE_CAPSULE`, random / degenerate (`a==b`) capsule | [x] |
| 30 | `c2GJKSimplexMetric` | `count ∈ {0,1,2,3}` × random simplex vertex positions | [x] |
| 31 | `c22` | random 2-vertex simplices spanning all three branches (vertex-A, vertex-B, edge) | [x] |
| 32 | `c22` | `a == b` (`u == v == 0` ⇒ vertex-A branch); `a`/`b` containing `±inf`/NaN | [x] |
| 33 | `c23` | random 3-vertex simplices; seeds chosen to hit each of the 7 branches | [x] |
| 34 | `c23` | degenerate/collinear triangles (`area == 0`), duplicated vertices, NaN vertices (interior fallthrough) | [x] |
| 35 | `c2D` | `count ∈ {1,2,3}` random simplices; `count == 2` with `det > 0` and `det <= 0`; `det == NaN` | [x] |
| 36 | `c2Witness` | `count ∈ {1,2,3}` × random `sA`,`sB`,`u`,`div` | [x] |
| 37 | `c2Witness` | `div == 0` (`den == inf`), `div` denormal, `div` NaN, `u` values `±inf`/NaN | [x] |
| 38 | `c2L` | `count ∈ {1,2}` random; `count == 3` (the `default` `(0,0)`); `div == 0`; NaN `u` | [x] |
| 39 | `c2GJK` | CIRCLE vs CIRCLE, both transforms `NULL`, `use_radius = 0`, no cache, random separated / touching / overlapping | [x] |
| 40 | `c2GJK` | CIRCLE vs CIRCLE, `use_radius = 1` (radius-shrink path incl. the `dist <= rA+rB` fallback) | [x] |
| 41 | `c2GJK` | CIRCLE vs AABB, both `use_radius` values, transforms `NULL` | [x] |
| 42 | `c2GJK` | CIRCLE vs CAPSULE, both `use_radius` values, transforms `NULL` | [x] |
| 43 | `c2GJK` | AABB vs AABB, both `use_radius` values, transforms `NULL` | [x] |
| 44 | `c2GJK` | AABB vs CAPSULE, both `use_radius` values, transforms `NULL` | [x] |
| 45 | `c2GJK` | CAPSULE vs CAPSULE, both `use_radius` values, transforms `NULL` (parallel, crossing, collinear, degenerate) | [x] |
| 46 | `c2GJK` | every ordered type pair from {CAPSULE, CIRCLE, AABB} (9), `ax_ptr` non-`NULL` random rotation+translation, `bx_ptr` `NULL` | [x] |
| 47 | `c2GJK` | every ordered type pair (9), `ax_ptr` `NULL`, `bx_ptr` non-`NULL` random transform | [x] |
| 48 | `c2GJK` | every ordered type pair (9), **both** transforms non-`NULL` random, `use_radius ∈ {0,1}` | [x] |
| 49 | `c2GJK` | non-unit / zero rotation `(c,s) = (0,0)` and `(2,3)` in the transform | [x] |
| 50 | `c2GJK` | `cache` non-`NULL`, `count = 0` on entry (cold cache) — checks the 36-byte write-back | [x] |
| 51 | `c2GJK` | `cache` warm: call twice with the same cache (round-trip), asserting both the result and the whole cache | [x] |
| 52 | `c2GJK` | `cache` warm then shapes moved (transform changed between the two calls) | [x] |
| 53 | `c2GJK` | `cache` hand-built with `count ∈ {1,2,3}` and in-range `iA`/`iB`, random `metric`/`div` | [x] |
| 54 | `c2GJK` | `outA = NULL`; `outB = NULL`; `iterations = NULL` — in every combination (2³) | [x] |
| 55 | `c2GJK` | inputs at `±inf` / NaN coordinates (iteration cap and the `d1 > d0` break) | [x] |
| 56 | `c2GJK` | coincident shapes (deep overlap ⇒ `s.count == 3` ⇒ `hit`), `use_radius ∈ {0,1}` | [x] |
| 57 | `c2CircletoCircleManifold` | separated, exactly touching, shallow overlap, deep overlap, concentric (`l == 0`), zero radius, negative radius | [x] |
| 58 | `c2CircletoAABBManifold` | circle outside, touching, overlapping an edge, overlapping a corner, centre **inside** (`d2 == 0` deep branch) | [x] |
| 59 | `c2CircletoAABBManifold` | centre inside with `x_overlap == y_overlap` (tie ⇒ y axis); centre exactly on an edge / corner; inverted AABB; zero-extent AABB | [x] |
| 60 | `c2CircletoCapsuleManifold` | separated, touching, overlapping, circle centre on the capsule axis (`d == 0`), degenerate capsule (`a == b`), zero/negative radii | [x] |
| 61 | `c2AABBtoAABBManifold` | separated on x, separated on y, overlapping (x-minor), overlapping (y-minor), `dx == dy` tie, identical boxes, one inside the other, inverted, zero-extent | [x] |
| 62 | `c2CapsuletoCapsuleManifold` | separated, touching, crossing, parallel overlapping, collinear, both degenerate, one degenerate | [x] |
| 63 | `c2CapsuletoPolyManifold` | random convex `B` with `count ∈ {3,4,5,6,7,8}`, `bx_ptr = NULL`, capsule far from the origin (no-contact branch) | [x] |
| 64 | `c2CapsuletoPolyManifold` | ditto, capsule in the shallow band `1e-6 <= d < A.r` | [x] |
| 65 | `c2CapsuletoPolyManifold` | ditto, `d < 1e-6` ⇒ face branch `code == 0` | [x] |
| 66 | `c2CapsuletoPolyManifold` | `d < 1e-6` with the capsule axis in a vertex region ⇒ `code == 1` | [x] |
| 67 | `c2CapsuletoPolyManifold` | `d < 1e-6` from the other side ⇒ `code == 2` | [x] |
| 68 | `c2CapsuletoPolyManifold` | `bx_ptr` non-`NULL`: rotation-only, translation-only, both; `count ∈ {3,4,8}` | [x] |
| 69 | `c2CapsuletoPolyManifold` | poly built from a clockwise (inward-normal) vertex ring | [x] |
| 70 | `c2CapsuletoPolyManifold` | `m` pre-poisoned, exercising the untouched-field / early-return rows | [x] |
| 71 | `c2AABBtoCapsuleManifold` | random AABB × capsule: separated, touching, penetrating, capsule fully inside | [x] |
| 72 | `c2AABBtoCapsuleManifold` | capsule axis parallel to an AABB edge; axis through a corner; degenerate capsule; `m` pre-poisoned (trailing `c2Neg(m->n)` on the bail-out path) | [x] |
| 73 | `c2Collide` | all 9 valid ordered type pairs from {CAPSULE, CIRCLE, AABB}, random shapes, `m` pre-poisoned | [x] |
| 74 | `c2Collide` | the same 9 pairs on a coarse half-integer lattice so exact touching / shared-edge cases occur often | [x] |
| 75 | `omni_manifold` | all 16 `(type_a, type_b)` pairs including `POLY`, random float quintuples | [x] |
| 76 | `omni_manifold` | all 16 pairs on a small half-integer lattice (exact touches, shared edges, coincident shapes) | [x] |
| 77 | `omni_manifold` | all 16 pairs with the 5 floats drawn from the degenerate pool (`±0`, `±inf`, NaN, denormal, `FLT_MAX`, negative radii) | [x] |
| 78 | `omni_manifold` | `m` pre-poisoned with a distinctive bit pattern, all 16 pairs (verifies which fields are left untouched) | [x] |
| 79 | `ptr_from_parts` | `typ ∈ {CAPSULE, CIRCLE, AABB}` — the returned heap struct's bytes | [x] |
