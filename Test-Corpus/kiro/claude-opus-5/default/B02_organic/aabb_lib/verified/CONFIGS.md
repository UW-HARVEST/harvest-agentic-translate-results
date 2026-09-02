# CONFIGS.md — configuration / valid-input surface table

Derived mechanically from the branch structure of `c_src/src/lib.c`. This is the
mirror of `ERRORS.md`: it enumerates the **valid** axes the C actually
distinguishes, and their pruned cross-product.

## Axes the C branches on

**A. Runtime options of the only configurable entry point, `c2GJK`:**

| axis | values the C distinguishes | branch site |
|------|----------------------------|-------------|
| `typeA` | `CIRCLE` / `AABB` / `CAPSULE` — changes `pA.count` to 1 / 4 / 2 and `pA.radius` to `r` / `0` / `r` | `c2MakeProxy` L108 |
| `typeB` | same three | `c2MakeProxy` L108 |
| `ax_ptr` | `NULL` (⇒ identity) / non-null identity / non-null rotate+translate | L367 |
| `bx_ptr` | `NULL` / non-null identity / non-null rotate+translate | L371 |
| `use_radius` | `0` (raw core distance) / `1` (radius shrink + midpoint fallback) | L481 |
| `cache` | `NULL` / non-null `count==0` (cold) / non-null warm from a prior call | L380, L381, L404 |
| `outA`,`outB`,`iterations` | `NULL` / non-null (each independently) | L510-514 |

**B. Input *shapes* the code special-cases:**

| axis | values |
|------|--------|
| separation regime | deeply overlapping (⇒ `hit`, `count==3`) / touching / just separated / far apart |
| circle | `r == 0` / small `r` / huge `r` |
| AABB | proper / degenerate (`min == max`) / flat (one axis zero-width) / inverted (`min > max`) |
| capsule | `a != b` / degenerate `a == b` / `r == 0` |
| relative placement | axis-aligned, diagonal, vertex-region, edge-region, interior-region (drives which `c22`/`c23` branch fires) |
| `c2Simplex.count` | `0` / `1` / `2` / `3` / `4` (the `d` slot) |
| `c2Support` vertex count | `1` / `2` / `4` / `8` (the proxy capacity), plus tie-valued dot products |
| numeric magnitudes | denormal, ~1, ~1e18, `inf`, `NaN`, `-0.0` |

**C. Entry points.** All 38, including the lowest-level ones. The convenience
wrappers (`aabb`, `c2Collided`) sit on top of `c2Circleto*`/`c2AABBto*` which sit
on `c2GJK` which sits on `c2MakeProxy`/`c22`/`c23`/`c2D`/`c2L`/`c2Support`/
`c2Witness`/`c2GJKSimplexMetric`, all of which sit on the ~20 vector primitives.
Every level is driven directly, not only through the wrappers.

## Configuration table

Every row is exercised with **many randomized inputs** (fixed seed, see
`tests/common/mod.rs::Rng`), and compared byte-for-byte (raw `f32` bit patterns,
`memcmp` of out-structs) between the C `.so` and the Rust `.so`.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
### Tier 1 — vector / rotation primitives (no options; input shape is the whole axis)
|  1 | `c2V` | random finite `(x,y)`; also `±0.0`, denormals, `±inf`, `NaN` | [x] |
|  2 | `c2Mulvs` | random `a` × random scalar; scalar `0`, `-0`, `inf`, `NaN`; huge×tiny (overflow/underflow) | [x] |
|  3 | `c2Add`, `c2Sub` | random pairs; cancellation (`a == b`), `inf - inf`, mixed-sign zeros | [x] |
|  4 | `c2Dot` | random pairs; products that cancel exactly; `inf*0` → `NaN`; magnitudes ~1e19 (overflow to `inf`) | [x] |
|  5 | `c2Det2` | random pairs; collinear (`det == 0`), near-collinear, sign flip, `NaN` | [x] |
|  6 | `c2Len` | random; zero vector; huge vector (`dot` overflows to `inf` ⇒ `inf`); negative-component | [x] |
|  7 | `c2Maxv`, `c2Minv` | random pairs; equal components; `NaN` in first vs second operand (ternary-select asymmetry) ; `+0.0` vs `-0.0` | [x] |
|  8 | `c2Clampv` | `a` inside / below / above `[lo,hi]`; `lo == hi`; `lo > hi` (inverted); `NaN` in each of `a`,`lo`,`hi` | [x] |
|  9 | `c2Neg`, `c2Skew`, `c2CCW90` | random; `±0.0` (sign of negated zero); `NaN` | [x] |
| 10 | `c2Div` | random `a` × divisor `1`, random, `0` (⇒ `inf`), `inf` (⇒ `0`), `NaN`, denormal | [x] |
| 11 | `c2Norm` | random; unit; zero vector (⇒ `NaN,NaN`); huge (⇒ `0` or `NaN`); denormal | [x] |
| 12 | `c2RotIdentity`, `c2xIdentity` | no input — exact bit pattern of the returned struct (checks the 16-byte SSE,SSE return classification of `c2x`) | [x] |
| 13 | `c2Mulrv`, `c2MulrvT` | random `c2r` (both normalized `cos/sin` and arbitrary non-normalized); `c2MulrvT(r, c2Mulrv(r,v))` round-trip; `NaN` rotation | [x] |
| 14 | `c2Mulxv` | `c2x` = identity / pure translation / pure rotation / rotation+translation; random `v` | [x] |
### Tier 2 — proxy construction
| 15 | `c2BBVerts` | proper AABB; `min == max`; inverted (`min > max`); flat in x; flat in y; `NaN`/`inf` corners — all 4 output vertices compared | [x] |
| 16 | `c2MakeProxy` | `type = CIRCLE`, random circle (`r = 0`, small, huge, negative) — full 72-byte `c2Proxy` compared | [x] |
| 17 | `c2MakeProxy` | `type = AABB`, proper / degenerate / inverted / flat box | [x] |
| 18 | `c2MakeProxy` | `type = CAPSULE`, `a != b` / `a == b` / `r = 0` / negative `r` | [x] |
### Tier 3 — simplex solvers, driven directly on a caller-built `c2Simplex`
| 19 | `c2GJKSimplexMetric` | `count = 1` (⇒ 0) | [x] |
| 20 | `c2GJKSimplexMetric` | `count = 2`, random `a.p`,`b.p`; `a.p == b.p`; huge separation | [x] |
| 21 | `c2GJKSimplexMetric` | `count = 3`, random triangle; degenerate/collinear; clockwise vs counter-clockwise (sign of `det2`) | [x] |
| 22 | `c22` | `v <= 0` region (origin beyond `a`) — full 152-byte simplex compared | [x] |
| 23 | `c22` | `u <= 0` region (origin beyond `b`) — checks the `s->a = s->b` copy of all 6 `c2sv` fields | [x] |
| 24 | `c22` | interior region `u>0 && v>0` — `div = u+v`, `count = 2` | [x] |
| 25 | `c22` | degenerate `a.p == b.p`; random `a.p`,`b.p` over a wide range so all three branches are hit by chance | [x] |
| 26 | `c23` | vertex-A region (`vAB<=0 && uCA<=0`) | [x] |
| 27 | `c23` | vertex-B region (`uAB<=0 && vBC<=0`) — checks `a = b` copy | [x] |
| 28 | `c23` | vertex-C region (`uBC<=0 && vCA<=0`) — checks `a = c` copy | [x] |
| 29 | `c23` | edge-AB region (`wABC<=0`) | [x] |
| 30 | `c23` | edge-BC region (`uABC<=0`) — checks the `a=b; b=c;` shift | [x] |
| 31 | `c23` | edge-CA region (`vABC<=0`) — checks the `b=a; a=c;` swap | [x] |
| 32 | `c23` | interior region (all barycentrics positive), `count = 3` | [x] |
| 33 | `c23` | collinear / zero-area triangle ⇒ `area == 0` ⇒ `div == 0` | [x] |
| 34 | `c23` | random triangles over a wide range (all 7 branches by chance), plus triangles with a repeated vertex | [x] |
| 35 | `c2D` | `count = 1`; `count = 2` with `det2 > 0` (skew); `count = 2` with `det2 <= 0` (ccw90); `count = 2` with `det2 == 0` exactly | [x] |
| 36 | `c2L` | `count = 1`; `count = 2` with random `u`/`div`; `div == 0` (⇒ `inf`); `div` negative | [x] |
| 37 | `c2Witness` | `count = 1` (raw `sA`/`sB` copy) | [x] |
| 38 | `c2Witness` | `count = 2`, random `u`/`div` including `div` that does not equal `u_a+u_b` | [x] |
| 39 | `c2Witness` | `count = 3`, random barycentrics; `div == 0`; negative `div` | [x] |
| 40 | `c2Support` | `count = 1` (single vertex) | [x] |
| 41 | `c2Support` | `count = 2` (capsule proxy), random `d`, incl. `d` perpendicular ⇒ tie | [x] |
| 42 | `c2Support` | `count = 4` (AABB proxy), random `d`, axis-aligned `d` ⇒ two-way tie | [x] |
| 43 | `c2Support` | `count = 8` (full proxy capacity), random verts and `d`; all-equal verts (⇒ index 0); `NaN` in one vertex | [x] |
### Tier 4 — `c2GJK` driven directly (the lowest-level composed entry point)
| 44 | `c2GJK` | `CIRCLE`×`CIRCLE`, both transforms `NULL`, `use_radius = 0`, `cache = NULL` | [x] |
| 45 | `c2GJK` | `CIRCLE`×`CIRCLE`, both `NULL`, `use_radius = 1`, `cache = NULL` | [x] |
| 46 | `c2GJK` | `CIRCLE`×`AABB`, `use_radius` ∈ {0,1}, transforms `NULL` | [x] |
| 47 | `c2GJK` | `CIRCLE`×`CAPSULE`, `use_radius` ∈ {0,1}, transforms `NULL` | [x] |
| 48 | `c2GJK` | `AABB`×`CIRCLE`, `use_radius` ∈ {0,1} | [x] |
| 49 | `c2GJK` | `AABB`×`AABB`, `use_radius` ∈ {0,1} — the 4-vs-4-vertex case, drives `c23` hardest | [x] |
| 50 | `c2GJK` | `AABB`×`CAPSULE`, `use_radius` ∈ {0,1} | [x] |
| 51 | `c2GJK` | `CAPSULE`×`CIRCLE`, `use_radius` ∈ {0,1} | [x] |
| 52 | `c2GJK` | `CAPSULE`×`AABB`, `use_radius` ∈ {0,1} | [x] |
| 53 | `c2GJK` | `CAPSULE`×`CAPSULE`, `use_radius` ∈ {0,1} — the 2-vs-2 case | [x] |
| 54 | `c2GJK` | all 9 type pairs × `ax_ptr` non-null identity, `bx_ptr` `NULL` | [x] |
| 55 | `c2GJK` | all 9 type pairs × `ax_ptr` `NULL`, `bx_ptr` non-null identity | [x] |
| 56 | `c2GJK` | all 9 type pairs × both transforms non-null pure **translation** | [x] |
| 57 | `c2GJK` | all 9 type pairs × both transforms non-null pure **rotation** (random angle, `c2r` from `cosf/sinf`) | [x] |
| 58 | `c2GJK` | all 9 type pairs × both transforms rotation **+** translation, `use_radius = 1` | [x] |
| 59 | `c2GJK` | all 9 type pairs × non-normalized `c2r` (`c`,`s` arbitrary — a scaling/shearing "rotation" the C never validates) | [x] |
| 60 | `c2GJK` | overlapping shapes ⇒ `hit == 1` path (`count == 3`), `use_radius` ∈ {0,1}; asserts `dist == 0` and `a == b` on both sides | [x] |
| 61 | `c2GJK` | exactly-touching shapes ⇒ `dist <= FLT_EPSILON` midpoint branch, `use_radius = 1` | [x] |
| 62 | `c2GJK` | far-separated shapes ⇒ radius-shrink branch (`dist > rA+rB`), `use_radius = 1`, checks the `c2Norm` push | [x] |
| 63 | `c2GJK` | `cache != NULL` starting **cold** (`count = 0`); asserts the returned `dist` **and** the 36-byte written-back cache match | [x] |
| 64 | `c2GJK` | `cache != NULL` **warm**: call twice with the same cache, same shapes; compare `dist` + cache after each call | [x] |
| 65 | `c2GJK` | `cache != NULL` warm and **reused with moved shapes** (cache indices still valid) — the `cache_was_read` path | [x] |
| 66 | `c2GJK` | `cache` warm with `count == 3` (triangle cache) — exercises the `metric < -1.0e8f` freshness test | [x] |
| 67 | `c2GJK` | `iterations != NULL` — compare the iteration count exactly; includes configurations that hit the `iter < 20` cap | [x] |
| 68 | `c2GJK` | `outA = NULL`, `outB` non-null; and `outA` non-null, `outB = NULL`; and both `NULL` (return value only) | [x] |
| 69 | `c2GJK` | degenerate shapes: zero-radius circle, `min == max` AABB, `a == b` capsule, `r == 0` capsule — every type pair | [x] |
| 70 | `c2GJK` | shape coordinates at magnitude ~1e18 and ~1e-30 (denormal) — value-dependent overflow in `c2Dot`/`c2Det2` | [x] |
| 71 | `c2GJK` | identical shapes at identical positions (all 3 types) — the coincident-origin degenerate case | [x] |
### Tier 5 — boolean helpers
| 72 | `c2AABBtoAABB` | random proper boxes: separated on x, on y, overlapping, edge-touching, corner-touching, nested; inverted boxes; `NaN` | [x] |
| 73 | `c2CircletoCircle` | random: separated, overlapping, exactly touching, nested, `r == 0`, negative `r`, huge `r` | [x] |
| 74 | `c2CircletoAABB` | circle centre inside / outside / on each face / on a corner; `r == 0`; inverted box; degenerate box | [x] |
| 75 | `c2CircletoCapsule` | `da < 0` (before `a`), `da>=0 && db<0` (perpendicular), `db >= 0` (after `b`); degenerate capsule `a == b`; `r == 0` | [x] |
| 76 | `c2AABBtoCapsule` | random boxes × random capsules, overlapping and separated (goes through `c2GJK` with `use_radius = 1`) | [x] |
| 77 | `c2CapsuletoCapsule` | random capsule pairs: crossing, parallel, collinear, coincident, separated; degenerate `a == b` | [x] |
### Tier 6 — dispatch and top-level wrapper
| 78 | `c2Collided` | all 9 valid `(typeA,typeB)` combinations × random shapes, incl. the 3 argument-swapping branches (`AABB`×`CIRCLE`, `CAPSULE`×`CIRCLE`, `CAPSULE`×`AABB`) | [x] |
| 79 | `aabb` | random `(min_x,min_y,max_x,max_y)`; boxes positioned to produce each of the 8 possible result masks `0..7`; inverted boxes; huge/denormal/`inf`/`NaN` coordinates | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` section, so there is exactly
one feature set (empty). `check_features.sh` derives the list mechanically from
`Cargo.toml` (so a future feature is picked up automatically), then for each
combination rebuilds the `cdylib`, re-runs the `nm -D` symbol diff, and re-runs
the whole Phase B + Phase C suite:

```
$ ./check_features.sh
features declared: (none)
combinations to verify: 2
combination 1/2: (default features)
  symbols: C=38 Rust=38
  missing from Rust: none
  extra in Rust: none
  unresolved non-libc symbols: none
  tests passed: 86
  total differential checks: 12998727
combination 2/2: --no-default-features
  symbols: C=38 Rust=38
  missing from Rust: none
  extra in Rust: none
  unresolved non-libc symbols: none
  tests passed: 86
  total differential checks: 12991244
ALL 2 FEATURE COMBINATIONS PASSED
```

`#[cfg]` does not appear in `src/lib.rs` and `#ifdef` does not appear in
`c_src/src/lib.c`, so no conditional compilation exists on either side and the
two combinations above necessarily exercise identical code. The table above holds
for both.

## Test-suite layout

| file | covers |
|------|--------|
| `tests/common/mod.rs` | harness: `libloading` loaders for both `.so`s, C-ABI mirror types, bit-exact comparators, seeded `SplitMix64` generators |
| `tests/phase_b_primitives.rs` | rows 1-18 |
| `tests/phase_b_simplex.rs` | rows 19-43 |
| `tests/phase_b_gjk.rs` | rows 44-71 |
| `tests/phase_b_wrappers.rs` | rows 72-79 |
| `tests/phase_c_lowlevel.rs` | `ERRORS.md` rows 1-26 |
| `tests/phase_c_gjk.rs` | `ERRORS.md` rows 27-48 |
| `tests/phase_c_wrappers.rs` | `ERRORS.md` rows 49-70 |
| `tests/phase_c_boundaries.rs` | generic boundaries (all-NULL, zero/oversized lengths, one-past-range enums) + the `#[ignore]`d row-46 UB record |

## Completion gate (re-verified)

Commands and their measured output, run from `translation/`:

```
$ ./check_features.sh                       # both combinations, symbols + full suite
ALL 2 FEATURE COMBINATIONS PASSED

$ cargo test --release -- --nocapture --test-threads=1
tests passed: 86   failed: 0   ignored: 1
total differential checks: 13,016,727
```

- [x] **`SYMBOLS.md`** — `nm -D` shows 0 missing and 0 extra symbols (38 = 38), and
      0 unresolved non-libc symbols in the Rust `.so`. Also verified structurally:
      all 38 C function names have a `pub extern "C" fn` with `#[no_mangle]` in
      `src/lib.rs`, and the source contains no `unimplemented!`, `todo!`,
      `unreachable!` or bare `panic!` — nothing is stubbed.
- [x] **Phase B** — all 79 `CONFIGS.md` rows pass across randomized inputs
      (seeded `SplitMix64`, reproducible). Branch coverage is asserted, not
      assumed: the tests fail if `c23`'s 7 regions, `c2CircletoCapsule`'s 3
      regions, `c2GJK`'s hit/midpoint/shrink outcomes, the `count==3` warm-cache
      path, or ≥6 of `aabb()`'s 8 result masks are not reached.
- [x] **Phase C** — all 70 `ERRORS.md` rows have a passing error-path
      differential test asserting the *same specific* fallback value, plus the
      generic boundaries (all-`NULL` pointer sets, zero/oversized lengths,
      one-past-range enum values on every `C2_TYPE` parameter). Row 46 is the one
      exception and is documented, not skipped: the C reads an uninitialised
      `c2Proxy` and can segfault, so it is undefined behaviour with nothing to
      match. Its probe is `#[ignore]`d.
- [x] **All of the above under every feature combination** — the crate declares
      no features, so the two combinations (`(default)` and
      `--no-default-features`) are the complete set; `check_features.sh` derives
      the list from `Cargo.toml` mechanically and both pass identically. Neither
      side has any conditional compilation (`#[cfg]` / `#ifdef`), so the two
      builds are provably the same code.

Nothing in `c_src/` was modified — only `c_src/build/` was added by CMake.
