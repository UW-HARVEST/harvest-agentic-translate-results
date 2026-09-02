# CONFIGS.md — Phase A configuration surface table (valid inputs)

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

**A1 — entry-point level (call hierarchy from `c_src/src/lib.c`).** The only
header-declared function is `gjk`, but all 31 symbols are public, so the low
level entry points are part of the surface and are driven directly:

| level | functions |
|---|---|
| L0 pure math | `c2V` `c2Mulvs` `c2Maxv` `c2Minv` `c2Clampv` `c2Sub` `c2Add` `c2Dot` `c2Det2` `c2Len` `c2Neg` `c2Skew` `c2CCW90` `c2Div` `c2Norm` `c2Mulrv` `c2MulrvT` `c2Mulxv` `c2RotIdentity` `c2xIdentity` |
| L1 shape/proxy | `c2BBVerts` `c2MakeProxy` `c2Support` |
| L2 simplex | `c2GJKSimplexMetric` `c22` `c23` `c2D` `c2L` `c2Witness` |
| L3 solver | `c2GJK` |
| L4 wrapper | `gjk` |

**A2 — `C2_TYPE typeA` x `typeB`** (`switch` in `c2MakeProxy`): `CIRCLE` (1 vert,
radius `r`), `AABB` (4 verts, radius 0), `CAPSULE` (2 verts, radius `r`) -> 3x3 = 9
proxy pairings, each giving a different `count`/`radius` and hence a different
`c2Support` loop length and a different `use_radius` shrink.

**A3 — transforms `ax_ptr` / `bx_ptr`** (`if (!ax_ptr)` + `c2Mulxv`/`c2MulrvT`):
`NULL` (identity substituted), explicit identity, pure translation, pure
rotation, translation+rotation, non-unit rotation.

**A4 — `use_radius`** (`else if (use_radius)`): `0` (raw simplex distance) vs
non-zero (radius shrink, with the `dist > rA+rB && dist > FLT_EPSILON`
sub-branch).

**A5 — `cache`** (`if (cache)`, `!!cache->count`, `cache_was_read`): `NULL`;
zero-count (cold); warm cache round-tripped from a previous `c2GJK` call;
hand-built cache with count 1/2/3. Warm caches take a completely different
simplex-seeding path.

**A6 — output pointers**: `outA`/`outB`/`iterations` each present or `NULL` (the
`if (outA)` guards).

**A7 — input shape / geometry**: separated far, separated near, exactly touching,
overlapping, fully contained, coincident; zero-extent AABB, inverted AABB,
zero-length capsule, zero radius, large radius; magnitudes spanning denormal,
~1, and ~1e18; negative and mixed-sign coordinates.

**A8 — simplex `count`** for the L2 entry points: 1, 2, 3 (and the `default`
arms, which live in `ERRORS.md`).

**A9 — `gjk` `reverse`** (`if (reverse)`): zero vs non-zero `char`.

No `#ifdef` / conditional compilation exists in `c_src/src/lib.c`, and
`translation/Cargo.toml` declares no `[features]`, so there is a single build
configuration.

## Table

Every row is exercised with many randomized inputs (fixed seed, see
`tests/common/mod.rs::Rng`), not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V`, `c2Neg`, `c2Skew`, `c2CCW90` | random f32 pairs incl. `±0`, denormals, `1e-30`..`1e30`, negatives | [x] |
| 2 | `c2Add`, `c2Sub` | random pairs; also near-cancelling operands (`a - a`), mixed magnitudes | [x] |
| 3 | `c2Mulvs`, `c2Div` | random vector x random scalar; scalar `±1`, `±0.5`, huge, denormal | [x] |
| 4 | `c2Dot`, `c2Det2` | random pairs; orthogonal, parallel, and catastrophic-cancellation cases | [x] |
| 5 | `c2Len`, `c2Norm` | random vectors; unit, tiny (denormal-length), huge, axis-aligned | [x] |
| 6 | `c2Maxv`, `c2Minv` | random pairs; equal components, `-0.0` vs `+0.0` ordering | [x] |
| 7 | `c2Clampv` | `a` inside / below / above `[lo,hi]`, per-component mixes, `lo == hi` | [x] |
| 8 | `c2RotIdentity`, `c2xIdentity` | no inputs — bit-exact constant return (verifies the 8/16-byte struct ABI) | [x] |
| 9 | `c2Mulrv`, `c2MulrvT` | random `c2r` x random `c2v`; unit rotations at many angles, `c=s=0`, non-unit | [x] |
| 10 | `c2Mulxv` | random `c2x` (translation+rotation) x random `c2v`; identity, pure-translation, pure-rotation sub-cases | [x] |
| 11 | `c2BBVerts` | random AABBs: normal, zero-extent, single-axis-degenerate, large, negative-coordinate | [x] |
| 12 | `c2MakeProxy` | `type = CIRCLE`, random `c2Circle` -> `count = 1`, `radius = r` | [x] |
| 13 | `c2MakeProxy` | `type = AABB`, random `c2AABB` -> `count = 4`, `radius = 0`, 4 corners | [x] |
| 14 | `c2MakeProxy` | `type = CAPSULE`, random `c2Capsule` -> `count = 2`, `radius = r` | [x] |
| 15 | `c2MakeProxy` | pre-poisoned destination proxy, each valid type — asserts the untouched `verts[count..8]` tail bytes match too | [x] |
| 16 | `c2Support` | `count = 1` (circle proxy shape) x random directions | [x] |
| 17 | `c2Support` | `count = 2` (capsule shape) x random directions incl. perpendicular ties | [x] |
| 18 | `c2Support` | `count = 4` (AABB shape) x random directions incl. exact diagonal ties | [x] |
| 19 | `c2Support` | `count = 8` (full `verts[8]`) x random directions, duplicate verts present | [x] |
| 20 | `c2GJKSimplexMetric` | `count = 2` — returns `c2Len` of the edge; random simplices | [x] |
| 21 | `c2GJKSimplexMetric` | `count = 3` — returns `c2Det2` of the two edges; random simplices | [x] |
| 22 | `c22` | random 2-vertex simplices covering all three branches (`v<=0`, `u<=0`, interior); full struct compared, incl. `iA`/`iB` and the untouched `c`/`d` slots | [x] |
| 23 | `c23` | random 3-vertex simplices covering all seven branches; full 136-byte struct compared | [x] |
| 24 | `c23` | 3-vertex simplices seeded from *real* GJK runs (origin inside/outside the triangle) rather than pure noise | [x] |
| 25 | `c2D` | `count = 1` and `count = 2` (both the `c2Det2 > 0` -> `c2Skew` and the `c2CCW90` sub-branches) | [x] |
| 26 | `c2L` | `count = 1`; `count = 2` with random `u`/`div` weights | [x] |
| 27 | `c2Witness` | `count = 1`, `2`, `3` with random `sA`/`sB`/`u`/`div`; both out-pointers compared | [x] |
| 28 | `c2GJK` | all 9 `(typeA, typeB)` pairings, `ax_ptr = bx_ptr = NULL`, `use_radius = 1`, cache `NULL`, all outputs requested, random separated shapes | [x] |
| 29 | `c2GJK` | all 9 pairings, `use_radius = 0`, transforms `NULL`, random separated shapes | [x] |
| 30 | `c2GJK` | all 9 pairings, random **overlapping** shapes (drives the `hit` / `count == 3` path), `use_radius = 1` | [x] |
| 31 | `c2GJK` | all 9 pairings, random overlapping shapes, `use_radius = 0` | [x] |
| 32 | `c2GJK` | all 9 pairings, shapes placed to be near-touching within `FLT_EPSILON` (drives the `dist > rA+rB` boundary both ways) | [x] |
| 33 | `c2GJK` | explicit identity `c2x` passed for both (must equal the `NULL` result) | [x] |
| 34 | `c2GJK` | pure-translation `ax`, `bx_ptr = NULL`; and the mirror case | [x] |
| 35 | `c2GJK` | pure-rotation `ax`/`bx` at random angles (exercises `c2MulrvT` in the support call) | [x] |
| 36 | `c2GJK` | translation+rotation on **both** shapes, random, all 9 pairings | [x] |
| 37 | `c2GJK` | non-unit / zero rotation matrices on both shapes | [x] |
| 38 | `c2GJK` | `cache != NULL` with `count = 0` (cold) — asserts the **written-back** cache (`metric`, `count`, `iA`, `iB`, `div`) matches bit-for-bit | [x] |
| 39 | `c2GJK` | warm-cache sequence: call 1 cold, then call 2 reusing the cache with the *same* shapes | [x] |
| 40 | `c2GJK` | warm-cache sequence with *moved* shapes between calls (the realistic reuse pattern) | [x] |
| 41 | `c2GJK` | long warm-cache chain: 8 successive calls along a random motion path, cache carried forward, every intermediate result and cache state compared | [x] |
| 42 | `c2GJK` | hand-built caches with `count = 1`, `2`, `3` and in-range `iA`/`iB` indices, random `metric`/`div` | [x] |
| 43 | `c2GJK` | output-pointer combinations: `(outA,outB)`, `(outA,NULL)`, `(NULL,outB)`, `(NULL,NULL)`, each x `iterations` present/`NULL` | [x] |
| 44 | `c2GJK` | zero-extent AABB, zero-length capsule, zero-radius circle/capsule (duplicate-support / `dup` break path) | [x] |
| 45 | `c2GJK` | coincident shapes (A and B identical) — degenerate simplex, `div == 0` path | [x] |
| 46 | `c2GJK` | one shape fully containing the other, all 9 pairings | [x] |
| 47 | `c2GJK` | coordinate magnitudes ~`1e18` and ~`1e-30` (denormal) for both shapes | [x] |
| 48 | `c2GJK` | large radii relative to shape extent (radius dominates the shrink) | [x] |
| 49 | `gjk` | `reverse = 0`, random AABB+capsule, both out-pointers | [x] |
| 50 | `gjk` | `reverse` non-zero (`1`, `2`, `-1`, `0x7f`), random AABB+capsule | [x] |
| 51 | `gjk` | random AABB+capsule spanning separated / touching / overlapping / contained, both `reverse` values, out-pointers present | [x] |
| 52 | `gjk` | degenerate AABB (zero extent, inverted) and degenerate capsule (zero length, zero radius) x both `reverse` values | [x] |

## Result

All 52 rows pass. Each row is a test in `tests/phase_b_lowlevel.rs` (rows 1-27,
the L0/L1/L2 entry points) or `tests/phase_b_gjk.rs` (rows 28-52, `c2GJK` and
`gjk`), named `rowNN_*`, and each drives 1200-4000 randomized inputs from a
fixed seed rather than a single hand-picked value. Comparisons are bit-exact and
cover the **whole** observable surface, not just the return value: the mutated
`c2Simplex` (all four slots, including the ones the C leaves untouched), the
`c2Proxy` tail bytes beyond `count`, the written-back `c2GJKCache`, `*outA`,
`*outB` and `*iterations`.

Branch coverage is asserted rather than assumed — `row22`, `row23` and `row25`
fail if `c22`/`c23`/`c2D` never produced each of their possible outcomes across
the randomized inputs.

`tests/fuzz_differential.rs` adds a wider net on top: 24 independent seeds x 2500
configurations in which the axes are randomized *together* (type pair, magnitude
regime from denormal to ~1e18, separation regime, degenerate/inverted/negative-
radius shape variants, transform regime, `use_radius`, output-pointer mask, cache
mode including warm reuse), plus 200 000 randomized low-level calls. That is
roughly 60 000 additional `c2GJK` configurations beyond the named rows.

Nothing needed fixing for the valid-path rows themselves; the one real
divergence found during verification was on the NaN paths and is written up in
`ERRORS.md`.
