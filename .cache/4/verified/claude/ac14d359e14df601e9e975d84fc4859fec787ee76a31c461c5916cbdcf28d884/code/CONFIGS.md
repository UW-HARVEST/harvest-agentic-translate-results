# CONFIGS.md — Configuration surface table (Phase A → gate for Phase B)

## Build-time configuration axes

| axis | values | source |
|------|--------|--------|
| Cargo features | **none declared** — `Cargo.toml` has no `[features]` table | `Cargo.toml` |
| Cargo feature combinations to verify | exactly one: `--no-default-features` (≡ default) | derived |
| CMake options | none (`c_src/CMakeLists.txt` has no `option()`, no `#ifdef`/`#if` anywhere in `c_src/`) | `c_src/CMakeLists.txt`, `grep -rn '#if' c_src/` |
| Translation units | one: `c_src/src/lib.c` | `c_src/CMakeLists.txt` |

So there is a **single** build configuration; all rows below are verified under it
(and re-run under `--no-default-features` explicitly in Phase D).

## Runtime configuration axes (what the C actually branches on)

1. **Shape-type tag** `C2_TYPE typeA` × `typeB` — the only "option/mode" the public
   API exposes. Valid values `{C2_TYPE_CIRCLE=0, C2_TYPE_AABB=1}` → 4 valid
   dispatch pairs, each reaching a different collision routine, and the
   `AABB×CIRCLE` pair swaps its arguments (`c2CircletoAABB(*B, *A)`).
2. **Entry-point level** — the low-level exports (`c2V`, `c2Maxv`, `c2Minv`,
   `c2Clampv`, `c2Sub`, `c2Dot`) and mid-level exports (`c2CircletoCircle`,
   `c2CircletoAABB`, `c2AABBtoAABB`) are all public in the `.so` and are driven
   directly, not only through the `collided` convenience dispatcher.
3. **Float value shape** — the branches are all comparisons (`>`, `<`), so the
   distinguished input shapes are: strictly-greater / strictly-less / exactly
   equal operands, `±0.0` pairs, NaN in either operand position, `±Inf`,
   subnormals, magnitudes that overflow `f32` on multiply/add, and exact
   tie values on the collision thresholds (`d2 == r2`).
4. **Geometric shape** — circle radius sign/zero; AABB well-formed vs degenerate
   (`min == max`) vs inverted (`min > max`); point inside / outside / on each of
   the 4 edges / past each of the 4 corners of the box; separation along x only,
   y only, both, neither.
5. **Pointer shape** for `collided` — properly aligned vs deliberately
   byte-misaligned buffers, plus aliasing `A == B`.

Every row is driven with **many randomized inputs** from a fixed-seed PCG
generator (see `tests/differential.rs`, `Rng::new(seed)`), plus the hand-picked
boundary values named in the row.

## Rows

All 17 rows are checked off: each has a passing differential test in
`tests/valid_paths.rs` (named in the last column).

| # | entry point(s) | configuration (options set + input shape) | done / test |
|---|----------------|------------------------------------------|-------------|
| C1 | `c2V` | random `f32` bit patterns (all classes: normal, subnormal, ±0, ±Inf, NaN payloads) round-tripped through the struct return | [x] `cfg_c1_c2v` |
| C2 | `c2Maxv` | random pairs; plus `a>b`, `a<b`, `a==b`, `+0.0/-0.0`, NaN in `a`, NaN in `b`, NaN in both, ±Inf — per-component independently | [x] `cfg_c2_c2maxv` |
| C3 | `c2Minv` | same shape set as C2 | [x] `cfg_c3_c2minv` |
| C4 | `c2Clampv` | `a` below `lo`, inside, above `hi`, `lo == hi`, **inverted** `lo > hi`, NaN in `a`/`lo`/`hi`, ±Inf bounds; random triples | [x] `cfg_c4_c2clampv` |
| C5 | `c2Sub` | random pairs; `Inf - Inf`, `x - x`, `-0.0 - 0.0`, subnormal cancellation, overflow to ±Inf, NaN | [x] `cfg_c5_c2sub` |
| C6 | `c2Dot` | random pairs; products that overflow to `+Inf`/`-Inf`, `Inf*0` → NaN, subnormal underflow, exact cancellation `x*y + (-x)*y == 0` | [x] `cfg_c6_c2dot` |
| C7 | `c2CircletoCircle` | random circles in a small coordinate box so hits and misses both occur; plus exact tangency (`d == rA+rB`, i.e. `d2 == r2` → C returns 0), concentric, one circle inside the other, `r == 0`, **negative** `r`, huge `r` (overflow in `r2`), NaN/Inf coords | [x] `cfg_c7_circle_circle` |
| C8 | `c2CircletoAABB` | random circle × random AABB (well-formed); circle centre inside the box, outside each of the 4 edges, past each of the 4 corners, exactly on an edge/corner (`d2 == r2` tie), `r == 0`, negative `r`, degenerate box (`min == max`), **inverted** box (`min > max`), NaN/Inf | [x] `cfg_c8_circle_aabb` |
| C9 | `c2AABBtoAABB` | random pairs; overlapping, edge-touching (`A.max.x == B.min.x` → C returns 1 since `<` is false), separated in x only, y only, both, one box contained in the other, degenerate (`min == max`), inverted (`min > max`), NaN/Inf | [x] `cfg_c9_aabb_aabb` |
| C10 | `collided` | `typeA=CIRCLE, typeB=CIRCLE` — random circle pairs + all C7 boundary shapes, passed through the void-pointer dispatcher | [x] `cfg_c10_collided_circle_circle` |
| C11 | `collided` | `typeA=CIRCLE, typeB=AABB` — random circle/AABB + all C8 boundary shapes | [x] `cfg_c11_collided_circle_aabb` |
| C12 | `collided` | `typeA=AABB, typeB=CIRCLE` — the **argument-swapping** arm (`c2CircletoAABB(*B, *A)`); same data as C11 with roles exchanged, to catch a swapped-argument translation bug | [x] `cfg_c12_collided_aabb_circle` |
| C13 | `collided` | `typeA=AABB, typeB=AABB` — random AABB pairs + all C9 boundary shapes | [x] `cfg_c13_collided_aabb_aabb` |
| C14 | `collided` | all 4 valid tag pairs with **byte-misaligned** `A`/`B` buffers (offset 1..=7 inside an over-allocated block) | [x] `cfg_c14_collided_unaligned_pointers` |
| C15 | `collided` | all 4 valid tag pairs with `A == B` (same pointer aliased for both operands, incl. CIRCLE×AABB reinterpreting the same bytes as two different structs) | [x] `cfg_c15_collided_aliased_pointers` |
| C16 | `c2V` → `c2Sub` → `c2Dot` → `c2CircletoCircle` composed pipeline | drive the low-level exports in sequence and feed their results into the mid-level ones, reproducing the internal call chain through the FFI boundary (catches bugs only visible in composition) | [x] `cfg_c16_composed_pipeline` |
| C17 | `c2Clampv` → `c2Sub` → `c2Dot` composed pipeline | the exact internal chain of `c2CircletoAABB`, driven externally and cross-checked against `c2CircletoAABB` itself in both libraries | [x] `cfg_c17_composed_clamp_pipeline` |
