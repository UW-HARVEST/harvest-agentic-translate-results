# CONFIGS.md — Phase B configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs.  Axes derived mechanically from the
branches `c_src/src/lib.c` actually takes.

## Build-time axes

| axis | values | source |
|------|--------|--------|
| cargo features | *none declared* → 1 combination (default == `--no-default-features` == `--all-features`) | `Cargo.toml` has no `[features]` |
| C preprocessor | *none* | `grep '#if\|#ifdef\|#ifndef' c_src/**` is empty; `CMakeLists.txt` has no `option()` / `target_compile_definitions` |
| Rust profile | `dev` and `release` (both built and both run against the same C `.so`) | `run_diff_tests.sh` |

## Runtime axes

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `A1` shape type of A | `C2_TYPE_CAPSULE`(0), `C2_TYPE_CIRCLE`(1), `C2_TYPE_AABB`(2) | `c2MakeProxy` L109, `c2Collided` L572 |
| `A2` shape type of B | same 3 | `c2Collided` L574/586/598 |
| `A3` `use_radius` | `0`, non-zero | `c2GJK` L477 |
| `A4` `ax_ptr` / `bx_ptr` | `NULL`, identity `c2x`, pure translation, pure rotation, rotation+translation, non-unit `c2r` | `c2GJK` L363/367 |
| `A5` `cache` | `NULL`, zeroed (`count==0`), warm (`count`∈{1,2,3} from a previous call), warm-but-rejected by the L400 predicate | `c2GJK` L378/400/495 |
| `A6` out params | `outA`/`outB`/`iterations` each `NULL` or non-`NULL` (8 combinations, plus all-`NULL`) | `c2GJK` L505/507/509 |
| `A7` relative placement | deeply overlapping (⇒ `hit`, simplex count 3), touching, just separated, far separated, identical shapes, concentric | drives L436 / L442 / L446 / L466 / L480 |
| `A8` proxy vertex count | 1 (circle), 2 (capsule), 4 (AABB) — i.e. `c2Support` loop trip counts 0/1/3 | `c2MakeProxy`, `c2Support` L296 |
| `A9` radius magnitude | `0`, tiny (`< FLT_EPSILON`), normal, huge | `c2GJK` L480 |
| `A10` degenerate shapes | zero-size AABB (`min==max`), degenerate capsule (`a==b`), zero-radius circle, axis-aligned collinear capsules | `c22`/`c23` branch selection |
| `A11` value magnitude | tiny (`1e-30`), normal (`±100`), huge (`1e30`, near `FLT_MAX`), exact powers of two, mixed signs, `±0.0` | float branch predicates throughout |
| `A12` simplex `count` (direct low-level calls) | `1`, `2`, `3` | `c22`, `c23`, `c2D`, `c2L`, `c2Witness`, `c2GJKSimplexMetric` |
| `A13` `c22`/`c23` branch taken | `c22`: 3 branches; `c23`: 7 branches | L186/190/195 and L217/221/226/231/236/243/250 |

## Entry points (all 39 exported symbols are covered)

Tier 0 (pure scalar/vector): `c2V` `c2Mulvs` `c2Maxv` `c2Minv` `c2Clampv`
`c2Sub` `c2Add` `c2Dot` `c2Det2` `c2Len` `c2Neg` `c2Skew` `c2CCW90` `c2Div`
`c2Norm` `c2RotIdentity` `c2xIdentity` `c2Mulrv` `c2MulrvT` `c2Mulxv`

Tier 1 (struct/pointer): `c2BBVerts` `c2MakeProxy` `c2Support`

Tier 2 (simplex): `c2GJKSimplexMetric` `c22` `c23` `c2D` `c2L` `c2Witness`

Tier 3 (GJK): `c2GJK`

Tier 4 (boolean shape tests): `c2AABBtoAABB` `c2AABBtoCapsule`
`c2CapsuletoCapsule` `c2CircletoCircle` `c2CircletoAABB` `c2CircletoCapsule`

Tier 5 (public/convenience): `c2Collided` `ptr_from_parts` `omni_collide`

## Table

Every row is driven with **many randomised inputs** (`SplitMix64`, fixed seed
`0x9E3779B97F4A7C15`), not one hand-picked value, and compared bit-for-bit
(`to_bits()` on every returned/written `f32`).

| #  | entry point(s) | configuration (options set + input shape) | ✔ |
|----|----------------|-------------------------------------------|---|
|  1 | `c2V` | random `(x,y)` from the full float grid incl. `±0`, `±inf`, `NaN`, subnormals | [x] |
|  2 | `c2Mulvs` | random `c2v` × random scalar; scalar ∈ {0, ±0, 1, -1, huge, tiny, inf, NaN} | [x] |
|  3 | `c2Maxv` / `c2Minv` | both-normal, one-`NaN`, both-`NaN`, `±0` pairs, equal operands | [x] |
|  4 | `c2Clampv` | `lo<hi` (valid), `lo==hi`, `a` below / inside / above the box, `NaN` in each of the 3 args | [x] |
|  5 | `c2Sub` / `c2Add` | random pairs, cancellation (`a-a`), overflow to `±inf`, `NaN` operands | [x] |
|  6 | `c2Dot` | random pairs; catastrophic cancellation; `inf*0` → `NaN`; both-`NaN` (operand-order/NaN-payload sensitive) | [x] |
|  7 | `c2Det2` | random pairs; collinear (`det==0`); `NaN`; overflow | [x] |
|  8 | `c2Len` | random `c2v`; zero vector; overflow to `inf`; `NaN`; subnormal | [x] |
|  9 | `c2Neg` / `c2Skew` / `c2CCW90` | random `c2v` incl. `±0` (sign of negative zero), `NaN` | [x] |
| 10 | `c2Div` | random `c2v` × random divisor incl. `1`, huge, tiny, subnormal (`1/x` rounding is observable) | [x] |
| 11 | `c2Norm` | random `c2v`; unit vectors; huge (overflow in `c2Len`); tiny (underflow) | [x] |
| 12 | `c2RotIdentity` / `c2xIdentity` | no inputs — exact bit compare of the returned aggregates (`c2r` 8B / `c2x` 16B, 2-register SSE return) | [x] |
| 13 | `c2Mulrv` / `c2MulrvT` | `c2r` = identity, real `(cos θ, sin θ)` for random θ, **non-unit** `c2r`, `c2r` with `NaN`/`inf`; random `c2v` | [x] |
| 14 | `c2Mulxv` | `c2x` = identity / pure translation / pure rotation / rotation+translation / non-unit rotation; random `c2v` | [x] |
| 15 | `c2BBVerts` | valid AABB, `min==max` (zero size), inverted (`min>max`), huge, `NaB`; all 4 written verts compared | [x] |
| 16 | `c2MakeProxy` | `type=CIRCLE` (⇒ `count=1`, `radius=r`) — whole 72-byte `c2Proxy` compared | [x] |
| 17 | `c2MakeProxy` | `type=AABB` (⇒ `count=4`, `radius=0`, 4 verts via `c2BBVerts`) — whole `c2Proxy` compared | [x] |
| 18 | `c2MakeProxy` | `type=CAPSULE` (⇒ `count=2`, `radius=r`) — whole `c2Proxy` compared | [x] |
| 19 | `c2MakeProxy` | each of the 3 types written **over** a pre-filled proxy (verifies the untouched `verts[n..8]` tail is left alone) | [x] |
| 20 | `c2Support` | `count=1` (circle proxy), random `d` | [x] |
| 21 | `c2Support` | `count=2` (capsule proxy), random `d`, incl. `d` ⟂ to the segment (tie ⇒ index 0) | [x] |
| 22 | `c2Support` | `count=4` (AABB proxy), random `d`, incl. the 4 axis directions and the 4 diagonal ties | [x] |
| 23 | `c2Support` | `count=8` (full `verts[8]`), random verts and `d` | [x] |
| 24 | `c2GJKSimplexMetric` | `count=1` → `0`; `count=2` → `c2Len`; `count=3` → `c2Det2`; randomised simplex points | [x] |
| 25 | `c22` | randomised `a.p`/`b.p` hitting branch `v<=0` — whole 152-byte `c2Simplex` compared | [x] |
| 26 | `c22` | randomised, branch `u<=0` — whole `c2Simplex` compared | [x] |
| 27 | `c22` | randomised, branch `u>0 && v>0` — whole `c2Simplex` compared | [x] |
| 28 | `c22` | fully random simplexes (all three branches hit statistically), `sA`/`sB`/`u`/`iA`/`iB` also randomised so the field copies are observable | [x] |
| 29 | `c23` | randomised simplexes, branch 1 (`vAB<=0 && uCA<=0`) | [x] |
| 30 | `c23` | branch 2 (`uAB<=0 && vBC<=0`) — checks the `a=b` copy | [x] |
| 31 | `c23` | branch 3 (`uBC<=0 && vCA<=0`) — checks the `a=c` copy | [x] |
| 32 | `c23` | branch 4 (`wABC<=0`) | [x] |
| 33 | `c23` | branch 5 (`uABC<=0`) — checks the `a=b; b=c` shift | [x] |
| 34 | `c23` | branch 6 (`vABC<=0`) — checks the `b=a; a=c` shift | [x] |
| 35 | `c23` | fall-through `else` (`count=3`), incl. `div = uABC+vABC+wABC` summation order | [x] |
| 36 | `c23` | fully random simplexes (all 7 branches hit statistically) | [x] |
| 37 | `c2D` | `count=1` (`-a.p`), `count=2` with `c2Det2>0` (`c2Skew`), `count=2` with `c2Det2<=0` (`c2CCW90`), `count=3` | [x] |
| 38 | `c2L` | `count=1`, `count=2` with random `u`/`div` (barycentric blend) | [x] |
| 39 | `c2Witness` | `count=1`, `2`, `3` with random `sA`/`sB`/`u`/`div` | [x] |
| 40 | `c2GJK` | CIRCLE×CIRCLE, `use_radius=1`, all transforms `NULL`, no cache, overlapping | [x] |
| 41 | `c2GJK` | CIRCLE×CIRCLE, `use_radius=1`, separated (radius-subtraction branch) | [x] |
| 42 | `c2GJK` | CIRCLE×CIRCLE, `use_radius=0`, separated (raw distance) | [x] |
| 43 | `c2GJK` | CIRCLE×AABB, `use_radius`∈{0,1}, overlapping / separated | [x] |
| 44 | `c2GJK` | CIRCLE×CAPSULE, `use_radius`∈{0,1}, overlapping / separated | [x] |
| 45 | `c2GJK` | AABB×AABB, `use_radius`∈{0,1}, overlapping / touching / separated | [x] |
| 46 | `c2GJK` | AABB×CAPSULE, `use_radius`∈{0,1} | [x] |
| 47 | `c2GJK` | AABB×CIRCLE (reversed order — asymmetric code path) | [x] |
| 48 | `c2GJK` | CAPSULE×CAPSULE, `use_radius`∈{0,1}, parallel / crossing / collinear / degenerate (`a==b`) | [x] |
| 49 | `c2GJK` | CAPSULE×CIRCLE and CAPSULE×AABB (reversed orders) | [x] |
| 50 | `c2GJK` | **all 9** `typeA`×`typeB` pairs × `use_radius`∈{0,1}, fully randomised shapes (wide value range) | [x] |
| 51 | `c2GJK` | `ax_ptr` = identity, `bx_ptr` = `NULL` | [x] |
| 52 | `c2GJK` | `ax_ptr` = pure translation, `bx_ptr` = pure translation | [x] |
| 53 | `c2GJK` | `ax_ptr` = pure rotation (`cos/sin` of random θ), `bx_ptr` = identity | [x] |
| 54 | `c2GJK` | both `ax_ptr` and `bx_ptr` = rotation + translation, random θ (exercises `c2Mulxv` + `c2MulrvT` inside the loop) | [x] |
| 55 | `c2GJK` | non-unit `c2r` in the transforms (scaling — the code never normalises) | [x] |
| 56 | `c2GJK` | `cache` = zeroed struct (cold), single call — cache **written** on exit and compared field-by-field | [x] |
| 57 | `c2GJK` | `cache` warm: same shapes called twice in a row, cache carried over; both calls' return values **and** the cache contents compared | [x] |
| 58 | `c2GJK` | `cache` warm across a *moved* shape (typical consumer loop: 8 sequential steps translating B, one persistent cache) | [x] |
| 59 | `c2GJK` | `cache` hand-crafted with `count`∈{1,2,3} and in-range `iA`/`iB`, random `metric`/`div` (hits both sides of the L400 predicate) | [x] |
| 60 | `c2GJK` | `iterations` non-`NULL` — the iteration count itself compared for every shape pair | [x] |
| 61 | `c2GJK` | all 8 `NULL`/non-`NULL` combinations of `outA`/`outB`/`iterations` | [x] |
| 62 | `c2GJK` | identical shapes at identical positions (fully-contained ⇒ `hit`) for all 3 types | [x] |
| 63 | `c2GJK` | zero-radius circles/capsules and zero-size AABBs (`min==max`) | [x] |
| 64 | `c2GJK` | radii below `FLT_EPSILON` and radii `>1e18` (both sides of the L480/L481 predicate) | [x] |
| 65 | `c2AABBtoAABB` | overlapping / touching (shared edge) / separated / nested / inverted / random | [x] |
| 66 | `c2CircletoCircle` | overlapping / exactly touching (`d == rA+rB`) / separated / concentric / zero radius / random | [x] |
| 67 | `c2CircletoAABB` | centre inside / centre outside near a face / near a corner / exactly on the boundary / zero radius / random | [x] |
| 68 | `c2CircletoCapsule` | all 3 `da`/`db` branches; degenerate capsule; zero radii; random | [x] |
| 69 | `c2AABBtoCapsule` | overlapping / separated / capsule crossing the box / degenerate capsule / random (goes through `c2GJK`) | [x] |
| 70 | `c2CapsuletoCapsule` | parallel / crossing / collinear / identical / degenerate / random (goes through `c2GJK`) | [x] |
| 71 | `c2Collided` | all 9 `typeA`×`typeB` pairs with correctly-typed shape pointers, randomised shapes | [x] |
| 72 | `ptr_from_parts` | each of the 3 valid types — the `malloc`'d struct's bytes are read back and compared (12 / 16 / 20 bytes) | [x] |
| 73 | `omni_collide` | all 9 `type_a`×`type_b` pairs, randomised `a1..a5` / `b1..b5` over a wide value range | [x] |
| 74 | `omni_collide` | all 9 pairs with clustered coordinates (high collision probability ⇒ exercises the `hit` paths) | [x] |
| 75 | `omni_collide` | all 9 pairs with `±0.0`, subnormal, `1e30`, `FLT_MAX`, `±inf`, `NaN` field values | [x] |
| 76 | `omni_collide` | all 9 pairs with integral coordinates on a small grid (exhaustive-ish sweep, boundary/touching cases) | [x] |
| 77 | `omni_collide` | all 9 pairs with negative radii | [x] |

## Test-file map

| `CONFIGS.md` rows | test file | tests |
|-------------------|-----------|-------|
| 1–14  | `tests/tier0_scalar.rs`  | 14 |
| 15–23 | `tests/tier1_struct.rs`  |  9 |
| 24–39 | `tests/tier2_simplex.rs` |  8 |
| 40–64 | `tests/tier3_gjk.rs`     | 12 |
| 65–71 | `tests/tier4_shapes.rs`  |  7 |
| 72–77 | `tests/tier5_public.rs`  |  6 |
| (`ERRORS.md` rows 1–94) | `tests/errors.rs` | 37 |
| harness self-check | `tests/harness_sanity.rs` | 5 |
| exploratory (`#[ignore]`) | `tests/search_iter_cap.rs` | 3 |

**98 non-ignored tests**, all run against 6 different builds of the Rust `.so`
by `./run_diff_tests.sh`.

## Branch-coverage evidence

The rows above are not just "called once"; the tests count which internal branch
each input reached and FAIL if any is missed.  Observed counts:

```
c22  branch hits = [8277, 4728, 6995]                                  (3/3)
c23  branch hits = [9222, 6780, 5080, 6418, 4708, 3260, 44532]         (7/7)
c2D  branch hits = [20000, 7021, 12979, 20000]                         (4/4)
c2CircletoCapsule da/db branches = [~1/3 each]                         (3/3)
c2GJK per type-pair: dist==0 ~3.3-4.6k / dist!=0 ~11.4-12.7k,
                     radius-sub ~4.4-5.7k / midpoint ~2.3-3.6k         (9/9 pairs)
cold-cache written counts by cache.count = [0, 1752, 1503, 745, 0]     (1,2,3 all seen)
cache predicate quadrants [min<max*2][metric<-1e8] = [[80,96],[5462,1874]]  (4/4)
```

Every `c2Collided` / `omni_collide` type pair is additionally asserted to
produce **both** a hit and a miss, so no row can pass by only ever taking one
side of the final comparison.

## Harness properties (why these rows are trustworthy)

* Both libraries are loaded with `libloading` (`dlopen`, `RTLD_LOCAL`) and
  called through real `extern "C"` function pointers — **no** Rust function is
  ever called directly, so the `#[no_mangle]` wrappers and the SysV struct ABI
  are part of what is tested.
* `tests/harness_sanity.rs` asserts the two `.so`s resolve to *different*
  addresses, that all 39 symbols resolve in both, and that `eq_raw`/`eq_f32`/
  `eq_int` actually panic on a difference (including `0.0` vs `-0.0` and two
  NaNs with different payloads) — so the suite cannot be vacuously green.
* Comparison is **bit-exact**: `f32` via `to_bits()`, aggregates via a raw byte
  compare of the whole struct (`c2Simplex` = 152 B, `c2Proxy` = 72 B,
  `c2GJKCache` = 36 B — none has padding, every field is 4-byte).
* Out-parameters are pre-filled with poison values, so "wrote too much" and
  "wrote too little" are detected as well as wrong values.
* Input buffers are compared after each call to prove neither library mutates
  its `const` arguments.
