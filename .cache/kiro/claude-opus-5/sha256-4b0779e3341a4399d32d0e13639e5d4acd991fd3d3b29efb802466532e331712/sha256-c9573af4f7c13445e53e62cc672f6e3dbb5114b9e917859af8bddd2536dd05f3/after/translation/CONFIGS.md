# CONFIGS.md — configuration surface table (valid inputs)

Axes derived mechanically from `c_src/src/lib.c`:

**Runtime options the public API can set**

| option | where | values the C branches on |
|--------|-------|--------------------------|
| `C2_TYPE typeA` | `c2MakeProxy` switch, `c2Collided` outer switch | `CIRCLE(0)`, `AABB(1)`, `CAPSULE(2)` |
| `C2_TYPE typeB` | `c2MakeProxy` switch, `c2Collided` inner switch | `CIRCLE(0)`, `AABB(1)`, `CAPSULE(2)` |
| `int use_radius` | `c2GJK` `else if (use_radius)` | `0`, `1` |
| `const c2x *ax_ptr` | `if (!ax_ptr)` | `NULL` (identity) vs. supplied transform |
| `const c2x *bx_ptr` | `if (!bx_ptr)` | `NULL` (identity) vs. supplied transform |
| `c2GJKCache *cache` | `if (cache)`, `cache->count`, metric guard | `NULL`, cold (`count == 0`), warm (`count ∈ {1,2,3}` valid), warm-rejected |
| `c2v *outA`, `c2v *outB`, `int *iterations` | `if (outA)` etc. | `NULL` vs. supplied |

**Input shapes the C special-cases**

| shape axis | values |
|------------|--------|
| proxy vertex count (from type) | 1 (circle), 2 (capsule), 4 (AABB) |
| proxy radius | `0` (AABB) vs. `r` (circle/capsule) |
| `c2Simplex.count` | 1, 2, 3 (+ out-of-range → ERRORS.md) |
| transform rotation | identity (`c=1,s=0`), pure rotation (`c²+s²=1`), non-unit `c2r` |
| transform translation | zero, non-zero |
| geometry relation | far apart / just touching / overlapping / coincident / one inside other |
| degeneracy | zero-radius, zero-extent AABB, zero-length capsule, collinear simplex |
| float classes | normal, denormal, `±0`, `±inf`, `NaN`, `±FLT_MAX`, `±FLT_MIN` |

**All 38 public entry points are covered, lowest level first** — the pure-vector
helpers (`c2V` … `c2MulrvT`), the struct builders (`c2BBVerts`, `c2MakeProxy`),
the simplex solvers (`c22`, `c23`, `c2D`, `c2L`, `c2Witness`,
`c2GJKSimplexMetric`, `c2Support`), the raw `c2GJK`, the six shape-pair
predicates, the `c2Collided` dispatcher, and the `reverse_collide` one-shot
wrapper. Every row is driven with **many randomized inputs (seeded xorshift128+,
fixed seed) plus the interesting hand-picked float classes**, and compared
**bit-for-bit** (`f32::to_bits`) between the C `.so` and the Rust `.so`.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `c2V` | random `(x, y)` from the mixed float pool (normal/denormal/±0/±inf/NaN/±FLT_MAX) | [x] |
| 2 | `c2Mulvs` | random `c2v` × random scalar, incl. `0`, `-0`, `inf`, `NaN` | [x] |
| 3 | `c2Add`, `c2Sub` | random `c2v` pairs, incl. cancellation (`a == b`), `inf - inf` | [x] |
| 4 | `c2Dot` | random `c2v` pairs; overflow to `inf`; `inf*0` → `NaN` | [x] |
| 5 | `c2Det2` | random `c2v` pairs; collinear (`det == 0`); overflow | [x] |
| 6 | `c2Maxv`, `c2Minv` | random pairs incl. `NaN` operands (C's `>`/`<` ternary picks `b`/`b` on NaN) and `+0` vs `-0` | [x] |
| 7 | `c2Clampv` | random `a` with `lo <= hi`; also inverted `lo > hi`; NaN in each of the three args | [x] |
| 8 | `c2Neg`, `c2Skew`, `c2CCW90` | random `c2v` incl. `±0` (sign of zero must match) and `NaN` | [x] |
| 9 | `c2Len` | random `c2v`; zero vector; `FLT_MAX` (overflow → `inf`); denormals (underflow → `0`); NaN | [x] |
| 10 | `c2Div` | random `c2v` × random divisor incl. `1`, `-1`, huge, denormal | [x] |
| 11 | `c2Norm` | random `c2v`; already-unit; huge; denormal (`len` underflows) | [x] |
| 12 | `c2RotIdentity`, `c2xIdentity` | no arguments — constant return, bit-compared | [x] |
| 13 | `c2Mulrv`, `c2MulrvT` | identity `c2r`; unit rotation `(cos θ, sin θ)` over many θ; non-unit / random `c2r`; NaN | [x] |
| 14 | `c2Mulxv` | `c2x` = identity; rotation-only; translation-only; both; random non-unit | [x] |
| 15 | `c2BBVerts` | random AABB (`min <= max`); zero-extent (`min == max`); inverted (`min > max`); `inf` bounds — all 4 output verts bit-compared | [x] |
| 16 | `c2MakeProxy` | `type = CIRCLE`, random circle (radius 0 / positive / negative / huge) → `count = 1` | [x] |
| 17 | `c2MakeProxy` | `type = AABB`, random AABB incl. degenerate & inverted → `count = 4`, `radius = 0` | [x] |
| 18 | `c2MakeProxy` | `type = CAPSULE`, random capsule incl. `a == b` → `count = 2` | [x] |
| 19 | `c2GJKSimplexMetric` | `count = 1` → `0`; `count = 2` → length; `count = 3` → det — random simplex `p` values, incl. degenerate/collinear and NaN | [x] |
| 20 | `c2Support` | `count = 1` (circle proxy) with random direction | [x] |
| 21 | `c2Support` | `count = 2` (capsule proxy), random direction incl. perpendicular ties | [x] |
| 22 | `c2Support` | `count = 4` (AABB proxy), random direction incl. axis-aligned ties (first-max wins) | [x] |
| 23 | `c2Support` | `count = 8` (full proxy array), random verts and direction; NaN verts | [x] |
| 24 | `c22` | random 2-simplex hitting the `v <= 0` arm (origin beyond `a`) | [x] |
| 25 | `c22` | random 2-simplex hitting the `u <= 0` arm (origin beyond `b`, vertex shifted) | [x] |
| 26 | `c22` | random 2-simplex hitting the interior arm (`count = 2`, `div = u+v`) | [x] |
| 27 | `c22` | fully random `p` values (all arms mixed, incl. degenerate `a == b`, NaN) | [x] |
| 28 | `c23` | 3-simplex hitting vertex arm A (`vAB <= 0 && uCA <= 0`) | [x] |
| 29 | `c23` | vertex arm B (`uAB <= 0 && vBC <= 0`) | [x] |
| 30 | `c23` | vertex arm C (`uBC <= 0 && vCA <= 0`) | [x] |
| 31 | `c23` | edge arm AB (`uAB>0 && vAB>0 && wABC<=0`) | [x] |
| 32 | `c23` | edge arm BC (`uBC>0 && vBC>0 && uABC<=0`) | [x] |
| 33 | `c23` | edge arm CA (`uCA>0 && vCA>0 && vABC<=0`) | [x] |
| 34 | `c23` | interior arm (origin inside triangle, `count = 3`) | [x] |
| 35 | `c23` | fully random `p` values incl. collinear (`area == 0`), coincident, NaN | [x] |
| 36 | `c2D` | `count = 1`; `count = 2` with `det > 0` (skew branch); `count = 2` with `det <= 0` (CCW90 branch); random | [x] |
| 37 | `c2L` | `count = 1`; `count = 2` with random `u`/`div`; random | [x] |
| 38 | `c2Witness` | `count = 1`, random `sA`/`sB` | [x] |
| 39 | `c2Witness` | `count = 2`, random `u`, `div` | [x] |
| 40 | `c2Witness` | `count = 3`, random `u`, `div` | [x] |
| 41 | `c2GJK` | `CIRCLE`×`CIRCLE`, `use_radius = 0`, both transforms `NULL`, no cache, all out-params supplied | [x] |
| 42 | `c2GJK` | `CIRCLE`×`CIRCLE`, `use_radius = 1`, transforms `NULL`, no cache | [x] |
| 43 | `c2GJK` | `CIRCLE`×`AABB`, `use_radius ∈ {0,1}`, transforms `NULL`, no cache | [x] |
| 44 | `c2GJK` | `CIRCLE`×`CAPSULE`, `use_radius ∈ {0,1}`, transforms `NULL`, no cache | [x] |
| 45 | `c2GJK` | `AABB`×`CIRCLE`, `use_radius ∈ {0,1}`, transforms `NULL`, no cache | [x] |
| 46 | `c2GJK` | `AABB`×`AABB`, `use_radius ∈ {0,1}`, transforms `NULL`, no cache | [x] |
| 47 | `c2GJK` | `AABB`×`CAPSULE`, `use_radius ∈ {0,1}`, transforms `NULL`, no cache | [x] |
| 48 | `c2GJK` | `CAPSULE`×`CIRCLE`, `use_radius ∈ {0,1}`, transforms `NULL`, no cache | [x] |
| 49 | `c2GJK` | `CAPSULE`×`AABB`, `use_radius ∈ {0,1}`, transforms `NULL`, no cache | [x] |
| 50 | `c2GJK` | `CAPSULE`×`CAPSULE`, `use_radius ∈ {0,1}`, transforms `NULL`, no cache | [x] |
| 51 | `c2GJK` | all 9 type pairs × `use_radius ∈ {0,1}` with `ax_ptr` supplied (random unit rotation + translation), `bx_ptr = NULL` | [x] |
| 52 | `c2GJK` | all 9 type pairs × `use_radius ∈ {0,1}` with `bx_ptr` supplied, `ax_ptr = NULL` | [x] |
| 53 | `c2GJK` | all 9 type pairs × `use_radius ∈ {0,1}` with **both** transforms supplied (random unit rotations + translations) | [x] |
| 54 | `c2GJK` | both transforms supplied with **non-unit** `c2r` (`c²+s² != 1`) — the C never normalises | [x] |
| 55 | `c2GJK` | both transforms = explicit identity struct (must equal the `NULL` result) | [x] |
| 56 | `c2GJK` | overlapping shapes (`hit` path, 3-simplex) across all 9 type pairs | [x] |
| 57 | `c2GJK` | far-apart shapes (clean separating axis) across all 9 type pairs | [x] |
| 58 | `c2GJK` | exactly touching shapes (`dist ≈ rA + rB`) across all 9 type pairs, `use_radius = 1` | [x] |
| 59 | `c2GJK` | coincident shapes (identical position) across all 9 type pairs | [x] |
| 60 | `c2GJK` | degenerate shapes: zero-radius circle, zero-extent AABB, zero-length capsule, all pairs | [x] |
| 61 | `c2GJK` | huge-coordinate shapes (`±1e30`) — `d0/d1` overflow, `iter` early-exit paths | [x] |
| 62 | `c2GJK` | cache supplied, cold (`count = 0`): result must equal the `cache = NULL` result, and the written-back cache is compared field-by-field | [x] |
| 63 | `c2GJK` | cache warm-started from a previous call on the same shapes (`count ∈ {1,2,3}`), then re-run — cache contents and return value compared | [x] |
| 64 | `c2GJK` | cache warm chain: 4 successive calls with the shape translated a little each time, cache carried forward (the real consumer pattern) | [x] |
| 65 | `c2GJK` | cache reuse with a *different* shape pair than the cache was built from (indices still in range) | [x] |
| 66 | `c2GJK` | `outA = NULL`, `outB` supplied; `outA` supplied, `outB = NULL`; both `NULL`; `iterations = NULL` — return value must still match | [x] |
| 67 | `c2GJK` | `iterations` supplied — the iteration count itself is compared (catches divergent loop trip counts) | [x] |
| 68 | `c2AABBtoAABB` | random AABB pairs: separated on each of the 4 sides, overlapping, nested, edge-touching, zero-extent, inverted | [x] |
| 69 | `c2CircletoCircle` | random pairs: separated, overlapping, nested, exactly touching, zero radius, negative radius | [x] |
| 70 | `c2CircletoAABB` | random: centre inside box, outside on each face, outside at each corner, exactly on the boundary, zero-extent box | [x] |
| 71 | `c2CircletoCapsule` | random hitting the `da < 0` arm (before `a`) | [x] |
| 72 | `c2CircletoCapsule` | random hitting the `db < 0` arm (between `a` and `b`, perpendicular distance) | [x] |
| 73 | `c2CircletoCapsule` | random hitting the `else` arm (beyond `b`) | [x] |
| 74 | `c2CircletoCapsule` | degenerate capsule `a == b`; zero radii; fully random | [x] |
| 75 | `c2AABBtoCapsule` | random: separated, overlapping, capsule crossing the box, capsule inside the box, degenerate box/capsule | [x] |
| 76 | `c2CapsuletoCapsule` | random: parallel, crossing, collinear, coincident, separated, degenerate (point) capsules | [x] |
| 77 | `c2Collided` | `CIRCLE`×`CIRCLE` — random circles | [x] |
| 78 | `c2Collided` | `CIRCLE`×`AABB` — note the C passes `(A, B)` straight through | [x] |
| 79 | `c2Collided` | `CIRCLE`×`CAPSULE` | [x] |
| 80 | `c2Collided` | `AABB`×`CIRCLE` — the C **swaps** the arguments (`c2CircletoAABB(*B, *A)`) | [x] |
| 81 | `c2Collided` | `AABB`×`AABB` | [x] |
| 82 | `c2Collided` | `AABB`×`CAPSULE` | [x] |
| 83 | `c2Collided` | `CAPSULE`×`CIRCLE` — arguments **swapped** | [x] |
| 84 | `c2Collided` | `CAPSULE`×`AABB` — arguments **swapped** | [x] |
| 85 | `c2Collided` | `CAPSULE`×`CAPSULE` | [x] |
| 86 | `reverse_collide` | random `(x, y, r)` over the play area (hits all 3 bits and every combination of them) | [x] |
| 87 | `reverse_collide` | boundary sweep: `r` exactly grazing each of the three fixed shapes; `r = 0`; negative `r`; huge `r` (all 3 bits set) | [x] |
| 88 | `reverse_collide` | mixed float pool for `x`, `y`, `r` (`±0`, denormal, `±inf`, `NaN`, `±FLT_MAX`) | [x] |
| 89 | `c2BBVerts` | **`out` overlaps `*bb`** — no `restrict`, and the C interleaves reads of `bb` with writes to `out`, so `out[1]` clobbers `bb->max` before `out[2]` reads it | [x] |
| 90 | `c2MakeProxy` | **`shape` aliases `p`** — the C writes `p->radius`/`p->count` before reading `c->p`/`c->a`/`c->b`, and for the AABB arm `p->verts` overlaps `*bb` | [x] |
| 91 | `c2Witness` | **`a` and/or `b` point into `*s`**, at every `c2v`-aligned field the function reads (`verts[k].sA`, `verts[k].sB`), for every `count` | [x] |
| 92 | `c2Witness` | **`a == b`** (same `c2v`): the second write wins | [x] |
| 93 | `c2GJK` | **out-params overlapping each other and `*cache`**: the C's write order is `cache`, `outA`, `outB`, `iterations` | [x] |
| 94 | `c2GJK` | **`A == B`** (one blob read as both shapes), with and without a cache | [x] |

## Where each row is tested

| rows | test file :: test |
|------|-------------------|
| 1–14 | `tests/math.rs :: row01_c2v` … `row14_mulxv` |
| 15–23 | `tests/structs.rs :: row15_bbverts`, `row16_17_18_makeproxy`, `row19_simplex_metric`, `row20_23_support` |
| 24–40 | `tests/simplex.rs :: row24_27_c22`, `row28_35_c23`, `row36_c2d`, `row37_c2l`, `row38_40_c2witness` |
| 41–50, 56–61 | `tests/gjk.rs :: row41_50_type_pairs_no_transform` (the 6 geometric relations × 9 type pairs × `use_radius` cross-product) |
| 51–55 | `tests/gjk.rs :: row51_55_transforms` |
| 62 | `tests/gjk.rs :: row62_cold_cache` |
| 63–64 | `tests/gjk.rs :: row63_64_warm_cache` |
| 65 | `tests/gjk.rs :: row65_cross_shape_cache_reuse` |
| 66–67 | `tests/gjk.rs :: row66_67_out_param_combinations` |
| 68–76 | `tests/shapes.rs :: row68_aabb_to_aabb` … `row76_capsule_to_capsule` |
| 77–85 | `tests/shapes.rs :: row77_85_collided` |
| 86–88 | `tests/shapes.rs :: row86_88_reverse_collide` |
| 89–94 | `tests/aliasing.rs` |
| layout / harness self-checks | `tests/layout.rs` |

## Branch-coverage evidence

A row is only checked off once the branch it targets is *proven* to have run.
Each solver/dispatcher test classifies its own inputs with the same predicates
the C uses and fails if any arm was never reached. Measured on the default seed:

```
c22  arms:                 keep-a=13568  keep-b=6555  edge=19877
c23  arms:                 A=9953 B=2520 C=2717 AB=4657 BC=3643 CA=3507 interior=13003
c2CircletoCapsule arms:    before-a=6599  middle=13118  beyond-b=10361
c2GJK loop exits:          Hit, NoProgress, DegenerateDir, Dup   (IterCap proven unreachable)
c2GJK use_radius arms:     SkippedByHit, Disabled, Shrink, ShrinkCollapsed, Midpoint
c2GJK cache metric guard:  rejected=1600  accepted=6496  (NaN-metric accepts=534)
c2GJK cache counts seen:   count1=6306  count2=2477  count3=1297
reverse_collide results:   all 8 bitmask values 0..7 produced
```

For `c2GJK`, whose five loop exits and five `use_radius` arms are not visible in
the return value, the control flow is re-traced in
`tests/common/mod.rs :: classify()` using the **C library's own exported
primitives** (`c2MakeProxy`, `c22`, `c23`, `c2L`, `c2D`, `c2Support`, `c2Witness`,
… all via `dlsym`). That is used only to label an input for coverage
bookkeeping; the correctness assertion is always the C-vs-Rust comparison.

## Comparison policy

Outputs are decomposed into typed 4-byte lanes (`Lane::F` / `Lane::I`) and
compared lane-by-lane; `tests/layout.rs` asserts the decomposition covers
`size_of::<T>() / 4` words for every struct, so no byte is skipped and no
padding can hide a difference.

Every value class is compared **bit-for-bit**: ordinary values, `+0` vs `-0`,
denormals, and infinities. The single exception is the **payload and sign of a
NaN**, which is compared only as "both are NaN". IEEE 754 and the C standard both
leave NaN payload propagation unspecified, and the two toolchains legitimately
differ: `gcc -O0` compiles `a.x += b.x` to `addss %xmm1(a.x),%xmm0(b.x)`, so the
*second* operand's NaN survives, while LLVM keeps `a.x` in the destination and the
*first* survives; LLVM also folds `-a.s*b.x + a.c*b.y` into an `fsub`, which does
not flip the NaN sign that the C's explicit negation does. No NaN payload is
observable through any comparison this library performs, so it cannot affect any
`int` or boolean result. `tests/layout.rs :: comparator_rejects_real_differences`
proves the tolerance does not leak: a 1-ulp change, a `+0`/`-0` flip and an `int`
field change are all still rejected.

## Restrictions (undefined behaviour in the C, not differentially testable)

* **Row 65** only reuses a cache whose stored indices are in range for the new
  proxies. `c2MakeProxy` writes only `verts[0 .. count)`, so an out-of-range
  index makes the C read `c2Proxy` slots it never initialised.
* `c2GJK` with a `C2_TYPE` outside `{0,1,2}` is excluded for the same reason,
  and worse: the uninitialised `pA.count` reaches `c2Support` as a loop bound and
  SIGSEGVs. See the corresponding entry in `ERRORS.md`.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../../translation
cargo build --release          # produces the Rust .so the tests dlopen
./check_symbols.sh             # Phase A / D symbol parity
./check_features.sh            # Phases B-D under every feature combination
cargo test                     # the differential suite

# Confirm the results are not an artefact of one seed:
for o in 0 1 2 7 13 101 4242 999983; do DIFF_SEED_OFFSET=$o cargo test; done
```

Note: the C `.so` is linked **without `-lm`**, so its `sqrtf` is left undefined.
The harness `dlopen`s `libm.so.6` with `RTLD_GLOBAL` before loading it
(`tests/common/mod.rs :: preload_libm`), since `c_src/` must not be modified.
