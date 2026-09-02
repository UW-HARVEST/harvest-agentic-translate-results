# CONFIGS.md — configuration surface table (valid inputs)

Axes derived mechanically from `c_src/src/lib.c` — every `switch`, every `if` on
a flag/pointer argument, and every input shape the code special-cases. There are
no `#ifdef`s and no `[features]` in `translation/Cargo.toml`, so the compile-time
axis is a single default configuration.

## Runtime option axes

| axis | values the C branches on | branch site |
|------|--------------------------|-------------|
| `typeA` | `CAPSULE(0)`, `CIRCLE(1)`, `AABB(2)` | `c2MakeProxy` switch, `c2Collided` outer switch |
| `typeB` | `CAPSULE(0)`, `CIRCLE(1)`, `AABB(2)` | `c2MakeProxy` switch, `c2Collided` inner switches |
| `use_radius` | `0`, `1` (and other truthy ints) | `else if (use_radius)` in `c2GJK` |
| `ax_ptr` | `NULL` (→ identity), non-NULL | `if (!ax_ptr)` |
| `bx_ptr` | `NULL` (→ identity), non-NULL | `if (!bx_ptr)` |
| transform content | identity, pure translation, pure rotation, rotation+translation, non-unit `c2r` | `c2Mulxv` / `c2MulrvT` |
| `outA` / `outB` | `NULL`, non-NULL | `if (outA)` / `if (outB)` |
| `iterations` | `NULL`, non-NULL | `if (iterations)` |
| `cache` | `NULL`, cold (`count==0`), warm (reused across calls), hand-crafted | `if (cache)`, `cache_was_good`, `cache_was_read` |
| simplex `count` | 1, 2, 3 (and out-of-range for `default:` arms) | `c22`, `c23`, `c2D`, `c2L`, `c2Witness`, `c2GJKSimplexMetric` |

## Input-shape axes

| axis | shapes the C distinguishes |
|------|----------------------------|
| proxy vertex count | 1 (circle), 2 (capsule), 4 (AABB) |
| radius | `0`, small, large, negative |
| separation | deeply overlapping, touching exactly, barely separated, far apart |
| AABB | normal, zero-area (`min == max`), inverted (`min > max`), huge |
| capsule | normal segment, degenerate (`a == b`), axis-aligned, very long |
| float class | normal, `±0.0`, subnormal, `±inf`, `NaN`, `FLT_MAX`, `FLT_MIN` |
| magnitude | ~1, ~1e-30 (cancellation), ~1e30 (overflow in `c2Dot`) |
| `c2Support` count | 0, 1, 2, 4, 8 (full `verts[8]`), negative |

## Rows

Each row is one combination the C treats differently. Every row is exercised in
`tests/configs.rs` with **many** randomized inputs (deterministic
`SplitMix64`/xorshift PRNG, fixed seeds) plus hand-picked boundary values, and
compared bit-for-bit between the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random `(x,y)` over all float classes | [x] |
| 2 | `c2Mulvs` | random vector × random scalar (incl. `0`, `inf`, `NaN`) | [x] |
| 3 | `c2Maxv`, `c2Minv` | random pairs incl. `NaN` operands (ternary asymmetry) | [x] |
| 4 | `c2Clampv` | `lo<hi`, `lo==hi`, `lo>hi`, `NaN` bounds | [x] |
| 5 | `c2Sub`, `c2Add` | random pairs, `inf-inf`, `±0` signs | [x] |
| 6 | `c2Dot` | random pairs; huge magnitudes (overflow), tiny (cancellation) | [x] |
| 7 | `c2Det2` | random pairs; collinear (result `0`/`-0`) | [x] |
| 8 | `c2Len` | random vector; zero, huge (overflow to `inf`), `NaN` | [x] |
| 9 | `c2Div`, `c2Norm` | random vector ÷ {random, `0`, `inf`, `NaN`}; zero/inf vector | [x] |
| 10 | `c2Neg`, `c2Skew`, `c2CCW90` | random vectors incl. `±0` (sign of negated zero) | [x] |
| 11 | `c2RotIdentity`, `c2xIdentity` | no inputs — constant return, bitwise compare | [x] |
| 12 | `c2Mulrv`, `c2MulrvT` | identity rot, unit rot `(cos,sin)`, non-unit rot, `inf`/`NaN` rot | [x] |
| 13 | `c2Mulxv` | identity `c2x`, translation only, rotation only, both | [x] |
| 14 | `c2BBVerts` | normal / zero-area / inverted / huge AABB → 4 output verts | [x] |
| 15 | `c2MakeProxy` | `type=CIRCLE` → radius, count 1, verts[0] | [x] |
| 16 | `c2MakeProxy` | `type=AABB` → radius 0, count 4, verts[0..4] | [x] |
| 17 | `c2MakeProxy` | `type=CAPSULE` → radius, count 2, verts[0..2] | [x] |
| 18 | `c2Support` | `count=1` (circle proxy), random `d` | [x] |
| 19 | `c2Support` | `count=2` (capsule proxy), random `d` incl. ties | [x] |
| 20 | `c2Support` | `count=4` (AABB proxy), random `d` incl. axis-aligned ties | [x] |
| 21 | `c2Support` | `count=8` (full array), random verts and `d` | [x] |
| 22 | `c2GJKSimplexMetric` | `count=1` / `2` / `3`, random simplex | [x] |
| 23 | `c22` | random 2-vertex simplex: `v<=0` region | [x] |
| 24 | `c22` | random 2-vertex simplex: `u<=0` region (swaps a←b) | [x] |
| 25 | `c22` | random 2-vertex simplex: interior region (`count` stays 2) | [x] |
| 26 | `c23` | random 3-vertex simplex, vertex region A (`vAB<=0 && uCA<=0`) | [x] |
| 27 | `c23` | vertex region B (`uAB<=0 && vBC<=0`) | [x] |
| 28 | `c23` | vertex region C (`uBC<=0 && vCA<=0`) | [x] |
| 29 | `c23` | edge region AB (`wABC<=0`) | [x] |
| 30 | `c23` | edge region BC (`uABC<=0`) | [x] |
| 31 | `c23` | edge region CA (`vABC<=0`) | [x] |
| 32 | `c23` | interior region (`count` stays 3) | [x] |
| 33 | `c23` | degenerate triangle (`area == 0`, all `*ABC == 0`) | [x] |
| 34 | `c2D` | `count=1`, `count=2` (both `c2Skew` and `c2CCW90` branches), `count=3` | [x] |
| 35 | `c2L` | `count=1`, `count=2`, random `div`/`u` | [x] |
| 36 | `c2Witness` | `count=1`, `2`, `3`; random `div`/`u` | [x] |
| 37 | `c2AABBtoAABB` | random AABB pairs: overlapping / touching / separated on each axis | [x] |
| 38 | `c2AABBtoAABB` | zero-area and inverted AABBs | [x] |
| 39 | `c2CircletoCircle` | random pairs: overlap / exact touch (`d2 == r2`) / separated; `r=0`; negative `r` | [x] |
| 40 | `c2CircletoAABB` | circle centre inside / outside / on each face / on a corner; `r=0`; inverted AABB | [x] |
| 41 | `c2CircletoCapsule` | `da<0` branch (before segment start) | [x] |
| 42 | `c2CircletoCapsule` | `da>=0 && db<0` branch (projection inside segment) | [x] |
| 43 | `c2CircletoCapsule` | `da>=0 && db>=0` branch (past segment end) | [x] |
| 44 | `c2CircletoCapsule` | degenerate capsule `a == b` (`n == 0`) | [x] |
| 45 | `c2AABBtoCapsule` | random AABB + capsule, `use_radius=1` via the wrapper; overlap and separated | [x] |
| 46 | `c2CapsuletoCapsule` | random capsule pairs: crossing, parallel, collinear, degenerate | [x] |
| 47 | `c2GJK` | `CIRCLE`×`CIRCLE`, `use_radius=0`, all pointers NULL except none | [x] |
| 48 | `c2GJK` | `CIRCLE`×`CIRCLE`, `use_radius=1`, `outA`/`outB`/`iterations` non-NULL | [x] |
| 49 | `c2GJK` | `CIRCLE`×`AABB`, `use_radius` ∈ {0,1} | [x] |
| 50 | `c2GJK` | `CIRCLE`×`CAPSULE`, `use_radius` ∈ {0,1} | [x] |
| 51 | `c2GJK` | `AABB`×`CIRCLE`, `use_radius` ∈ {0,1} | [x] |
| 52 | `c2GJK` | `AABB`×`AABB`, `use_radius` ∈ {0,1} | [x] |
| 53 | `c2GJK` | `AABB`×`CAPSULE`, `use_radius` ∈ {0,1} | [x] |
| 54 | `c2GJK` | `CAPSULE`×`CIRCLE`, `use_radius` ∈ {0,1} | [x] |
| 55 | `c2GJK` | `CAPSULE`×`AABB`, `use_radius` ∈ {0,1} | [x] |
| 56 | `c2GJK` | `CAPSULE`×`CAPSULE`, `use_radius` ∈ {0,1} | [x] |
| 57 | `c2GJK` | all 9 type pairs, `ax_ptr` non-NULL translation-only transform | [x] |
| 58 | `c2GJK` | all 9 type pairs, `bx_ptr` non-NULL rotation-only transform | [x] |
| 59 | `c2GJK` | all 9 type pairs, both transforms rotation+translation, random | [x] |
| 60 | `c2GJK` | all 9 type pairs, non-unit (unnormalised) `c2r` in the transform | [x] |
| 61 | `c2GJK` | all 9 type pairs, cold cache (`count=0`), cache written back | [x] |
| 62 | `c2GJK` | all 9 type pairs, warm cache reused across 2 consecutive calls (same shapes) | [x] |
| 63 | `c2GJK` | all 9 type pairs, warm cache reused after moving shape B (typical broadphase use) | [x] |
| 64 | `c2GJK` | all 9 type pairs, deeply overlapping shapes (`hit` path, `count==3`) | [x] |
| 65 | `c2GJK` | all 9 type pairs, exactly touching shapes | [x] |
| 66 | `c2GJK` | all 9 type pairs, identical shapes (degenerate search direction → early break) | [x] |
| 67 | `c2GJK` | all 9 type pairs, far-apart shapes (single-iteration convergence) | [x] |
| 68 | `c2GJK` | all 9 type pairs, huge coordinates (~1e18) | [x] |
| 69 | `c2GJK` | all 9 type pairs, tiny coordinates (~1e-30, subnormal intermediates) | [x] |
| 70 | `c2GJK` | all 9 type pairs, zero radii | [x] |
| 71 | `c2GJK` | all 9 type pairs, large radii (radius > separation → midpoint collapse) | [x] |
| 72 | `c2GJK` | zero-area AABB and degenerate capsule (`a==b`) proxies | [x] |
| 73 | `c2GJK` | inverted AABB (`min > max`) proxies | [x] |
| 74 | `c2Collided` | all 9 `(typeA,typeB)` pairs, randomized shapes | [x] |
| 75 | `omni_collide` | all 9 `(type_a,type_b)` pairs, randomized 5-float payloads | [x] |
| 76 | `omni_collide` | all 9 pairs, payloads restricted to a small grid (dense near-boundary coverage) | [x] |
| 77 | `omni_collide` | all 9 pairs, payloads containing `NaN`/`±inf`/`±0`/subnormals | [x] |
| 78 | `ptr_from_parts` | `typ` ∈ {CIRCLE, AABB, CAPSULE}: returned struct bytes compared | [x] |
| 79 | end-to-end pipeline | `ptr_from_parts` → `c2Collided` composed across libraries (C ptr into Rust `c2Collided` and vice-versa) | [x] |
| 80 | end-to-end pipeline | `c2MakeProxy` → `c2Support` → `c22`/`c23` → `c2Witness` driven manually, mixed step-by-step comparison | [x] |

## Status

All 80 rows pass. They are implemented in `tests/configs.rs`, which loads both
the C `.so` and the Rust `.so` with `libloading` and compares every result
bit-for-bit (`f32::to_bits`, so NaN payloads and signed zeros count).

Volume per run: `N = 60_000` randomized inputs per scalar/vector row and
`GJK_N = 5_000` per `c2GJK` row — and each `c2GJK` row loops over all 9
`(typeA, typeB)` pairs and both `use_radius` values, so the GJK rows alone make
several million cross-library comparisons. Seeds are fixed per test, so a failure
reproduces exactly.

| test | rows |
|------|------|
| `row01_c2V` … `row13_c2Mulxv` | 1-13 |
| `row14_c2BBVerts`, `row15_16_17_c2MakeProxy`, `rows18_21_c2Support` | 14-21 |
| `row22_c2GJKSimplexMetric`, `rows23_25_c22`, `rows26_33_c23`, `row34_c2D`, `row35_c2L`, `row36_c2Witness` | 22-36 |
| `rows37_38_c2AABBtoAABB`, `row39_c2CircletoCircle`, `row40_51_c2CircletoAABB`, `rows41_44_c2CircletoCapsule`, `row45_c2AABBtoCapsule`, `row46_c2CapsuletoCapsule` | 37-46 |
| `rows47_56_gjk_type_matrix`, `rows57_60_gjk_transforms`, `rows61_63_gjk_cache`, `rows64_67_gjk_separations`, `rows68_71_gjk_magnitudes`, `rows72_73_gjk_degenerate` | 47-73 |
| `row74_c2Collided`, `row75_omni_collide_random`, `row76_omni_collide_grid`, `row77_omni_collide_float_zoo`, `row78_ptr_from_parts_valid`, `row79_cross_library_pipeline`, `row80_manual_pipeline` | 74-80 |

Rows 23-33 additionally assert *branch coverage*: `c22`/`c23` tag their simplex
vertices with distinguishable `iA` markers, and the test fails unless every one
of the three `c22` regions and all seven `c23` regions (three vertex regions,
three edge regions, interior) was actually reached.

## Harness sensitivity

The suite was validated with five semantic mutations of `src/lib.rs`; every one
was caught, so the comparisons are not vacuous:

| mutation | failing tests |
|----------|---------------|
| `c2CircletoCircle`/`c2CircletoCapsule`: `d2 < r2` → `d2 <= r2` | 4 |
| `c22`: `v <= 0` → `v < 0` | 6 |
| `c23`: vertex-region C copies `verts[1]` instead of `verts[2]` | 16 |
| `c2GJK`: `dist > rA+rB` → `dist >= rA+rB` | 4 |
| `c2Support`: `dot > dmax` → `dot >= dmax` | 12 |

One mutation was *not* caught and is genuinely unobservable: swapping the
operands of `A.r + B.r` in `c2CircletoCircle`. That only changes which NaN
payload survives, and the value is immediately squared and fed to a comparison
whose result is `false` for any NaN — so no caller can distinguish the two.

## Configuration axes with no rows

`translation/Cargo.toml` declares no `[features]`, so there is exactly one
compile-time configuration; `run_all.sh` derives the feature powerset
mechanically from `Cargo.toml` and therefore runs the single default
combination. There are no `#ifdef`s in `c_src` either. To cover the remaining
build axis, `run_all.sh` also re-runs the whole suite against the
**debug-profile** cdylib (`RUST_SO=target/debug/...`); it passes identically,
which is the check that the inline-`asm!` NaN fixes are not an artefact of one
optimisation level.
