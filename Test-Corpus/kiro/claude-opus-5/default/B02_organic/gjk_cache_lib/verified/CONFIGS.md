# CONFIGS.md — Phase A configuration surface table (valid inputs)

Axes derived mechanically from the branches in `c_src/src/lib.c`:

* **shape type** (`C2_TYPE` switch in `c2MakeProxy`): `CIRCLE` (1 vert, radius),
  `AABB` (4 verts, radius forced to 0), `CAPSULE` (2 verts, radius) — for both
  the A and the B operand → 3 × 3 = 9 orderings, and A/B are **not**
  symmetric in `c2GJK` (`c2MulrvT(ax.r, -d)` vs `c2MulrvT(bx.r, d)`,
  `p = sB - sA`, `a = b` on hit).
* **transform** (`if (!ax_ptr)` / `if (!bx_ptr)`): NULL → identity, or an actual
  `c2x` with rotation `(c,s)` and translation.
* **`use_radius`** (`else if (use_radius)`): 0 / non-zero.
* **`cache`**: NULL / fresh (`count = 0`) / warm (written by a previous
  `c2GJK` call, so the `cache_was_read` path runs) / hand-built.
* **out params**: `outA`, `outB`, `iterations` each NULL or non-NULL.
* **simplex `count`** for the low-level simplex functions: 1 / 2 / 3 / 4 / 0 /
  negative (each `switch` distinguishes them).
* **geometry relation**: far apart / near-touching / touching / overlapping /
  concentric / identical — this decides `hit`, the `d1 > d0` break, the
  `Dot(d,d) < eps^2` break, the duplicate-support break, and the
  `dist > rA+rB` radius branch.
* **degenerate shapes**: zero-radius circle, zero-area AABB (min == max),
  zero-length capsule (a == b), zero-radius capsule, inverted AABB.
* **`gjk_cache`'s `reverse`**: 0 / non-zero.

Every row is exercised through the `.so` exports of **both** libraries with
many randomized inputs (`SplitMix64`, fixed seed `0x5EED_1234_5678_9ABC`) and
compared **bit-for-bit** (`f32::to_bits`, so `-0.0 != 0.0` and NaN payloads must
match too).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V`, `c2Mulvs`, `c2Sub`, `c2Add`, `c2Dot`, `c2Det2`, `c2Neg`, `c2Skew`, `c2CCW90` | random finite f32 pairs over a wide exponent range (incl. ±0, subnormals) | [x] |
| 2 | same as row 1 | random values incl. `NaN`, `±inf`, `FLT_MAX`, `FLT_MIN` | [x] |
| 3 | `c2Maxv`, `c2Minv`, `c2Clampv` | random finite triples, ordered range `lo <= hi` | [x] |
| 4 | `c2Maxv`, `c2Minv`, `c2Clampv` | NaN operands and inverted range `lo > hi` | [x] |
| 5 | `c2Len`, `c2Div`, `c2Norm` | random finite vectors, non-zero length | [x] |
| 6 | `c2Len`, `c2Div`, `c2Norm` | zero vector, huge (overflowing dot), tiny (subnormal) vectors, `b == 0` divisor | [x] |
| 7 | `c2RotIdentity`, `c2xIdentity` | no inputs — struct-return ABI check | [x] |
| 8 | `c2Mulrv`, `c2MulrvT`, `c2Mulxv` | random unit rotations `(cos t, sin t)` + random translation | [x] |
| 9 | `c2Mulrv`, `c2MulrvT`, `c2Mulxv` | non-normalised / zero / NaN rotations (the API never validates `c^2+s^2==1`) | [x] |
| 10 | `c2BBVerts` | normal AABB (`min < max`) | [x] |
| 11 | `c2BBVerts` | degenerate (`min == max`) and inverted (`min > max`) AABB | [x] |
| 12 | `c2MakeProxy` | `type = CIRCLE`, random centre + radius (incl. r = 0, r < 0) | [x] |
| 13 | `c2MakeProxy` | `type = AABB`, random / degenerate / inverted box | [x] |
| 14 | `c2MakeProxy` | `type = CAPSULE`, random endpoints (incl. a == b), radius incl. 0 | [x] |
| 15 | `c2GJKSimplexMetric` | `count = 1`, `2`, `3` with random simplex `p` values | [x] |
| 16 | `c2L` | `count = 1` and `count = 2` with random `u` / `div` (incl. `div` = 1, random, huge) | [x] |
| 17 | `c2D` | `count = 1`; `count = 2` with `Det2 > 0` (skew branch) and `Det2 <= 0` (CCW90 branch) | [x] |
| 18 | `c2Witness` | `count = 1`, `2`, `3` with random `sA`/`sB`/`u`/`div` | [x] |
| 19 | `c22` | random 2-point simplexes covering all three arms (`v<=0`, `u<=0`, interior) | [x] |
| 20 | `c23` | random 3-point simplexes covering all seven arms (3 vertex, 3 edge, 1 interior) | [x] |
| 21 | `c23` | collinear / duplicated points → `area == 0` degenerate triangle | [x] |
| 22 | `c2Support` | `count = 1, 2, 4, 8` verts, random directions incl. ties and zero direction | [x] |
| 23 | `c2GJK` | `CIRCLE` vs `CIRCLE`, identity transforms, `use_radius = 1`, `cache = NULL` | [x] |
| 24 | `c2GJK` | `CIRCLE` vs `AABB`, identity, `use_radius = 1`, `cache = NULL` | [x] |
| 25 | `c2GJK` | `CIRCLE` vs `CAPSULE`, identity, `use_radius = 1`, `cache = NULL` | [x] |
| 26 | `c2GJK` | `AABB` vs `CIRCLE`, identity, `use_radius = 1`, `cache = NULL` | [x] |
| 27 | `c2GJK` | `AABB` vs `AABB`, identity, `use_radius = 1`, `cache = NULL` | [x] |
| 28 | `c2GJK` | `AABB` vs `CAPSULE`, identity, `use_radius = 1`, `cache = NULL` | [x] |
| 29 | `c2GJK` | `CAPSULE` vs `CIRCLE`, identity, `use_radius = 1`, `cache = NULL` | [x] |
| 30 | `c2GJK` | `CAPSULE` vs `AABB`, identity, `use_radius = 1`, `cache = NULL` | [x] |
| 31 | `c2GJK` | `CAPSULE` vs `CAPSULE`, identity, `use_radius = 1`, `cache = NULL` | [x] |
| 32 | `c2GJK` | all 9 type pairs, `use_radius = 0` | [x] |
| 33 | `c2GJK` | all 9 type pairs, `ax_ptr` non-NULL (rotation + translation), `bx_ptr` NULL | [x] |
| 34 | `c2GJK` | all 9 type pairs, `ax_ptr` NULL, `bx_ptr` non-NULL | [x] |
| 35 | `c2GJK` | all 9 type pairs, both transforms non-NULL, random rotations | [x] |
| 36 | `c2GJK` | all 9 type pairs, fresh cache (`count = 0`) — cache-write path only | [x] |
| 37 | `c2GJK` | all 9 type pairs, **warm cache**: call twice in a row with the same cache (the `cache_was_read` path), compare returned dist, witness points, iterations and the full cache struct after each call | [x] |
| 38 | `c2GJK` | all 9 type pairs, warm cache **plus moved transforms** between the two calls (cache indices reused against different world positions) | [x] |
| 39 | `c2GJK` | `outA = NULL`, `outB = NULL`, `iterations = NULL` (return value only) | [x] |
| 40 | `c2GJK` | overlapping / intersecting shapes → `hit = 1` path | [x] |
| 41 | `c2GJK` | exactly touching shapes (`dist == rA + rB`) → midpoint-collapse branch | [x] |
| 42 | `c2GJK` | far-separated shapes → full radius-shrink branch | [x] |
| 43 | `c2GJK` | degenerate shapes: zero-area AABB, zero-length capsule, zero-radius circle/capsule | [x] |
| 44 | `c2GJK` | inverted AABB (`min > max`) | [x] |
| 45 | `c2GJK` | huge coordinates (1e18) and tiny coordinates (1e-20) → overflow / underflow in `Dot` and `Det2` | [x] |
| 46 | `c2GJK` | coordinates that force the 20-iteration cap / the `d1 > d0` break / the duplicate-support break | [x] |
| 47 | `c2GJK` | hand-built cache with `count = 1, 2, 3` and arbitrary in-range `iA`/`iB`, random `metric` and `div` (drives both cache-accept and cache-reject) | [x] |
| 48 | `gjk_cache` | `reverse = 0`, random AABB + capsule params (no observable output — asserted not to crash and to leave caller buffers untouched) | [x] |
| 49 | `gjk_cache` | `reverse != 0` (1, -1, 0x7f), random AABB + capsule params | [x] |
| 50 | `gjk_cache` | degenerate/non-finite AABB and capsule params | [x] |

## Test coverage map

| CONFIGS.md rows | test |
|---|---|
| 1, 2 | `phase_b_lowlevel::row01_row02_basic_vector_math` |
| 3, 4 | `phase_b_lowlevel::row03_row04_min_max_clamp` |
| 5, 6 | `phase_b_lowlevel::row05_row06_len_div_norm` |
| 7 | `phase_b_lowlevel::row07_identities` |
| 8, 9 | `phase_b_lowlevel::row08_row09_rotations` |
| 10, 11 | `phase_b_lowlevel::row10_row11_bbverts` |
| 12, 13, 14 | `phase_b_lowlevel::row12_row13_row14_make_proxy` |
| 15 | `phase_b_lowlevel::row15_simplex_metric` |
| 16 | `phase_b_lowlevel::row16_c2L` |
| 17 | `phase_b_lowlevel::row17_c2D` (asserts both the skew and the CCW90 arm are hit) |
| 18 | `phase_b_lowlevel::row18_witness` |
| 19 | `phase_b_lowlevel::row19_c22` (asserts all three arms are hit) |
| 20, 21 | `phase_b_lowlevel::row20_row21_c23` (asserts all three result arities are hit) |
| 22 | `phase_b_lowlevel::row22_support` |
| 23–31 | `phase_b_gjk::row23_to_row31_all_type_pairs_radius` |
| 32 | `phase_b_gjk::row32_all_type_pairs_no_radius` |
| 33, 34, 35 | `phase_b_gjk::row33_row34_row35_transforms` |
| 36, 37, 38 | `phase_b_gjk::row36_row37_row38_cache` (3 warm generations + moved transforms) |
| 39 | `phase_b_gjk::row39_null_out_params` |
| 40, 41, 42 | `phase_b_gjk::row40_row41_row42_hit_touch_far` (asserts each terminal branch is hit) |
| 43, 44 | `phase_b_gjk::row43_row44_degenerate_and_inverted` |
| 45, 46 | `phase_b_gjk::row45_row46_extremes_and_loop_exits` |
| 47 | `phase_b_gjk::row47_handbuilt_cache` |
| 48, 49, 50 | `phase_b_gjk::row48_row49_row50_gjk_cache` |

## Adversarial reinforcement (`tests/phase_bc_adversarial.rs`)

Randomised floating-point inputs approach the C's strict `>` / `<=` boundaries
but rarely land exactly on them. These sweeps use small integers, half-integers
and exact powers of two so the arithmetic is exact and the boundaries are
actually hit:

| test | what it forces | scale |
|---|---|---|
| `adversarial_exact_integer_grid` | exact `c2Support` ties, exact overlap/touch, warm cache over 3 generations | 145,800 differential calls |
| `adversarial_symmetric_and_ties` | identical/mirrored shapes, zero-area AABBs and zero-length capsules (total support ties), exact 90/180/270° rotations | 540,800 calls |
| `adversarial_exact_radius_boundary` | `dist == rA + rB` exactly, via Pythagorean triples, plus one ULP either side | 3-4-5 family |
| `adversarial_bulk_half_integer_random` | bulk exact-arithmetic coverage with cache feedback | 160,000 calls |
| `adversarial_extremes_with_warm_cache` | extreme magnitudes (`FLT_MAX`, `1e18`, `1e-20`, subnormals) **combined with** hand-built warm caches — the combination needed to reach `d1 == d0` | 600,000 calls |
| `regression_d1_eq_d0_boundary_witnesses` | 4 exact bit-pattern configurations sitting on the `if (d1 > d0)` boundary | fixed literals |
| `regression_deep_iteration_paths` | the deepest loop paths, `iterations` 3 through **7** (the measured maximum) | fixed literals |

The two `regression_*` tests exist because a mutation search showed the
`d1 > d0` boundary and the deep loop paths were reachable but *not* being
reached by the RNG. Keeping them as literals makes that coverage
seed-independent. Both were found by building a deliberately-wrong `.so` and
searching for an input where it disagreed with the C — see the mutation-evidence
section of `ERRORS.md`.

## Axes deliberately NOT crossed

* Cache indices are always kept `< proxy vertex count`. A larger index makes the
  C read `c2Proxy.verts[i]` that `c2MakeProxy` never wrote — an indeterminate
  value, i.e. UB (`ERRORS.md` row 46), not a valid configuration.
* `cache->count > 3` and out-of-range `C2_TYPE` values passed *through* `c2GJK`
  are UB for the same reason (`ERRORS.md` rows 36, 37). The defined halves are
  asserted in Phase C.

## Configurations

`Cargo.toml` declares no `[features]`, so the complete cross-product is the
default configuration and `--no-default-features`. `scripts/verify_all.sh`
enumerates them from `Cargo.toml`, and for each one rebuilds the cdylib, diffs
`nm -D` against the C `.so`, and runs the full suite. Both pass with 31/31
symbol parity. The suite additionally passes with the **debug**-profile cdylib
and against the C compiled at `-O0`, `-O1`, `-O2` and `-O3`.
