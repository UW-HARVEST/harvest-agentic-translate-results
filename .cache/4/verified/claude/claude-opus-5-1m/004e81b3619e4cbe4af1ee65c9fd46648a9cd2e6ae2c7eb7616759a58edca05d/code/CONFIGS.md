# CONFIGS.md — CONFIGURATION-SURFACE TABLE (Phase A, gates Phase B)

Mechanically enumerated from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Build-time configuration

`translated_rust/Cargo.toml` has **no `[features]` section**, so the complete
set of feature combinations is:

| combo | command |
|-------|---------|
| default (= empty) | `cargo test` |
| `--no-default-features` (= empty, identical) | `cargo test --no-default-features` |

`c_src/CMakeLists.txt` has no `option()`, no `target_compile_definitions`, and
no `#ifdef` anywhere in `lib.c`/`lib.h`. There is exactly **one** build
configuration for the C side too. Both combos are run by `run_all.sh`.

## Runtime axes the C code actually branches on

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `C2_TYPE typeB` (dispatch mode) | `C2_TYPE_CIRCLE=0`, `C2_TYPE_AABB=1`, `C2_TYPE_CAPSULE=2`, `C2_TYPE_POLY=3`, **out-of-range int** | `c2CastRay` L368 |
| `const c2x *bx` | `NULL` (→ `c2xIdentity`), identity, pure translation, pure rotation, rotation+translation, non-unit `c2r` | `c2RaytoPoly` L338 |
| `c2Poly.count` | `<0`, `0`, `1`, `2`, `3`, `4`, `5..8`, `>8` (OOB read) | `c2RaytoPoly` L344 |
| ray direction `A.d` | `(0,0)`, `+x`, `-x`, `+y`, `-y`, diagonal unit, un-normalised, huge, `inf`, `NaN` | all raycasts |
| ray max distance `A.t` | `0`, small, exactly-touching, large, `inf`, `NaN`, negative | all raycasts |
| ray origin position | inside shape, outside-facing, outside-behind, exactly on boundary | all raycasts |
| `c2AABB` shape | normal, degenerate (`min==max`), inverted (`min>max`), huge, `NaN` | `c2RaytoAABB`, `c2AABBtoAABB`, `c2AABBtoPoint` |
| `c2Circle.r` | `0`, small, large, negative, `inf`, `NaN` | `c2RaytoCircle`, `c2CircleToPoint` |
| `c2Capsule` shape | `a!=b` axis-aligned, `a!=b` diagonal, `a==b` (degenerate), `r==0`, `r<0` | `c2RaytoCapsule` |
| `c2Capsule` branch reached | slab-hit early, cap-a early, cap-b early, `|yAp.x|<r` → circle a / circle b, side-hit, `y<=0` → circle a, `y>=yBb.y` → circle b, full miss | `c2RaytoCapsule` L262-308 |
| `c2RaytoAABB` winning axis | `t0` (`-x`), `t1` (`+x`), `t2` (`-y`), `t3` (`+y`), ties | L197-209 |
| float class of every scalar | `+0.0`, `-0.0`, normal, subnormal, `±FLT_MAX`, `±inf`, `NaN` | pervasive (`<`/`>` ternaries) |
| `out` pointer | valid, `NULL` on a path that never writes | L118/163/348 |

## Configuration rows

One row per combination the C treats differently. `[x]` = passes across the
randomized sweep (fixed-seed PRNG, ≥256 cases per row unless stated).

### Group 1 — leaf vector math (`tests/vec_math.rs`)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `c2V`                 | random pairs incl. `±0.0`, subnormal, `±inf`, `NaN`, `±FLT_MAX` | [x] |
| 2  | `c2Dot`               | random × random | [x] |
| 3  | `c2Dot`               | special × special (full cross product of the 14 special values) | [x] |
| 4  | `c2Len`               | random, zero vector, `inf`, `NaN`, overflowing (`dot` → `+inf`) | [x] |
| 5  | `c2Add`, `c2Sub`      | random × random + special × special | [x] |
| 6  | `c2Mulvs`             | random vector × {random, `0`, `-0.0`, `inf`, `NaN`} | [x] |
| 7  | `c2Div`               | random vector × {random, `0`, `-0.0`, `inf`, `NaN`} (unguarded `1/b`) | [x] |
| 8  | `c2Norm`              | random, unit, zero, `inf`, `NaN`, huge, subnormal | [x] |
| 9  | `c2Minv`, `c2Maxv`    | random pairs + special pairs (`NaN`/`±0.0` ternary semantics) | [x] |
| 10 | `c2Skew`, `c2CCW90`   | random + specials (sign of `-0.0`) | [x] |
| 11 | `c2Absv`              | random + specials (`-0.0` is **not** negated: `-0.0 < 0` is false) | [x] |
| 12 | `c2MulmvT`            | random `c2m` × random `c2v` + specials | [x] |
| 13 | `c2RotIdentity`, `c2xIdentity` | no arguments (constant) | [x] |
| 14 | `c2Mulrv`, `c2MulrvT` | unit rotations (`cos`,`sin` sweep), non-unit `c2r`, zero `c2r`, specials | [x] |
| 15 | `c2MulxvT`            | identity, pure translation, pure rotation, rotation+translation, non-unit, specials | [x] |

### Group 2 — overlap predicates (`tests/overlap.rs`)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 16 | `c2AABBtoAABB` | random overlapping boxes | [x] |
| 17 | `c2AABBtoAABB` | random separated boxes (each of the 4 axes) | [x] |
| 18 | `c2AABBtoAABB` | touching exactly (`A.max.x == B.min.x`, etc.) | [x] |
| 19 | `c2AABBtoAABB` | degenerate (`min==max`) and inverted (`min>max`) boxes | [x] |
| 20 | `c2AABBtoAABB` | `NaN` / `±inf` coordinates | [x] |
| 21 | `c2AABBtoPoint` | point inside / on each of the 4 edges / outside each of the 4 sides | [x] |
| 22 | `c2AABBtoPoint` | inverted + degenerate box, `NaN` point | [x] |
| 23 | `c2CircleToPoint` | point inside, exactly on the rim (exclusive!), outside | [x] |
| 24 | `c2CircleToPoint` | `r == 0`, `r < 0`, `r == inf`, `r == NaN` | [x] |

### Group 3 — `c2RaytoCircle` (`tests/raycast_circle.rs`)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 25 | `c2RaytoCircle` | random ray × random circle, fully randomized shotgun (4096 cases) | [x] |
| 26 | `c2RaytoCircle` | ray origin outside, direction toward circle, `A.t` large enough → hit | [x] |
| 27 | `c2RaytoCircle` | ray origin outside, `A.t` too small → `t > A.t` reject | [x] |
| 28 | `c2RaytoCircle` | ray origin inside the circle (`c < 0` → `t < 0` reject) | [x] |
| 29 | `c2RaytoCircle` | ray pointing away (circle behind) | [x] |
| 30 | `c2RaytoCircle` | tangent ray (`disc ≈ 0`) | [x] |
| 31 | `c2RaytoCircle` | `A.d` un-normalised / zero / huge; `A.t` `0`/`inf`/negative | [x] |
| 32 | `c2RaytoCircle` | `B.r` `0` / negative / `inf` / `NaN` | [x] |
| 33 | `c2RaytoCircle` | `NaN` in `A.p`, `A.d`, `A.t`, `B.p` | [x] |
| 34 | `c2RaytoCircle` | `out` pre-filled with a sentinel, verify unwritten bytes match on a miss | [x] |

### Group 4 — `c2RaytoAABB` (`tests/raycast_aabb.rs`)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 35 | `c2RaytoAABB` | random ray × random box shotgun (4096 cases) | [x] |
| 36 | `c2RaytoAABB` | crossing hit through each of the 4 faces (forces each `t0..t3` winner) | [x] |
| 37 | `c2RaytoAABB` | axis-aligned rays: `+x`, `-x`, `+y`, `-y` (many offsets) | [x] |
| 38 | `c2RaytoAABB` | diagonal rays hitting corners exactly (tie-breaking between `t0..t3`) | [x] |
| 39 | `c2RaytoAABB` | ray origin inside the box | [x] |
| 40 | `c2RaytoAABB` | swept bb overlaps but SAT rejects (`d > 0`) | [x] |
| 41 | `c2RaytoAABB` | swept bb misses (early `c2AABBtoAABB` reject) | [x] |
| 42 | `c2RaytoAABB` | `A.t == 0` (`p0 == p1`, `n == (0,0)`) | [x] |
| 43 | `c2RaytoAABB` | `A.d == (0,0)` | [x] |
| 44 | `c2RaytoAABB` | degenerate box (`min == max`), zero-area, line box | [x] |
| 45 | `c2RaytoAABB` | inverted box (`min > max`) | [x] |
| 46 | `c2RaytoAABB` | box faces exactly on the ray endpoints (`da == db` → `1/0` guard) | [x] |
| 47 | `c2RaytoAABB` | `inf` / `NaN` in `A`/`B`, huge coordinates (overflow in `c2Dot`) | [x] |
| 48 | `c2RaytoAABB` | `out` sentinel preserved on each of the 3 miss paths | [x] |

### Group 5 — `c2RaytoCapsule` (`tests/raycast_capsule.rs`)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 49 | `c2RaytoCapsule` | random ray × random capsule shotgun (8192 cases) | [x] |
| 50 | `c2RaytoCapsule` | vertical capsule (`a=(0,0)`, `b=(0,h)`), ray crossing the shaft from `+x` | [x] |
| 51 | `c2RaytoCapsule` | same, crossing from `-x` (forces `out->n = c2Skew(M.y)`) | [x] |
| 52 | `c2RaytoCapsule` | ray origin inside the slab (`c2AABBtoPoint` early `return 1`) | [x] |
| 53 | `c2RaytoCapsule` | ray origin inside cap A only | [x] |
| 54 | `c2RaytoCapsule` | ray origin inside cap B only | [x] |
| 55 | `c2RaytoCapsule` | `|yAp.x| < B.r` with `yAp.y < 0` → delegate to circle A | [x] |
| 56 | `c2RaytoCapsule` | `|yAp.x| < B.r` with `yAp.y >= 0` → delegate to circle B | [x] |
| 57 | `c2RaytoCapsule` | side crossing with `y <= 0` → delegate to circle A | [x] |
| 58 | `c2RaytoCapsule` | side crossing with `y >= yBb.y` → delegate to circle B | [x] |
| 59 | `c2RaytoCapsule` | full miss (outer `if` false) — `*out` still overwritten | [x] |
| 60 | `c2RaytoCapsule` | diagonal / arbitrarily rotated capsule axis | [x] |
| 61 | `c2RaytoCapsule` | `b` "below" `a` (negative `yBb.y`, inverted slab) | [x] |
| 62 | `c2RaytoCapsule` | degenerate `a == b` (`c2Norm(0)` → `NaN` everywhere) | [x] |
| 63 | `c2RaytoCapsule` | `B.r == 0` and `B.r < 0` | [x] |
| 64 | `c2RaytoCapsule` | `yAe.x - yAp.x == 0` (unguarded divide) | [x] |
| 65 | `c2RaytoCapsule` | `A.t == 0`, `A.t == inf`, `A.d == (0,0)` | [x] |
| 66 | `c2RaytoCapsule` | `NaN` / `inf` in `A.p`, `A.d`, `B.a`, `B.b`, `B.r` | [x] |

### Group 6 — `c2RaytoPoly` (`tests/raycast_poly.rs`)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 67 | `c2RaytoPoly` | random ray × random convex poly, `bx = NULL`, shotgun (8192 cases) | [x] |
| 68 | `c2RaytoPoly` | `count = 0` (empty) | [x] |
| 69 | `c2RaytoPoly` | `count = 1` (single half-plane) | [x] |
| 70 | `c2RaytoPoly` | `count = 2` (slab) | [x] |
| 71 | `c2RaytoPoly` | `count = 3` (triangle) | [x] |
| 72 | `c2RaytoPoly` | `count = 4` (box) | [x] |
| 73 | `c2RaytoPoly` | `count = 5,6,7,8` (regular n-gons) | [x] |
| 74 | `c2RaytoPoly` | `count < 0` | [x] |
| 75 | `c2RaytoPoly` | `count > 8` with a padded, fully-initialised backing buffer (OOB read) | [x] |
| 76 | `c2RaytoPoly` | `bx = NULL` vs an explicit `c2xIdentity()` — must agree | [x] |
| 77 | `c2RaytoPoly` | `bx` = pure translation | [x] |
| 78 | `c2RaytoPoly` | `bx` = pure rotation (angle sweep) | [x] |
| 79 | `c2RaytoPoly` | `bx` = rotation + translation | [x] |
| 80 | `c2RaytoPoly` | `bx.r` non-unit (`c*c+s*s != 1`), zero `c2r` | [x] |
| 81 | `c2RaytoPoly` | ray origin inside the poly (`index` stays `-1` → `0`) | [x] |
| 82 | `c2RaytoPoly` | ray parallel to a face, outside (`den==0 && num<0`) | [x] |
| 83 | `c2RaytoPoly` | ray parallel to a face, inside (`den==0 && num>=0`) | [x] |
| 84 | `c2RaytoPoly` | ray hitting each face in turn (all `index` values 0..count-1) | [x] |
| 85 | `c2RaytoPoly` | `A.t == 0` / `inf` / negative | [x] |
| 86 | `c2RaytoPoly` | `A.d == (0,0)` (every `den == 0`) | [x] |
| 87 | `c2RaytoPoly` | zero / `NaN` / `inf` normals and verts | [x] |
| 88 | `c2RaytoPoly` | grazing: ray exactly along a face plane | [x] |
| 89 | `c2RaytoPoly` | `out` sentinel preserved on each miss path | [x] |

### Group 7 — `c2CastRay` dispatch (`tests/cast_ray.rs`)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 90 | `c2CastRay` | `typeB = C2_TYPE_CIRCLE`, randomized; result must equal direct `c2RaytoCircle` | [x] |
| 91 | `c2CastRay` | `typeB = C2_TYPE_AABB`, randomized; equals `c2RaytoAABB` | [x] |
| 92 | `c2CastRay` | `typeB = C2_TYPE_CAPSULE`, randomized; equals `c2RaytoCapsule` | [x] |
| 93 | `c2CastRay` | `typeB = C2_TYPE_POLY`, `bx = NULL`, randomized; equals `c2RaytoPoly` | [x] |
| 94 | `c2CastRay` | `typeB = C2_TYPE_POLY`, `bx != NULL` (rotation+translation sweep) | [x] |
| 95 | `c2CastRay` | `typeB` non-POLY **with** a non-NULL `bx` (must be ignored) | [x] |
| 96 | `c2CastRay` | `typeB` out of range: `4`, `5`, `255`, `-1`, `i32::MIN`, `i32::MAX` | [x] |
| 97 | `c2CastRay` | same byte buffer reinterpreted under each of the 4 `typeB` values | [x] |

### Group 8 — public header entry point (`tests/poly_ray.rs`)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 98 | `poly_ray` | both `out` pointers valid, buffers pre-poisoned with 3 different sentinels | [x] |
| 99 | `poly_ray` | repeated calls (idempotence / no hidden global state) | [x] |
| 100 | `poly_ray` | return value bitmask + both `c2Raycast` payloads compared bit-for-bit | [x] |

## Row -> test mapping

Every row above is covered by a `#[test]` whose name encodes the row number(s).
139 tests total, all passing in every configuration.

| CONFIGS.md rows | test file | tests |
|-----------------|-----------|-------|
| 1-15   | `tests/vec_math.rs`        | 12 |
| 16-24  | `tests/overlap.rs`         | 10 |
| 25-34  | `tests/raycast_circle.rs`  | 10 |
| 35-48  | `tests/raycast_aabb.rs`    | 12 |
| 49-66  | `tests/raycast_capsule.rs` | 13 |
| 67-89  | `tests/raycast_poly.rs`    | 13 |
| 90-97  | `tests/cast_ray.rs`        |  7 |
| 98-100 | `tests/poly_ray.rs`        |  4 |
| (Phase C, `ERRORS.md`) | `tests/errors.rs`   | 53 |
| (Phase D, symbol parity) | `tests/symbols.rs` | 3 |
| (comparator justification) | `tests/nan_payload.rs` | 2 |

## Methodology

* **Both** libraries are driven exclusively through `dlopen` + `dlsym` — the
  Rust `cdylib`'s `#[no_mangle]` export wrappers and its C ABI are therefore
  part of what is under test. No Rust function is ever called directly.
* Every raycast row calls the entry point **three times per input**, each time
  with a different poison bit pattern pre-loaded into the `c2Raycast`
  out-parameter, and compares the resulting 12 bytes. "Did the callee write to
  `*out`?" is thus part of the differential comparison for every single input,
  which is what catches the `c2RaytoCapsule` unconditional pre-write and the
  untouched-on-miss behaviour of the other three raycasts.
* Rows are driven by a fixed-seed SplitMix64 PRNG (256-8192 cases per row) with
  two generators: `geom()` produces geometry-friendly values including exact
  integers/halves so that `<=` / `>=` boundaries are actually landed on, and
  `wild()` produces the full float spectrum (`±0.0`, subnormals, `±FLT_MAX`,
  `±inf`, `NaN`, and completely random bit patterns).
* The **low-level** entry points are exercised directly, and separately checked
  for agreement with the composed `c2CastRay` dispatcher and the `poly_ray`
  one-shot wrapper (rows 90-97), so a bug in the pipeline cannot hide behind a
  per-function test.
* `c2RaytoPoly` with `count > 8` (rows 41/75) is tested through a 4-byte-aligned,
  fully-initialised backing buffer larger than `sizeof(c2Poly)`, so both
  libraries perform the same out-of-bounds reads over the same bytes. To make
  this reproducible the Rust translation derives its `verts`/`norms` cursors from
  the `c2Poly*` itself via `offset_of!` byte arithmetic rather than from the
  array fields, preserving the caller's pointer provenance.

## Reproducing

```sh
./run_all.sh          # C build + every feature combo x {dev, release}
cargo test            # default features
cargo test --no-default-features

# extra: re-run the whole suite against an optimised C build
cmake -S c_src -B /tmp/cO2 -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DCMAKE_C_FLAGS=-O2
cmake --build /tmp/cO2
DIFFTEST_C_SO=/tmp/cO2/libtranslated_rust.so cargo test
```

## Result

| configuration | cargo check | symbol diff | tests |
|---------------|-------------|-------------|-------|
| features `[]`, profile `dev`     | clean (0 warnings) | empty (28/28) | 139 passed, 0 failed |
| features `[]`, profile `release` | clean (0 warnings) | empty (28/28) | 139 passed, 0 failed |
| C at `-O0` / `-O1` / `-O2` / `-O3` | — | — | 139 passed, 0 failed each |
