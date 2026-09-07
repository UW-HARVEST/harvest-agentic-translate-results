# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the branches in `c_src/src/lib.c`. Axes the C code
actually distinguishes:

**A. Runtime options of `c2GJK`** (the only function with options):

| axis | values the C branches on | branch site |
|------|--------------------------|-------------|
| `typeA`, `typeB` | `C2_TYPE_CIRCLE` (count 1, radius r) / `C2_TYPE_AABB` (count 4, radius 0) / `C2_TYPE_CAPSULE` (count 2, radius r) | `c2MakeProxy` `switch` |
| `ax_ptr`, `bx_ptr` | `NULL` → identity / non-NULL identity / non-NULL pure translation / non-NULL pure rotation / non-NULL rotation+translation | `if (!ax_ptr)`, and `c2Mulrv` / `c2MulrvT` become non-trivial only when `r.s != 0` |
| `use_radius` | `0` (raw Minkowski distance) / `1` (radius-shrunk) — and inside `1`: the `dist > rA+rB && dist > FLT_EPSILON` branch vs. the midpoint branch | `else if (use_radius)` + inner `if` |
| `outA` / `outB` / `iterations` | `NULL` / non-NULL (each independently) | trailing `if (outA)` … |
| `cache` | `NULL` / non-NULL cold (`count==0`) / non-NULL warm (written by a previous call) / warm re-used after the shapes moved | `if (cache)`, `cache_was_good`, `cache_was_read` |

**B. Input shapes / data configurations the C special-cases:**

| axis | values |
|------|--------|
| separation | widely separated / just touching / overlapping / deeply overlapping / identical shapes |
| resulting simplex `count` | 1 (vertex region) / 2 (edge region) / 3 (containment → `hit=1`) — the `c22` / `c23` / `if (s.count==3)` branches |
| termination path | `count==3` (`hit`) / `d1 > d0` / `dot(d,d) < FLT_EPSILON²` / `dup` / `iter == 20` |
| degeneracy | zero-radius circle / zero-extent AABB (min==max) / inverted AABB (min>max) / capsule with `a==b` / negative radius |
| magnitude | ~1 / ~1e3 / ~1e-6 (denormal-adjacent) / ~1e18 (overflow-adjacent) |

**C. Low-level entry points** (all are public and tested directly, not only
through `c2GJK`): the pure-vector helpers, the simplex mutators `c22`/`c23`,
the simplex queries `c2D`/`c2L`/`c2Witness`/`c2GJKSimplexMetric`, `c2Support`,
`c2MakeProxy`/`c2BBVerts`, plus `c2GJK` and `gjk_cache`.

Every row is driven with **many randomized inputs (fixed seed, xorshift64\*)**,
not one hand-picked value, and asserted **bit-for-bit** (`to_bits()` on every
returned/written `f32`, so `NaN` payloads and `-0.0` are compared exactly).

---

## Rows

### Group 1 — pure vector helpers (lowest level)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random finite `(x,y)`; plus `±0.0`, `±inf`, `NaN`, denormals | [x] |
| 2 | `c2Mulvs` | random `c2v` × random scalar; plus scalar `0`, `-0.0`, `inf`, `NaN`; overflow-producing pairs | [x] |
| 3 | `c2Sub` | random pairs; equal vectors (→ `±0.0` signs); `inf - inf` (→ NaN); NaN operands | [x] |
| 4 | `c2Add` | random pairs; `x + (-x)` (→ `+0.0`); `inf + -inf`; NaN operands | [x] |
| 5 | `c2Dot` | random pairs; orthogonal pairs (exact `0`); overflow-to-`inf`; NaN; cancellation cases (`a.x*b.x == -a.y*b.y`) | [x] |
| 6 | `c2Det2` | random pairs; parallel pairs (exact `0`); antiparallel; NaN; overflow | [x] |
| 7 | `c2Len` | random vectors; `(0,0)`; huge (overflowing dot); tiny/denormal; NaN component | [x] |
| 8 | `c2Neg` | random; `±0.0` (sign flip); `±inf`; NaN (sign of payload) | [x] |
| 9 | `c2Skew`, `c2CCW90` | random; `(0,0)`; NaN / signed-zero components | [x] |
| 10 | `c2Div` | random vector × random divisor; divisor `0`, `-0.0`, `inf`, `NaN`, denormal | [x] |
| 11 | `c2Norm` | random vectors; `(0,0)` (→ NaN); unit vectors; huge; NaN | [x] |
| 12 | `c2Maxv`, `c2Minv` | random pairs; equal; `+0.0` vs `-0.0` both orders; NaN in `a` only, in `b` only, in both (order-dependent result) | [x] |
| 13 | `c2Clampv` | random `a`/`lo`/`hi` with `lo<hi`; `a` below / inside / above; `lo==hi`; **inverted `lo>hi`**; NaN in each of the three args | [x] |
| 14 | `c2RotIdentity`, `c2xIdentity` | no inputs — exact bit pattern of the returned struct | [x] |
| 15 | `c2Mulrv` | random `c2r` × random `c2v`; `c2r` identity (`c:1,s:0`); pure 90°/180° rotations; non-unit `c2r`; NaN/inf in `c2r` and in `c2v` | [x] |
| 16 | `c2MulrvT` | same matrix as row 15 (transpose path, incl. the `-a.s * b.x` sign-of-NaN case); random + identity + NaN + inf | [x] |
| 17 | `c2Mulxv` | random `c2x` × random `c2v`; `c2x` identity; pure translation (`s==0`); pure rotation (`p==0`); rotation+translation; NaN/inf | [x] |

### Group 2 — proxy construction

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 18 | `c2BBVerts` | random AABB; `min == max` (degenerate point box); **inverted `min > max`**; huge / NaN / signed-zero corners | [x] |
| 19 | `c2MakeProxy` | `type = C2_TYPE_CIRCLE`, random `c2Circle`; radius `0`, negative, huge, NaN — verify `radius`/`count=1`/`verts[0]` and that `verts[1..8]` are untouched | [x] |
| 20 | `c2MakeProxy` | `type = C2_TYPE_AABB`, random `c2AABB` incl. inverted + degenerate — verify `radius=0`/`count=4`/`verts[0..4]`, `verts[4..8]` untouched | [x] |
| 21 | `c2MakeProxy` | `type = C2_TYPE_CAPSULE`, random `c2Capsule`; `a==b`; radius `0`/negative/NaN — verify `radius`/`count=2`/`verts[0..2]`, `verts[2..8]` untouched | [x] |

### Group 3 — simplex primitives (driven with hand-built `c2Simplex` state)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 22 | `c2GJKSimplexMetric` | `count = 1` (→ 0); `count = 2` random points; `count = 3` random points; also `count = 0/4/-1` (default path) | [x] |
| 23 | `c22` | `count = 2`, random `a.p`/`b.p` covering all three branches: `v<=0` (vertex A), `u<=0` (vertex B), else (edge); plus `a.p==b.p`, collinear-with-origin, NaN | [x] |
| 24 | `c23` | `count = 3`, random triangles covering **all seven** branches (3 vertex regions, 3 edge regions, interior); plus CW and CCW winding, degenerate/collinear, all-equal points, NaN | [x] |
| 25 | `c2D` | `count = 1`; `count = 2` with `det2(ab,-a.p) > 0` (skew) and `<= 0` (CCW90) and `== 0`; `count = 3`; `count = 0/4` | [x] |
| 26 | `c2L` | `count = 1`; `count = 2` with random `u`/`div` (incl. `div=0`, `div=NaN`, `u=0`); `count = 3` / `0` (default) | [x] |
| 27 | `c2Witness` | `count = 1/2/3` with random `sA`/`sB`/`u`/`div`; `div = 0` / `NaN` / `-0.0`; `count = 0/4/-1` (default → zeros) | [x] |
| 28 | `c2Support` | `count = 1` … `8` with random verts and random `d`; `d = (0,0)`; all-equal verts (tie → index 0); NaN verts; `count = 0` and negative | [x] |

### Group 4 — `c2GJK`, shape-type cross-product (`use_radius = 1`, `cache = NULL`, identity transforms)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 29 | `c2GJK` | circle × circle — separated / touching / overlapping / concentric, random | [x] |
| 30 | `c2GJK` | circle × AABB — separated / edge-region / corner-region / inside, random | [x] |
| 31 | `c2GJK` | circle × capsule — separated / overlapping / centre-on-segment, random | [x] |
| 32 | `c2GJK` | AABB × circle (reversed operand order of row 30) | [x] |
| 33 | `c2GJK` | AABB × AABB — disjoint / touching / overlapping / nested, random | [x] |
| 34 | `c2GJK` | AABB × capsule — the `gjk_cache` non-reverse case, random | [x] |
| 35 | `c2GJK` | capsule × circle | [x] |
| 36 | `c2GJK` | capsule × AABB — the `gjk_cache` reverse case, random | [x] |
| 37 | `c2GJK` | capsule × capsule — parallel / crossing / collinear / coincident segments, random | [x] |

### Group 5 — `c2GJK`, option axes (each crossed with all 9 type pairs where meaningful)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 38 | `c2GJK` | `use_radius = 0`, all 9 type pairs, random shapes | [x] |
| 39 | `c2GJK` | `use_radius = 1`, forced into the `dist > rA+rB && dist > eps` branch (well-separated, positive radii) | [x] |
| 40 | `c2GJK` | `use_radius = 1`, forced into the midpoint branch (`dist <= rA+rB`, overlapping) | [x] |
| 41 | `c2GJK` | `use_radius` = `2` / `-1` / `INT_MIN` (non-canonical truthy values) | [x] |
| 42 | `c2GJK` | `ax_ptr = NULL, bx_ptr = NULL` (identity fallback) | [x] |
| 43 | `c2GJK` | `ax_ptr` = explicit identity, `bx_ptr = NULL` (and the mirror) — must equal row 42 bit-for-bit | [x] |
| 44 | `c2GJK` | `ax_ptr`/`bx_ptr` = pure translation (`r = {1,0}`, random `p`) | [x] |
| 45 | `c2GJK` | `ax_ptr`/`bx_ptr` = pure rotation (random unit `{c,s}`, `p = 0`) — exercises `c2Mulrv` **and** `c2MulrvT` inside the support function | [x] |
| 46 | `c2GJK` | `ax_ptr`/`bx_ptr` = rotation + translation, both non-identity, random | [x] |
| 47 | `c2GJK` | non-unit / scaled `c2r` (`c`,`s` random, not normalised) — the C never validates | [x] |
| 48 | `c2GJK` | `outA = NULL`; `outB = NULL`; both NULL — return value must still match | [x] |
| 49 | `c2GJK` | `iterations = NULL`; and non-NULL to compare the iteration count itself | [x] |

### Group 6 — `c2GJK` cache axis

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 50 | `c2GJK` | `cache = NULL` (baseline) | [x] |
| 51 | `c2GJK` | cold cache (`count = 0`) — compare the full `c2GJKCache` struct written back | [x] |
| 52 | `c2GJK` | warm cache: call twice with identical shapes, compare both results *and* the cache after each call (the `gjk_cache` `d0`/`d1` pattern) | [x] |
| 53 | `c2GJK` | warm cache re-used after the shapes **moved** — the stale-simplex quirk path | [x] |
| 54 | `c2GJK` | warm cache re-used after the shape **types changed** — all 9 → 9 type-pair transitions. Carried-over indices are folded into the new proxies' initialised range (`clamp_cache`), because an index ≥ the new `count` makes the C read uninitialised stack (ERRORS.md rows 14/15, UB) | [x] |
| 55 | `c2GJK` | long chain: 8 successive cached calls along a random motion path, comparing distance + cache at every step | [x] |
| 56 | `c2GJK` | cache carried across *different* type pairs in sequence (written by circle×circle, read by AABB×capsule, …) through all 9 pairs, indices clamped as in row 54 | [x] |

### Group 7 — data-shape / degeneracy / magnitude axes for `c2GJK`

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 57 | `c2GJK` | zero-radius circle and zero-radius capsule (`r = 0`), `use_radius` both 0 and 1 | [x] |
| 58 | `c2GJK` | negative radius (circle and capsule), `use_radius = 1` | [x] |
| 59 | `c2GJK` | degenerate AABB `min == max` (point box) | [x] |
| 60 | `c2GJK` | inverted AABB `min > max` (reversed winding) | [x] |
| 61 | `c2GJK` | capsule with `a == b` (point capsule) | [x] |
| 62 | `c2GJK` | identical shapes at the identical location (exact containment → `hit = 1`) | [x] |
| 63 | `c2GJK` | tiny magnitudes (`~1e-6`…`1e-20`, denormal-adjacent) — exercises the `dot(d,d) < FLT_EPSILON²` exit | [x] |
| 64 | `c2GJK` | huge magnitudes (`~1e18`…`1e30`) — exercises overflow to `inf` and the `d1 > d0` exit | [x] |
| 65 | `c2GJK` | mixed huge/tiny operands (catastrophic cancellation) | [x] |
| 66 | `c2GJK` | fully unconstrained random bit patterns for all shape floats (NaN / inf / denormal included), all 9 type pairs | [x] |

### Group 8 — the public `gjk_cache` entry point

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 67 | `gjk_cache` | `reverse = 0`, random finite AABB + capsule params; `a9`/`b9` non-NULL pre-poisoned (must stay untouched) | [x] |
| 68 | `gjk_cache` | `reverse = 1`, same | [x] |
| 69 | `gjk_cache` | `reverse` = `-1` / `0x7F` / `0x80` / `'A'` (`char` truthiness incl. sign) | [x] |
| 70 | `gjk_cache` | `a9 = NULL`, `b9 = NULL` (never dereferenced) | [x] |
| 71 | `gjk_cache` | degenerate / NaN / inf / huge / tiny float arguments, both `reverse` values | [x] |

---

## Row → test mapping

| rows | test file | `#[test]` count |
|------|-----------|-----------------|
| 1..17  | `tests/phase_b_vectors.rs`    | 12 |
| 18..21 | `tests/phase_b_proxy.rs`      | 4  |
| 22..28 | `tests/phase_b_simplex.rs`    | 7  |
| 29..56 | `tests/phase_b_gjk.rs`        | 22 |
| 57..66 | `tests/phase_b_degenerate.rs` | 8  |
| 67..71 | `tests/phase_b_gjk_cache.rs`  | 4  |
| (Phase C: `ERRORS.md` rows 1..60) | `tests/phase_c_errors.rs` | 30 |

**87 tests total**, all passing in both the `debug` and `release` profiles.

Several rows additionally assert *branch coverage*, so they cannot pass
vacuously: `c22` must reach all three of its outcomes, `c23` must produce
result counts 1, 2 and 3, `c2D` must take both the `c2Skew` and the `c2CCW90`
path, `use_radius` must reach both the shrink and the midpoint branch, and
identical shapes must produce `dist == 0`.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table**, so the complete set of
feature combinations is `{ default }` ≡ `{ --no-default-features }`. Both are
run by `run_all.sh`. There is **no binary target** in either project
(`c_src/CMakeLists.txt` only calls `add_library(... SHARED ...)`), so there is
no stdout comparison in scope.
