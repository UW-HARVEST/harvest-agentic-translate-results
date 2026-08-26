# CONFIGS.md — Phase A configuration surface (valid inputs)

Axes derived mechanically from the branches `c_src/src/lib.c` actually takes.

## Build-time configuration axes

`Cargo.toml` has **no `[features]` section** (`cargo metadata` reports
`"features": {}`), and `c_src/CMakeLists.txt` has no `option()`, no
`target_compile_definitions` and no `#ifdef` anywhere in `lib.c`. Therefore there
is exactly **one** build configuration:

| # | configuration | command |
|---|---------------|---------|
| B1 | default (= no features; the only valid combination) | `cargo test --no-default-features` |

## Runtime configuration axes (grepped from the source)

| axis | values the C distinguishes | branch site |
|------|----------------------------|-------------|
| `typeA`, `typeB` | `C2_TYPE_CIRCLE` (1 vert, r), `C2_TYPE_AABB` (4 verts, r=0), `C2_TYPE_CAPSULE` (2 verts, r) | `c2MakeProxy` `switch` |
| `ax_ptr`, `bx_ptr` | `NULL` → `c2xIdentity()`; non-`NULL` → arbitrary `c2x` (rotation `c2r{c,s}` + translation) | `if (!ax_ptr)` / `if (!bx_ptr)` |
| `use_radius` | `0` (ignore radii) / non-zero (shrink-or-midpoint block) | `else if (use_radius)` |
| `cache` | `NULL`; `count==0` (cold); warm (`count` 1/2/3 written back by a prior call) | `if (cache)`, `!!cache->count` |
| out-params | each of `outA`, `outB`, `iterations` present / `NULL` | three `if (...)` guards |
| geometry relation | separated, touching, overlapping-but-not-containing, containing (`hit`), identical | `s.count==3`, `dist > rA+rB` |
| `s.count` (low-level) | `1`, `2`, `3` (plus out-of-range → `ERRORS.md`) | `switch` in `c22`/`c23`/`c2D`/`c2L`/`c2Witness`/`c2GJKSimplexMetric` |
| `c22` region | `v<=0` (vertex A), `u<=0` (vertex B), else (edge AB) | 3 arms |
| `c23` region | 7 arms: vertex A / vertex B / vertex C / edge AB / edge BC / edge CA / interior | 7 arms |
| `c2D` orientation | `c2Det2(ab, -a) > 0` → `c2Skew`, else `c2CCW90` | `if` inside `case 2` |
| `c2Support` shape | `count` 1 / 2 / 4; ties; direction quadrant | loop + `dot > dmax` |
| scale of coordinates | ~1, ~1e3, ~1e5 (drives the row-33 metric branch), subnormal, `0` | float arithmetic |

## Configuration table

Every row is exercised against **both** `.so`s with many randomised inputs
(fixed seed, `tests/common/mod.rs::Rng`, a xorshift64* PRNG) — not a single
hand-picked value. `[x]` = passing.

Status: **all 56 rows pass**, under both the `dev` and the `release` Rust `.so`
(`./run_all.sh`). Row → test mapping is by name: CONFIGS row *n* is
`rowNN_*` in `tests/vectors.rs` (rows 1–19), `tests/simplex.rs` (20–27),
`tests/gjk.rs` (28–50) and `tests/public.rs` (51–56).

Branch coverage actually achieved by the randomised inputs (printed with
`cargo test -- --nocapture --test-threads=1`), so no row is vacuously "covered":

| branch set | coverage |
|------------|----------|
| `c22` arms (vertex A, vertex B, edge AB) | `[4350, 4590, 7444]` |
| `c23` arms (A, B, C, AB, BC, CA, interior) | `[2894, 746, 367, 6039, 2188, 6285, 14249]` |
| `c2D` arms (`c2Skew` / `c2CCW90`) | `4085 / 4107` |
| `c2GJK` final arms (hit / radius-shrink / midpoint) | `[1597, 11717, 3070]` |
| `c2GJK` iteration counts | `[3119, 5635, 3094, 548, 0…]` (max 3) |

Note on the `use_radius == 0` rows: the midpoint arm is unreachable by
construction there, because the whole `else if (use_radius)` block is skipped —
so row 29 asserts shrink+hit coverage only.

### Pure vector helpers — lowest level entry points

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | 4096 random `(x,y)` incl. `0`, `-0`, subnormal, `±inf`, `NaN`, `±FLT_MAX` | [x] |
| 2 | `c2Mulvs` | random `c2v` × random scalar; scalar `0`, `-0`, `1`, `inf`, `NaN` | [x] |
| 3 | `c2Add`, `c2Sub` | random pairs; cancellation (`a==b`), `inf-inf`, `NaN` operand | [x] |
| 4 | `c2Dot` | random pairs; orthogonal, antiparallel, overflow-to-inf (`1e30`), `NaN` | [x] |
| 5 | `c2Det2` | random pairs; collinear (det==0), overflow, `NaN` | [x] |
| 6 | `c2Len` | random; zero vector, huge (overflow), subnormal, `NaN`, `inf` | [x] |
| 7 | `c2Maxv`, `c2Minv` | random pairs; equal components, `±0` pairs, `NaN` in `a` only, in `b` only, in both | [x] |
| 8 | `c2Clampv` | random `a`/`lo`/`hi` with `lo<hi`, `lo==hi`, and `lo>hi` (inverted) | [x] |
| 9 | `c2Neg`, `c2Skew`, `c2CCW90` | random incl. `±0` (sign of negated zero), `NaN` | [x] |
| 10 | `c2Div` | random `a` × divisor `{random, 1, 0, -0, inf, NaN, FLT_MIN}` | [x] |
| 11 | `c2Norm` | random unit-ish, huge, tiny, zero vector, `NaN`/`inf` components | [x] |
| 12 | `c2RotIdentity`, `c2xIdentity` | no inputs — constant-value parity (bit pattern) | [x] |
| 13 | `c2Mulrv`, `c2MulrvT` | random `c2r` (normalised and un-normalised) × random `c2v`; identity rot; `c=s=0` | [x] |
| 14 | `c2Mulxv` | random `c2x` (rot × translation) × random `c2v`; identity transform | [x] |

### Proxy construction

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 15 | `c2BBVerts` | random AABB: `min<max`, `min==max` (degenerate point), `min>max` (inverted), huge, `NaN` | [x] |
| 16 | `c2MakeProxy` | `C2_TYPE_CIRCLE` × random `c2Circle` (r>0, r==0, r<0, NaN r) — checks `radius`, `count==1`, `verts[0]`, and that `verts[1..8]` are left untouched | [x] |
| 17 | `c2MakeProxy` | `C2_TYPE_AABB` × random `c2AABB` (normal / degenerate / inverted) — `radius==0`, `count==4`, 4 verts | [x] |
| 18 | `c2MakeProxy` | `C2_TYPE_CAPSULE` × random `c2Capsule` (a!=b, a==b, r==0, r<0) — `count==2`, 2 verts | [x] |
| 19 | `c2MakeProxy` | same shape written into a **pre-filled** (0xAA-poisoned) `c2Proxy` for all 3 types — verifies exactly which bytes each arm writes | [x] |

### Simplex primitives (the low-level entry points, driven directly)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 20 | `c2GJKSimplexMetric` | `count==1` (→0); `count==2` random `p`s; `count==3` random `p`s (signed det, both windings, collinear) | [x] |
| 21 | `c22` | `count==2`, random `a.p`/`b.p` biased to hit all 3 arms (`v<=0`, `u<=0`, edge) + `a.p==b.p`, origin on segment, origin at a vertex | [x] |
| 22 | `c23` | `count==3`, random triangles biased to hit all **7** arms (3 vertex, 3 edge, 1 interior) + degenerate/collinear/duplicate-vertex triangles | [x] |
| 23 | `c2D` | `count==1`; `count==2` with `c2Det2(ab,-a) > 0` (skew arm) and `<= 0` (CCW90 arm) incl. `==0` boundary | [x] |
| 24 | `c2L` | `count==1`; `count==2` with random `u`/`div` (incl. `div` from a real `c22` run) | [x] |
| 25 | `c2Witness` | `count==1`; `count==2`; `count==3`; each with random `sA`/`sB`/`u`/`div` | [x] |
| 26 | `c2Support` | `count==1`; `count==2`; `count==4` (AABB verts); random directions in all 4 quadrants + axis-aligned ties (first-index-wins) | [x] |
| 27 | `c2Support` | `count==8` full proxy array, random verts and directions | [x] |

### `c2GJK` — the full pipeline, all option combinations

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 28 | `c2GJK` | shape-type cross product **9 rows in one**: `{circle,aabb,capsule} × {circle,aabb,capsule}`, identity transforms (`NULL`), `use_radius=1`, `cache=NULL`, random geometry | [x] |
| 29 | `c2GJK` | same 9 type combos, `use_radius=0` | [x] |
| 30 | `c2GJK` | `ax_ptr = NULL`, `bx_ptr` = random non-identity `c2x` (rotation + translation) | [x] |
| 31 | `c2GJK` | `ax_ptr` = random non-identity, `bx_ptr = NULL` | [x] |
| 32 | `c2GJK` | both transforms non-`NULL`, random rotations (normalised `c2r`) + translations | [x] |
| 33 | `c2GJK` | both transforms non-`NULL` with **un-normalised** `c2r` (scaling/skewing rotation) | [x] |
| 34 | `c2GJK` | `cache != NULL`, `count = 0` (cold start) — checks the full write-back (`metric`, `count`, `iA[]`, `iB[]`, `div`) | [x] |
| 35 | `c2GJK` | `cache != NULL`, **warm reuse**: call twice with the same cache and same shapes (the `gjk_cache` scenario) — 2nd call must read the cache | [x] |
| 36 | `c2GJK` | `cache != NULL`, warm reuse with **moved** shapes between the two calls (stale cache, still accepted per `ERRORS.md` row 32) | [x] |
| 37 | `c2GJK` | `cache != NULL`, warm cache **carried across a shape-type change** (cache from AABB reused for a circle) — restricted to indices valid for both | [x] |
| 38 | `c2GJK` | chain of 8 successive cached calls, shapes drifting each step (long-lived cache, all `count` transitions 1↔2↔3) | [x] |
| 39 | `c2GJK` | out-param matrix: all 8 combinations of `{outA, outB, iterations}` present/`NULL` | [x] |
| 40 | `c2GJK` | geometry relation: **separated** (`dist > rA+rB`, shrink branch) | [x] |
| 41 | `c2GJK` | geometry relation: **touching** (`dist ≈ rA+rB`, boundary of the shrink branch) | [x] |
| 42 | `c2GJK` | geometry relation: **overlapping** radii but cores disjoint (midpoint branch, `dist=0`) | [x] |
| 43 | `c2GJK` | geometry relation: **cores intersect** → `s.count==3` → `hit=1` path | [x] |
| 44 | `c2GJK` | geometry relation: **identical shapes** (A == B, `dist=0`, degenerate directions) | [x] |
| 45 | `c2GJK` | degenerate shapes: zero-radius circle, zero-area AABB (`min==max`), zero-length capsule (`a==b`) — all pairings | [x] |
| 46 | `c2GJK` | inverted AABB (`min > max`) — no validation in C | [x] |
| 47 | `c2GJK` | negative radii on circle and capsule (shrink branch grows the distance) | [x] |
| 48 | `c2GJK` | coordinate scale sweep: `~1e-6`, `~1`, `~1e3`, `~1e5`, `~1e7` (drives `iter` counts and the row-33 metric branch) | [x] |
| 49 | `c2GJK` | `use_radius` truthiness: `0`, `1`, `2`, `-1`, `INT_MIN` | [x] |
| 50 | `c2GJK` | large random sweep (16 384 cases) over *all* axes jointly: random types, random transforms, random `use_radius`, random cache state, random geometry & scale | [x] |

### `gjk_cache` — the declared public entry point (`include/lib.h`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 51 | `gjk_cache` | `reverse == 0` (AABB-vs-capsule ordering), random `a1..a4` box + `b1..b5` capsule | [x] |
| 52 | `gjk_cache` | `reverse != 0` (capsule-vs-AABB ordering), same random inputs | [x] |
| 53 | `gjk_cache` | `a9`/`b9` pointing at poisoned buffers — assert the buffers are **unchanged** (C never writes them) and identical between C and Rust | [x] |
| 54 | `gjk_cache` | `a9`/`b9` `NULL` (never dereferenced) | [x] |
| 55 | `gjk_cache` | degenerate/extreme args: inverted box, zero-size box, `a==b` capsule, `r=0`, `r<0`, `±inf`, `NaN`, `±FLT_MAX`, subnormals | [x] |
| 56 | `gjk_cache` | randomised sweep (4096 cases) over `reverse × a1..a4 × b1..b5` | [x] |
