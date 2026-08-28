# CONFIGS.md — Phase A configuration-surface table (VALID inputs)

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`: every axis
below corresponds to an `if` / `switch` / ternary the C code actually branches
on, or to an input *shape* the C code special-cases.

## Axes the C code branches on

| axis | values the C distinguishes | where |
|------|-----------------------------|-------|
| `A1` shape type of A | `C2_TYPE_CAPSULE=0`, `C2_TYPE_CIRCLE=1`, `C2_TYPE_AABB=2` | `c2MakeProxy` L109, `c2Collided` L572, `ptr_from_parts` L619 |
| `A2` shape type of B | same 3 | `c2Collided` L574/586/598 |
| `A3` `use_radius` | `0`, non-`0` | `c2GJK` L477 |
| `A4` `ax_ptr` | `NULL` (→identity), identity, pure translation, pure rotation, rotation+translation, non-unit rotor | `c2GJK` L363, `c2Mulxv`, `c2MulrvT` |
| `A5` `bx_ptr` | same 6 | `c2GJK` L367 |
| `A6` `cache` | `NULL`; non-`NULL` with `count == 0` (cold); non-`NULL` warm from a previous `c2GJK` (`cache_was_read`); non-`NULL` poisoned so the L400 metric test *fails* | `c2GJK` L378-403, L495-504 |
| `A7` out params | `outA`/`outB`/`iterations` each `NULL` or non-`NULL` | `c2GJK` L505-510 |
| `A8` simplex `count` | `1`, `2`, `3` (plus the 2 branches of `c22` and the 7 of `c23`) | `c22`, `c23`, `c2D`, `c2L`, `c2Witness`, `c2GJKSimplexMetric` |
| `A9` proxy vertex count | `1` (circle), `2` (capsule), `4` (AABB); `c2Support` is also reachable with `8` | `c2MakeProxy`, `c2Support` L296 |
| `A10` relative pose | separated / exactly touching / overlapping / nested / coincident / far apart | `c2GJK` L436, L480; all `c2*to*` predicates |
| `A11` degeneracy | `r == 0`, `r < 0`, capsule with `a == b`, AABB with `min == max`, inverted AABB (`min > max`) | no validation anywhere; changes which branch is taken |
| `A12` value magnitude | ordinary, `±0.0`, subnormal, `±FLT_MAX`, `±inf`, `NaN` | every FP compare/divide |

## Row table

`[x]` = a differential test drives **both** `.so`s in exactly this configuration
with **many** randomized inputs (fixed seed) and asserts bit-identical outputs.
The Phase B tests are split across three files by group, and each test is named
after its row:

| rows | file | test names |
|------|------|------------|
| 1-21  | `tests/phase_b_math.rs`    | `row01_*` .. `row21_*` |
| 22-35 | `tests/phase_b_simplex.rs` | `row22_*` .. `row35_*` |
| 36-50 | `tests/phase_b_gjk.rs`     | `row36_*` .. `row50_*` |
| 51-71 | `tests/phase_b_api.rs`     | `row51_*` .. `row71_*` |

Per-row iteration counts come from `tests/common/mod.rs`: `N = 4000` for cheap
rows, `N_SLOW = 800` per type-pair for the `c2GJK` rows (so 7200 calls per row
across the 9 pairs). Several rows additionally assert *coverage*: that both
outcomes of a boolean predicate were produced, that every branch of `c22`/`c23`
was visited, that the warm-cache path was entered, that ties were generated,
etc. — so a row cannot pass by accident on one-sided data.

### Group 1 — lowest-level vector math

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `c2V`, `c2Neg`, `c2Skew`, `c2CCW90` | A12=ordinary: uniform random f32 in ±1e3 | [x] |
| 2  | `c2V`, `c2Neg`, `c2Skew`, `c2CCW90` | A12=extreme: fully random 32-bit patterns (NaN / ±inf / subnormal / ±0) | [x] |
| 3  | `c2Add`, `c2Sub`, `c2Dot`, `c2Det2` | A12=ordinary random pairs | [x] |
| 4  | `c2Add`, `c2Sub`, `c2Dot`, `c2Det2` | A12=extreme (`inf-inf`, `0*inf`, `±FLT_MAX` overflow, mixed NaN) | [x] |
| 5  | `c2Mulvs`, `c2Div` | A12=ordinary vector × ordinary scalar | [x] |
| 6  | `c2Mulvs`, `c2Div` | A12=extreme scalar (`±0.0`, `±inf`, NaN, subnormal) × extreme vector | [x] |
| 7  | `c2Maxv`, `c2Minv` | A12=ordinary random pairs (incl. exact ties) | [x] |
| 8  | `c2Maxv`, `c2Minv` | A12=extreme: NaN in a, in b, in both, `+0.0` vs `-0.0` ties | [x] |
| 9  | `c2Clampv` | `lo <= hi` (well-formed range), `a` inside / below / above | [x] |
| 10 | `c2Clampv` | A11: inverted range `lo > hi` | [x] |
| 11 | `c2Clampv` | A12=extreme: NaN / ±inf in `a`, `lo`, `hi` | [x] |
| 12 | `c2Len`, `c2Norm` | A12=ordinary random vectors | [x] |
| 13 | `c2Len`, `c2Norm` | A12=extreme: zero vector, subnormal, `1e20` (dot overflows to `inf`), NaN, `±inf` | [x] |
| 14 | `c2RotIdentity`, `c2xIdentity` | no inputs — constant result, compared bitwise | [x] |
| 15 | `c2Mulrv`, `c2MulrvT`, `c2Mulxv` | A4=unit rotor (`cos θ`, `sin θ` for random θ) + random translation | [x] |
| 16 | `c2Mulrv`, `c2MulrvT`, `c2Mulxv` | A4=degenerate rotor: zero rotor, non-unit rotor, NaN/inf rotor; random vector | [x] |

### Group 2 — proxy construction

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 17 | `c2BBVerts` | well-formed AABB (`min < max`), ordinary values | [x] |
| 18 | `c2BBVerts` | A11: degenerate (`min == max`) and inverted (`min > max`) box; A12 extreme values | [x] |
| 19 | `c2MakeProxy` | A1=`C2_TYPE_CIRCLE`, random circle, destination proxy pre-poisoned with a known byte pattern; all 72 bytes compared | [x] |
| 20 | `c2MakeProxy` | A1=`C2_TYPE_AABB`, random / degenerate / inverted box, pre-poisoned proxy | [x] |
| 21 | `c2MakeProxy` | A1=`C2_TYPE_CAPSULE`, random / degenerate (`a == b`) capsule, pre-poisoned proxy | [x] |

### Group 3 — simplex internals (low-level entry points, driven directly)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 22 | `c2Support` | A9=1 (circle proxy), random search direction `d` | [x] |
| 23 | `c2Support` | A9=2 (capsule proxy), random `d` incl. exact ties (`dot == dmax`, which does **not** update `imax`) | [x] |
| 24 | `c2Support` | A9=4 (AABB proxy), random `d`, incl. axis-aligned `d` producing ties | [x] |
| 25 | `c2Support` | A9=8 (full `c2Proxy.verts[]`), random `d`, NaN dots (`>` always false → `imax` stays `0`) | [x] |
| 26 | `c2GJKSimplexMetric` | A8=1, 2, 3 with random `p` values (each `count` selects a different formula) | [x] |
| 27 | `c22` | A8=2, targeted `v <= 0` (origin beyond A) | [x] |
| 28 | `c22` | A8=2, targeted `u <= 0` (origin beyond B) | [x] |
| 29 | `c22` | A8=2, targeted interior (`u > 0 && v > 0`) → `count = 2` | [x] |
| 30 | `c22` | A8=2, fully random `sA`/`sB`/`p`/`u`/`iA`/`iB`; whole 152-byte simplex compared | [x] |
| 31 | `c23` | A8=3, each of the 7 branches targeted by construction (3 vertex regions, 3 edge regions, interior) | [x] |
| 32 | `c23` | A8=3, fully random simplex bytes; whole struct compared | [x] |
| 33 | `c2D` | A8=1; A8=2 with `c2Det2(ab, -a) > 0` (skew) and `<= 0` (CCW90); A8=3 | [x] |
| 34 | `c2L` | A8=1, 2, 3 with random `div` (incl. `div` making `den` huge) | [x] |
| 35 | `c2Witness` | A8=1, 2, 3 with random `sA`/`sB`/`u`/`div` | [x] |

### Group 4 — `c2GJK` (the low-level composed pipeline)

All rows sweep A1×A2 = all 9 ordered type pairs and use many randomized shapes.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 36 | `c2GJK` | A3=1, A4=A5=`NULL`, A6=`NULL`, A7=all non-`NULL`; A10 random poses | [x] |
| 37 | `c2GJK` | A3=0 (no radius), A4=A5=`NULL`, A6=`NULL`, A7 all non-`NULL` | [x] |
| 38 | `c2GJK` | A3=1, A4=explicit identity `c2x`, A5=`NULL` (asymmetric null handling) | [x] |
| 39 | `c2GJK` | A3=1, A4=pure translation, A5=pure translation | [x] |
| 40 | `c2GJK` | A3=1, A4=A5=unit rotor + translation (random θ) | [x] |
| 41 | `c2GJK` | A3=1, A4=A5=non-unit / scaled rotor (the C never normalises) | [x] |
| 42 | `c2GJK` | A3=1, A6=cold cache (`count == 0`), single call; cache contents compared afterwards | [x] |
| 43 | `c2GJK` | A3=1, A6=**warm** cache: call twice in a row on the same cache with the shapes nudged between calls (exercises `cache_was_read`, `c2GJKSimplexMetric` on the reloaded simplex, and the cache write-back). 4-call chains too | [x] |
| 44 | `c2GJK` | A3=1, A7: `outA=NULL`; `outB=NULL`; `iterations=NULL`; all three `NULL` | [x] |
| 45 | `c2GJK` | A10=overlapping (`hit` path, `s.count == 3`) — shapes forced to interpenetrate | [x] |
| 46 | `c2GJK` | A10=coincident (A and B identical shapes at the same place) | [x] |
| 47 | `c2GJK` | A10=far apart (coordinates ~`1e6`..`1e9`, hits `d1 > d0` and the 20-iteration cap) | [x] |
| 48 | `c2GJK` | A11=degenerate shapes: `r == 0` circle, `a == b` capsule, `min == max` AABB | [x] |
| 49 | `c2GJK` | A11=negative radii and inverted AABBs | [x] |
| 50 | `c2GJK` | A12=extreme coordinates (`±FLT_MAX`, subnormal, `±0.0`) — no NaN (would make the result nondeterministic in payload only) | [x] |

### Group 5 — boolean predicates (mid-level entry points)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 51 | `c2AABBtoAABB` | A10 sweep: separated on x, separated on y, touching, overlapping, nested; random | [x] |
| 52 | `c2AABBtoAABB` | A11/A12: inverted boxes, `min == max`, `±inf`, NaN | [x] |
| 53 | `c2CircletoCircle` | A10 sweep: separated / tangent (`d2 == r2` → `0`) / overlapping / nested / coincident | [x] |
| 54 | `c2CircletoCircle` | A11: `r == 0`, negative `r`, `A.r + B.r == 0`; A12 extremes | [x] |
| 55 | `c2CircletoAABB` | A10: centre inside box, outside on a face, outside at a corner, exactly on the boundary | [x] |
| 56 | `c2CircletoAABB` | A11: `r == 0`, negative `r`, degenerate + inverted box; A12 extremes | [x] |
| 57 | `c2CircletoCapsule` | A10: each of the 3 nearest-feature branches (`da < 0`, `db < 0`, else) driven deliberately | [x] |
| 58 | `c2CircletoCapsule` | A11: `a == b` capsule (`n == 0`), zero/negative radii; A12 extremes | [x] |
| 59 | `c2AABBtoCapsule` | A10 random poses (delegates to `c2GJK` with `use_radius = 1`) | [x] |
| 60 | `c2AABBtoCapsule` | A11 degenerate/inverted/negative-radius inputs | [x] |
| 61 | `c2CapsuletoCapsule` | A10: crossing, parallel, collinear, separated, coincident segments | [x] |
| 62 | `c2CapsuletoCapsule` | A11: `a == b` on one/both capsules, zero/negative radii | [x] |

### Group 6 — dispatch + public one-shot API

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 63 | `ptr_from_parts` | A1=`C2_TYPE_CIRCLE`; the 12 malloc'd bytes read back and compared | [x] |
| 64 | `ptr_from_parts` | A1=`C2_TYPE_AABB`; 16 bytes compared | [x] |
| 65 | `ptr_from_parts` | A1=`C2_TYPE_CAPSULE`; 20 bytes compared | [x] |
| 66 | `c2Collided` | A1×A2 = all 9 valid ordered pairs, random shapes in caller-owned buffers | [x] |
| 67 | `c2Collided` | A1×A2 = all 9, A11 degenerate/inverted/negative-radius shapes | [x] |
| 68 | `omni_collide` | A1×A2 = all 9, A12=ordinary random floats (the full public pipeline) | [x] |
| 69 | `omni_collide` | A1×A2 = all 9, A10-targeted: guaranteed-overlapping and guaranteed-separated placements | [x] |
| 70 | `omni_collide` | A1×A2 = all 9, A12=extreme floats (`±0`, subnormal, `±FLT_MAX`, `±inf`, NaN) | [x] |
| 71 | `omni_collide` | A1×A2 = all 9, A11: zero and negative radii, `a == b` capsules, inverted AABBs | [x] |

## Verification result

All 71 rows pass, in both the `debug` and `release` profiles of the Rust cdylib,
against the GCC-built C `.so`. Total: 110 test functions across 7 test binaries
(plus 1 `#[ignore]`d diagnostic), ~700k differential comparisons per run.

Each row uses a *fixed* per-row seed so any failure is reproducible, but the
whole suite can be re-sampled over a different region of the input space with
`SEED_OFFSET=<n> cargo test`. It was run with 16 different offsets
(`0..=7`, `11`, `23`, `47`, `101`, `999`, `31337`, `65536`, `1000003`) —
110 passed / 0 failed every time — so the fixed seeds are not merely lucky.

Two notes on what was *not* asserted, both C-side undefined behaviour rather
than translation gaps (see `ERRORS.md`):

* out-of-range `cache->iA`/`iB` indices, which make the C read its
  uninitialised automatic `c2Proxy` (row 27 of `ERRORS.md`);
* an out-of-range `C2_TYPE` passed to `c2GJK`, for the same reason.

Test adequacy is evidenced two independent ways:

* **Mutation testing** — `MUTATION.md`: 112/123 seeded defects killed, the
  remaining 11 proved equivalent.
* **C-side coverage** — the suite was re-run against a separately compiled
  `--coverage` build of `c_src/src/lib.c` (via the `$C_SO` harness override).
  All 111 tests pass against that second C build too, and it reaches
  **100% of 445 lines**, **100% of 157 branches**, **156/157 branch arcs** and
  **100% of 149 calls**. The one untaken arc is the `while (iter < 20)` cap
  exit, which `ERRORS.md` row 19 proves is structurally unreachable. See the
  "C-side coverage" section of `ERRORS.md` for the exact recipe.
