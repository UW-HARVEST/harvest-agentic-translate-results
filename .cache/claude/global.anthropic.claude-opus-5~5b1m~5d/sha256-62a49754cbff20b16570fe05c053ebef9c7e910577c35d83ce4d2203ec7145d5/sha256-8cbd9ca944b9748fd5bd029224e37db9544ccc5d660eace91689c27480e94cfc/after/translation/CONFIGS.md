# CONFIGS.md — Phase B configuration-surface table

Axes the C code actually branches on (derived from `c_src/src/lib.c`):

* **Shape type pair** (`C2_TYPE` × `C2_TYPE` = 9 combos) — dispatched by the
  nested `switch` in `c2Collided`; note the *asymmetric* re-ordering the C does
  (`AABB×CIRCLE` calls `c2CircletoAABB(B, A)`, `CAPSULE×AABB` calls
  `c2AABBtoCapsule(B, A)`, etc.).
* **`c2GJK` `use_radius`** flag: `0` (raw simplex distance) vs `!= 0` (radius
  shrink block). The convenience wrappers always pass `1`.
* **`c2GJK` transform pointers**: `NULL` (→ `c2xIdentity`) vs a real `c2x`
  (rotation + translation). Independently for A and B ⇒ 4 combinations.
* **`c2GJK` cache**: `NULL` / cold (`count == 0`) / warm (`count` 1, 2 or 3 with
  a plausible `metric`+`div`) / warm with hostile `metric` (`NaN`, `-1e9`).
* **`c2GJK` output pointers**: each of `outA`, `outB`, `iterations` present or
  `NULL`.
* **Proxy vertex count / shape kind**: `1` (circle), `2` (capsule), `4` (AABB) —
  drives `c2Support`'s loop length and which simplex sizes are reachable.
* **Simplex `count`** as an *input* to the low-level entry points
  (`c22`, `c23`, `c2D`, `c2L`, `c2Witness`, `c2GJKSimplexMetric`): `0,1,2,3,4+`.
* **Geometric relationship**: disjoint / touching / overlapping / identical /
  contained / degenerate (zero-size AABB, zero-length capsule, zero radius).
* **Value shape**: normal magnitudes, tiny (denormal / `FLT_EPSILON`-scale),
  huge (`1e30`, `FLT_MAX`), negative, `±0.0`, `NaN`, `±inf`, negative radii,
  inverted AABBs.

One row per combination the C treats differently. Every row is driven with
**many randomized inputs (fixed seed, xorshift PRNG)** through both `.so`s and
compared bit-for-bit (`f32::to_bits`, `i32`). `[x]` = passing.

## Level 1 — leaf vector math (`translation/tests/level1_vecmath.rs`)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `c2V` | random `(x,y)` incl. `±0`, `NaN`, `±inf`, denormals | [x] |
| 2 | `c2Mulvs` | random vector × random scalar, incl. `0`, `inf`, `NaN` | [x] |
| 3 | `c2Maxv` / `c2Minv` | random pairs, incl. equal, `±0` (sign of zero matters for the ternary), `NaN` on either side | [x] |
| 4 | `c2Clampv` | random `a`, random `lo`/`hi` incl. **inverted** (`lo > hi`), `NaN` bounds | [x] |
| 5 | `c2Sub` / `c2Add` | random pairs incl. cancellation, overflow to `inf` | [x] |
| 6 | `c2Dot` | random pairs incl. `inf*0 = NaN`, catastrophic cancellation | [x] |
| 7 | `c2Det2` | random pairs incl. collinear (det `0`/`±0`), overflow | [x] |
| 8 | `c2Len` | random vectors incl. `(0,0)`, huge (overflow in `dot`), `NaN` | [x] |
| 9 | `c2Div` | random vector ÷ random scalar incl. `0`, `±inf`, `NaN` | [x] |
| 10 | `c2Norm` | random vectors incl. `(0,0)` (→ `NaN`), huge, tiny | [x] |
| 11 | `c2Neg` / `c2Skew` / `c2CCW90` | random vectors incl. `±0` (sign flip), `NaN` | [x] |
| 12 | `c2RotIdentity` / `c2xIdentity` | no inputs — constant return, struct-by-value ABI check | [x] |
| 13 | `c2Mulrv` / `c2MulrvT` | random `c2r` (both normalized *and* unnormalized `c`,`s`) × random vector | [x] |
| 14 | `c2Mulxv` | random `c2x` (random rotation *and* translation) × random vector | [x] |

## Level 2 — proxy / support / simplex primitives (`translation/tests/level2_simplex.rs`)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 15 | `c2BBVerts` | random AABBs incl. inverted, zero-size, huge, `NaN` — all 4 output verts compared | [x] |
| 16 | `c2MakeProxy` | `type = CIRCLE`, random circle → `count 1`, `radius`, `verts[0]`; whole 72-byte proxy compared | [x] |
| 17 | `c2MakeProxy` | `type = AABB`, random AABB → `count 4`, `radius 0`, 4 verts | [x] |
| 18 | `c2MakeProxy` | `type = CAPSULE`, random capsule → `count 2`, `radius`, 2 verts | [x] |
| 19 | `c2Support` | `count = 1` (circle proxy), random direction | [x] |
| 20 | `c2Support` | `count = 2` (capsule proxy), random direction incl. exact ties (`>` not `>=`) | [x] |
| 21 | `c2Support` | `count = 4` (AABB proxy), random direction incl. axis-aligned ties | [x] |
| 22 | `c2Support` | `count = 8` (full proxy width), random verts + direction, `NaN` dots | [x] |
| 23 | `c2GJKSimplexMetric` | `count = 1` / `2` / `3`, random simplex `p` values | [x] |
| 24 | `c22` | random simplex, `count = 2`, hitting all 3 branches (`v<=0`, `u<=0`, else) — whole 152-byte simplex compared | [x] |
| 25 | `c23` | random simplex, `count = 3`, hitting all 7 branches of the `if/else` chain — whole simplex compared | [x] |
| 26 | `c2D` | `count = 1` / `2` (both `c2Skew` and `c2CCW90` sub-branches) / `3` | [x] |
| 27 | `c2L` | `count = 1` / `2` (with random `div`, incl. `0`) / other | [x] |
| 28 | `c2Witness` | `count = 1` / `2` / `3` / out-of-range, random `sA`/`sB`/`u`/`div` | [x] |

## Level 3 — `c2GJK` (the lowest-level composed entry point) (`translation/tests/level3_gjk.rs`)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 29 | `c2GJK` | all 9 `typeA × typeB` pairs, `ax=bx=NULL`, `use_radius=1`, all outputs requested, `cache=NULL`, random shapes | [x] |
| 30 | `c2GJK` | all 9 type pairs, `use_radius = 0` | [x] |
| 31 | `c2GJK` | all 9 type pairs, `ax` = random rotation+translation, `bx = NULL` | [x] |
| 32 | `c2GJK` | all 9 type pairs, `ax = NULL`, `bx` = random rotation+translation | [x] |
| 33 | `c2GJK` | all 9 type pairs, both `ax` and `bx` random transforms, `use_radius` both values | [x] |
| 34 | `c2GJK` | `cache` supplied, cold (`count = 0`) — cache contents after the call compared field-by-field | [x] |
| 35 | `c2GJK` | `cache` supplied, **warm**: the same cache reused across a sequence of slightly-perturbed calls (the real consumer pattern) | [x] |
| 36 | `c2GJK` | `cache` warm with `count = 1` / `2` / `3` and random valid `iA`/`iB` indices, random `div`, random `metric` | [x] |
| 37 | `c2GJK` | `outA`/`outB`/`iterations` each independently `NULL` (8 combinations) | [x] |
| 38 | `c2GJK` | shapes far apart (many GJK iterations, `iter` up to the cap of 20) | [x] |
| 39 | `c2GJK` | shapes deeply overlapping → `hit` branch (`count == 3`) | [x] |
| 40 | `c2GJK` | shapes exactly touching → `dist ≈ rA+rB` boundary of the radius block | [x] |
| 41 | `c2GJK` | identical shapes / coincident centres → `dist <= FLT_EPSILON` branch | [x] |
| 42 | `c2GJK` | degenerate shapes: zero-radius circle, zero-length capsule, zero-area AABB, point AABB | [x] |
| 43 | `c2GJK` | huge coordinates (`1e18`…`1e30`) → overflow/`inf` inside `c2Dot`/`c2Len` | [x] |
| 44 | `c2GJK` | tiny coordinates (`1e-30`, denormals) → `FLT_EPSILON` guards | [x] |
| 45 | `c2GJK` | negative radii on circle/capsule | [x] |
| 46 | `c2GJK` | inverted AABB (`min > max`) → non-convex vertex winding | [x] |

## Level 4 — boolean shape tests (`translation/tests/level4_shapes.rs`)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 47 | `c2AABBtoAABB` | random pairs: disjoint / touching / overlapping / nested / identical / inverted / degenerate / `NaN` | [x] |
| 48 | `c2CircletoCircle` | random pairs incl. exactly-tangent (`d2 == r2`, `<` is strict), zero + negative radii, `NaN` | [x] |
| 49 | `c2CircletoAABB` | circle centre inside / outside / on each edge / on each corner; inverted AABB; zero radius | [x] |
| 50 | `c2CircletoCapsule` | all 3 branches: `da < 0`, `db < 0` (perpendicular projection), else (`b`-end); degenerate `a == b` capsule | [x] |
| 51 | `c2CapsuletoCapsule` | parallel / crossing / collinear / disjoint / degenerate / negative radii | [x] |
| 52 | `c2AABBtoCapsule` | capsule fully inside / crossing an edge / crossing a corner / disjoint / degenerate | [x] |

## Level 5 — dispatch + public API (`translation/tests/level5_public.rs`)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 53 | `c2Collided` | `typeA = CIRCLE`, `typeB ∈ {CIRCLE, AABB, CAPSULE}` — heap-allocated shapes, random | [x] |
| 54 | `c2Collided` | `typeA = AABB`, `typeB ∈ {CIRCLE, AABB, CAPSULE}` (exercises the argument-swapping calls) | [x] |
| 55 | `c2Collided` | `typeA = CAPSULE`, `typeB ∈ {CIRCLE, AABB, CAPSULE}` (exercises the argument-swapping calls) | [x] |
| 56 | `ptr_from_parts` | `typ ∈ {CIRCLE, AABB, CAPSULE}` — resulting heap struct read back byte-for-byte and compared | [x] |
| 57 | `omni_collide` | all 9 `type_a × type_b` pairs, random `a1..a5`/`b1..b5` in a "mostly colliding" range | [x] |
| 58 | `omni_collide` | all 9 pairs, random params in a "mostly disjoint" (wide) range | [x] |
| 59 | `omni_collide` | all 9 pairs, params drawn from a small integer grid (many exact ties / touching cases) | [x] |
| 60 | `omni_collide` | all 9 pairs, params incl. `NaN`, `±inf`, `±0.0`, `FLT_MAX`, denormals, negative radii | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds **only** `add_library(... SHARED src/lib.c)` — no
`add_executable`. `translation/Cargo.toml` declares only `[lib] crate-type =
["cdylib"]` — no `[[bin]]`. There is therefore **no driver binary** whose stdout
could be compared; that completion-gate item is not applicable.

## Feature combinations

`translation/Cargo.toml` has no `[features]` table. The only combination is the
default; `--no-default-features` produces an identical build. Verified by
`translation/check_features.sh`.

---

## Phase B status: ALL 60 ROWS PASS ✅

Verified by `./run_tests.sh` (see the freshness warning below):

| test file | rows | tests | result |
|-----------|------|-------|--------|
| `tests/level1_vecmath.rs` | 1–14 | 14 | 14 passed |
| `tests/level2_simplex.rs` | 15–28 | 14 | 14 passed |
| `tests/level3_gjk.rs` | 29–46 | 18 | 18 passed |
| `tests/level4_shapes.rs` | 47–52 | 6 | 6 passed |
| `tests/level5_public.rs` | 53–60 (+60b) | 9 | 9 passed |
| `tests/errors.rs` (Phase C) | ERRORS.md 1–42 | 38 | 38 passed |
| **total** | | **99** | **99 passed, 0 failed** |

Every row is driven with thousands of randomized inputs from a fixed-seed
xorshift64\* PRNG (one distinct seed per row, so a failure is reproducible), and
outputs are compared bit-for-bit via `f32::to_bits`. Struct-returning and
struct-taking functions are compared over their whole payload — the full 72-byte
`c2Proxy`, the full 152-byte `c2Simplex`, and every field of `c2GJKCache`.

Rows are not merely "called once": each level asserts it actually reached the
branches it claims to cover, and fails otherwise —

* `row24`/`row25` count `c22`/`c23` result shapes and require every arm of the
  3-way and 7-way `if`/`else` chains to be hit,
* `row26` requires both the `c2Skew` and `c2CCW90` sub-branches of `c2D`,
* `row50` classifies `c2CircletoCapsule`'s three branches and requires all three,
* `row35` requires >100 genuine cache warm-starts,
* rows 47–59 require a *mix* of colliding and non-colliding outcomes
  (`assert!(yes > 0 && no > 0)`), so a row cannot pass by returning one constant.

### The one deliberate comparison relaxation

`BitEq for f32` treats two NaNs as equal even when sign bit / payload differ.
When both operands of an SSE arithmetic instruction are NaN the hardware
propagates the *destination* register's payload, so the sign bit of the result
depends only on which register the compiler chose; gcc `-O2` and rustc `-O3`
make different but equally valid choices for e.g. `a.x*b.x + a.y*b.y`. This is
not observable behaviour a caller can depend on. **Everything else stays
strict** — `+0.0` vs `-0.0`, `+inf` vs `-inf`, NaN vs non-NaN, and every finite
value are compared bit-for-bit. (This relaxation was the *only* divergence found
in the whole exercise; it appeared in `c2Dot`, `c2Det2` and `c2Mulrv` and only
for inputs containing `-NaN`.)

### ⚠️ `cargo test` alone is NOT sufficient — use `./run_tests.sh`

This crate's only lib target is a `cdylib`. **`cargo test` does not build a
cdylib-only lib target** (integration tests have no rlib to link), so a bare
`cargo test` loads whatever `.so` an earlier `cargo build` left behind. Verified
experimentally: touching `src/lib.rs` and running `cargo test --release` leaves
`target/release/libomni_collide_lib.so` byte-identical and *older* than the
source. Every differential assertion silently becomes a no-op.

Two defences are in place:

1. `run_tests.sh` builds the C library **and** `cargo build`s the Rust cdylib
   before running `cargo test`.
2. `tests/common/mod.rs` hard-fails with a `STALE Rust cdylib` panic if the
   `.so` is older than any `.rs` file under `src/`.

### Harness validated by mutation testing

Because all 99 tests passed on the first run, the suite was verified to actually
be capable of failing. Two single-character mutations were injected into
`src/lib.rs`:

| mutation | detected by |
|----------|-------------|
| `c2D`: `c2Det2(...) > 0.0` → `>= 0.0` | 9 rows — 26, 29, 30, 34, 35, 36, 37, 38, 42 |
| `c2CircletoCircle`: `d2 < r2` → `d2 <= r2` | 4 rows — 48, 59, 60, 60b |

13 rows across all 4 levels failed, then passed again after reverting. The suite
demonstrably detects behavioural divergence.
