# CONFIGS.md — configuration surface (Phase B)

Every row is a *valid-input* configuration that the C code treats differently
(derived mechanically from the `if` / `switch` / ternary branches in
`c_src/src/lib.c`, not from guesses about which options "matter").
Each row is exercised against BOTH `.so`s with **many randomized inputs**
(fixed seeds, property style) and compared **bit-for-bit** (`f32::to_bits`,
including the sign and payload of NaN results, plus the `int` return value and
the full `c2Raycast` out-parameter).

## Build-time configuration surface

| axis | values | note |
|------|--------|------|
| Cargo `[features]` | *(none declared)* | one configuration only: `--no-default-features` ≡ default |
| Cargo profile | `dev`, `release` (`panic = "abort"`) | both built & tested (`PROFILE_FLAGS=--release ./run_diff_tests.sh`) |
| CMake options | *(none declared)* | `CMakeLists.txt` has no `option()`/`target_compile_definitions` |
| C preprocessor | *(no `#if`/`#ifdef` anywhere in `lib.c`/`lib.h`)* | single translation configuration |

### Important: the C's own NaN payloads are optimization-level dependent

The reference artifact is the one the task specifies:

```sh
cd c_src && mkdir -p build && cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
```

No `CMAKE_BUILD_TYPE` is set, so this is an **`-O0`** build, and the Rust
translation's operand ordering was read off *that* artifact's disassembly
(`objdump -d`), instruction by instruction.

This matters because x86 SSE returns the **destination** operand when BOTH
operands of an arithmetic instruction are NaN, and gcc freely commutes
`mulss`/`addss` and constant-folds `p * -1.0f` at higher optimization levels.
Building the same `lib.c` at `-O2`/`-O3` therefore produces a library that
disagrees with the `-O0` one on those inputs.  Measured with the differential
harness pointed at each build (`C_SO_PATH=... cargo test`):

| inputs | `-O0` (reference) | `-O2` | `-O3` |
|--------|-------------------|-------|-------|
| any **finite / non-NaN** values (400 000+ checks: B12-B18, B19-B31, B32-B41, B42-B56, B57-B68) | identical | identical | identical |
| both operands of one SSE op are NaN (`t10_torture`, NaN-heavy) | identical | ~9 % of leaf-op checks differ | ~11 % differ |

Since the two C builds contradict each other, no single implementation can
match both; this translation matches the specified reference build exactly and
matches *every* build for every non-NaN input.  Concretely, the `-O0` artifact
differs from `-O2`/`-O3` in: `c2Dot`/`c2MulmvT` (product/summand order),
`c2Mulvs` (`mulps` with the broadcast scalar as destination at `-O2`),
`c2Len`/`c2Norm`/`c2Div` (inlined `c2Dot`), and
`c2SignedDistPointToPlane_OneDimensional` (`-O2` folds `p * -1.0f - d * -1.0f`
into `d - p`, which is a different NaN result; `-O0` keeps both `mulss`es).

## Runtime configuration axes actually branched on by the C

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `C2_TYPE typeB` (the only "mode" flag in the API) | `C2_TYPE_CIRCLE=0`, `C2_TYPE_AABB=1`, `C2_TYPE_CAPSULE=2`, out-of-range | `c2CastRay` `switch` |
| shape kind reinterpreted from `const void *B` | `c2Circle` (12 B), `c2AABB` (16 B), `c2Capsule` (20 B) | `*(T*)B` casts |
| ray length `A.t` | `> 0`, `== 0`, `< 0`, `NaN`, `inf` | `t <= A.t`, `p1 = A.p + A.d*A.t` |
| ray direction `A.d` | unit, non-unit, zero vector, `NaN`/`inf` | `c2Norm` of a zero vector ⇒ `NaN` |
| radius `r` | `> 0`, `== 0`, `< 0`, `NaN`, `inf` | `d2 < r*r`, `min(...) < B.r` |
| AABB shape | proper (`min<max`), degenerate (`min==max`), inverted (`min>max`), `NaN` | `c2AABBtoAABB`, `c2AABBtoPoint` |
| capsule shape | `a != b`, `a == b` (degenerate ⇒ `NaN` basis), `r < 0` (x-inverted `capsule_bb`); note `yBb.y == \|b-a\| >= 0` always | `c2Norm(b-a)`, `capsule_bb` |
| float class of every scalar | `+0`, `-0`, subnormal, normal, `FLT_MAX/MIN`, `±inf`, qNaN, sNaN, random bit patterns | ternary `min`/`max`/`abs` macros and `comiss` (unordered ⇒ all `<`/`>` false) |
| hit face selected (AABB) | `t0` / `t1` / `t2` / `t3` winner ⇒ `n = (-1,0)/(1,0)/(0,-1)/(0,1)` | 4-way `>=` cascade |
| capsule branch | 10 distinct outcomes (see B42…B51) | nested `if`s |
| `gen_ray` hit mask | all 8 values `0b000`…`0b111` | `hit + (x<<1) + (y<<2)` |

## Row table

Legend: **[x]** = passing across randomized inputs, both `.so`s bit-identical.

### Leaf vector helpers — `tests/t1_vector_ops.rs`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| B01 | `c2V` | 20 000 × (nice ∪ hostile) float pairs; struct-return ABI in `xmm0` | [x] |
| B02 | `c2Dot` | 20 000 × all 4 nice/hostile combinations of both vectors | [x] |
| B03 | `c2Len` | 20 000 × nice/hostile vectors incl. zero vector, `±inf`, sNaN (⇒ `sqrtf` parity) | [x] |
| B04 | `c2Add`, `c2Sub` | 20 000 × 4 nice/hostile combinations (both-NaN operand ordering) | [x] |
| B05 | `c2Mulvs` | 20 000 × vector×scalar, 4 nice/hostile combinations | [x] |
| B06 | `c2Div` | 20 000 × vector÷scalar incl. `b = ±0`, `±inf`, NaN (reciprocal-then-multiply) | [x] |
| B07 | `c2Norm` | 20 000 × incl. zero vector (`0/0`), `inf` components, NaN | [x] |
| B08 | `c2Minv`, `c2Maxv` | 20 000 × incl. `±0` pairs and NaN pairs (ternary macro, not `fminf`) | [x] |
| B09 | `c2Skew`, `c2Absv`, `c2CCW90` | 20 000 × incl. `-0.0` (macro does *not* clear the sign) and NaN | [x] |
| B10 | `c2MulmvT` | 20 000 × 4 matrix/vector nice/hostile combinations | [x] |
| B11 | all 1- and 2-operand leaf ops | exhaustive cross product of 10 special values (`±0`, `±inf`, qNaN, sNaN, `±FLT_MAX`, subnormal) over every operand position | [x] |

### Overlap predicates — `tests/t2_overlap.rs`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| B12 | `c2AABBtoAABB` | random overlapping box pairs (return 1) | [x] |
| B13 | `c2AABBtoAABB` | separated along each of the 4 axes: `d0`, `d1`, `d2`, `d3` individually | [x] |
| B14 | `c2AABBtoAABB` | edge-touching (`A.max.x == B.min.x`, etc.) — exact `<` boundary | [x] |
| B15 | `c2AABBtoAABB` | degenerate (`min == max`) and inverted (`min > max`) boxes | [x] |
| B16 | `c2AABBtoAABB` | NaN / ±inf coordinates (unordered ⇒ every `<` false ⇒ returns 1) | [x] |
| B17 | `c2AABBtoPoint` | point inside; outside via each of `d0..d3`; exactly on each edge/corner; NaN point; inverted box | [x] |
| B18 | `c2CircleToPoint` | point inside; outside; exactly on the circumference (`d2 == r*r`); `r == 0`; `r < 0`; NaN/inf `r` and point | [x] |

### `c2RaytoCircle` (lowest-level raycast) — `tests/t3_ray_circle.rs`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| B19 | `c2RaytoCircle` | origin outside, unit direction toward the circle, `A.t` long ⇒ hit with `0 < t < A.t` | [x] |
| B20 | `c2RaytoCircle` | origin exactly on the circle (`t == 0` boundary of `t >= 0`) | [x] |
| B21 | `c2RaytoCircle` | impact exactly at `t == A.t` (boundary of `t <= A.t`) | [x] |
| B22 | `c2RaytoCircle` | origin strictly inside the circle (`disc > 0` but `t < 0`) | [x] |
| B23 | `c2RaytoCircle` | direction pointing away from the circle | [x] |
| B24 | `c2RaytoCircle` | tangent ray (`disc == 0`, grazing) | [x] |
| B25 | `c2RaytoCircle` | `A.t == 0` (zero-length ray) | [x] |
| B26 | `c2RaytoCircle` | `A.t < 0` (negative ray length) | [x] |
| B27 | `c2RaytoCircle` | `r == 0`, `r < 0`, `r` huge (`1e30`), `r == inf` | [x] |
| B28 | `c2RaytoCircle` | non-normalized direction (‖d‖ = 0.1 … 100) — changes the meaning of `t` | [x] |
| B29 | `c2RaytoCircle` | zero direction vector `d = (0,0)` and `d = c2Norm((0,0))` (NaN) | [x] |
| B30 | `c2RaytoCircle` | NaN / ±inf in every field position of `A` and `B` | [x] |
| B31 | `c2RaytoCircle` | 20 000 unconstrained random rays × circles (nice), 20 000 hostile | [x] |

### `c2RaytoAABB` — `tests/t4_ray_aabb.rs`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| B32 | `c2RaytoAABB` | ray crossing the box, each of the 4 face-normal outcomes `(-1,0)`, `(1,0)`, `(0,-1)`, `(0,1)` | [x] |
| B33 | `c2RaytoAABB` | ray entirely inside the box | [x] |
| B34 | `c2RaytoAABB` | `a_box` overlaps `B` but the ray-normal separating axis rejects (`d > 0`) | [x] |
| B35 | `c2RaytoAABB` | axis-aligned rays: `d = (±1,0)` and `d = (0,±1)` (`da*db` zero products) | [x] |
| B36 | `c2RaytoAABB` | `A.t == 0` ⇒ `p1 == p0` ⇒ `ab = 0`, `n = 0`, all `da == db` (`d == 0` in `c2RayToPlane`) | [x] |
| B37 | `c2RaytoAABB` | degenerate box (`min == max`), inverted box (`min > max`) | [x] |
| B38 | `c2RaytoAABB` | ray endpoints exactly on a face / corner of the box | [x] |
| B39 | `c2RaytoAABB` | inputs chosen so each subset of `hit0..hit3` is set (incl. exactly one, all four) | [x] |
| B40 | `c2RaytoAABB` | NaN / ±inf in every field of `A` and `B` (incl. NaN `t_i` surviving `(float)hitN * tN`) | [x] |
| B41 | `c2RaytoAABB` | 20 000 unconstrained random (nice) + 20 000 hostile | [x] |

### `c2RaytoCapsule` — `tests/t5_ray_capsule.rs`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| B42 | `c2RaytoCapsule` | `yAp` inside `capsule_bb` ⇒ early `return 1` with `out` = `{0, norm(b-a)}` | [x] |
| B43 | `c2RaytoCapsule` | `A.p` inside end-cap circle `a` ⇒ `return 1` | [x] |
| B44 | `c2RaytoCapsule` | `A.p` inside end-cap circle `b` ⇒ `return 1` | [x] |
| B45 | `c2RaytoCapsule` | big condition false (`yAe.x*yAp.x >= 0` **and** `min(\|yAe.x\|,\|yAp.x\|) >= r`) ⇒ `return 0` | [x] |
| B46 | `c2RaytoCapsule` | `\|yAp.x\| < r` and `yAp.y < 0` ⇒ delegates to `c2RaytoCircle(Ca)` | [x] |
| B47 | `c2RaytoCapsule` | `\|yAp.x\| < r` and `yAp.y >= 0` ⇒ delegates to `c2RaytoCircle(Cb)` | [x] |
| B48 | `c2RaytoCapsule` | else-branch with `y <= 0` ⇒ `c2RaytoCircle(Ca)` | [x] |
| B49 | `c2RaytoCapsule` | else-branch with `y >= yBb.y` ⇒ `c2RaytoCircle(Cb)` | [x] |
| B50 | `c2RaytoCapsule` | else-branch side hit, `c > 0` ⇒ `out->n = M.x`, `out->t = t*A.t` | [x] |
| B51 | `c2RaytoCapsule` | else-branch side hit, `c <= 0` ⇒ `out->n = c2Skew(M.y)` | [x] |
| B52 | `c2RaytoCapsule` | degenerate capsule `a == b` ⇒ `c2Norm((0,0))` ⇒ NaN basis everywhere | [x] |
| B53 | `c2RaytoCapsule` | degenerate/inverted `capsule_bb`: `yBb.y == 0`, `yBb.y == NaN`, and x-inverted (`r < 0` ⇒ `min.x > max.x`). A *negative* `yBb.y` is unreachable because `yBb.y == \|b-a\|`; the test asserts this over 40 000 swap-order trials | [x] |
| B54 | `c2RaytoCapsule` | `r == 0`, `r < 0`, `r == inf` | [x] |
| B55 | `c2RaytoCapsule` | NaN / ±inf in every field of `A` and `B` | [x] |
| B56 | `c2RaytoCapsule` | 20 000 unconstrained random (nice) + 20 000 hostile | [x] |

### `c2CastRay` dispatcher — `tests/t6_castray.rs`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| B57 | `c2CastRay` | `typeB = C2_TYPE_CIRCLE`, `B` → `c2Circle`; hit and miss configurations | [x] |
| B58 | `c2CastRay` | `typeB = C2_TYPE_AABB`, `B` → `c2AABB`; hit and miss configurations | [x] |
| B59 | `c2CastRay` | `typeB = C2_TYPE_CAPSULE`, `B` → `c2Capsule`; hit and miss configurations | [x] |
| B60 | `c2CastRay` | one 20-byte buffer reinterpreted under all three `typeB` values (aliasing / partial reads) | [x] |
| B61 | `c2CastRay` | `out` pre-poisoned: verifies the *same* fields are (or are not) written by both libs | [x] |

### `gen_ray` (public header entry point) — `tests/t7_gen_ray.rs`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| B62 | `gen_ray` | hand-constructed scenarios covering all 8 hit masks `0b000`…`0b111` | [x] |
| B63 | `gen_ray` | `mp == ray.p` ⇒ `c2Norm((0,0))` ⇒ NaN direction and NaN `ray.t` | [x] |
| B64 | `gen_ray` | far-away mouse point + tiny radii/extents (large `A.t`, subnormal shapes) | [x] |
| B65 | `gen_ray` | hostile floats (`±0`, `±inf`, qNaN, sNaN, subnormal, `FLT_MAX`) in all 16 scalar parameters | [x] |
| B66 | `gen_ray` | aliased out-pointers (`cast1 == cast2 == cast3`) — write-order visibility | [x] |
| B67 | `gen_ray` | 20 000 random "nice" parameter sets (geometry-plausible) | [x] |
| B68 | `gen_ray` | 20 000 random hostile parameter sets | [x] |

## Coverage evidence (objective, from gcov of the C reference)

The C reference was rebuilt with `gcc -O0 -fPIC -shared --coverage` and the
FULL differential suite was run against it (`C_SO_PATH=…/libcov.so cargo test`,
all 76 test functions pass against the instrumented build too):

```
File 'src/lib.c'
Lines executed:99.53% of 212
Branches executed:100.00% of 88
Taken at least once:98.86% of 88
Calls executed:100.00% of 89
```

* **100 % of branches evaluated**, **100 % of calls executed**.
* The single unexecuted line and the single never-taken branch are the same
  thing: `c_src/src/lib.c:128-129`,
  `else if (da * db > 0) return 1.0f;` inside
  `c2RayToPlane_OneDimensional` — **0 of 1 633 554** evaluations took it.
  That arm is provably **dead code** when reached from its only caller
  (`c2RaytoAABB` rejects via `c2AABBtoAABB` before the planes are evaluated
  whenever `da > 0 && db > 0`; proof and a 631 608-plane counterexample search
  in `tests/t4_ray_aabb.rs::dead_code_da_times_db_positive_is_unreachable`).

Per-function call counts from the same run (evidence that every entry point was
driven, not just the convenience wrapper `gen_ray`).  `blocks executed` is 100 %
for all 24 functions except the one holding the dead branch:

| function | times called | blocks executed |
|---|---|---|
| `c2V` | 11 222 630 | 100 % |
| `c2Dot` | 11 953 544 | 100 % |
| `c2Len` | 3 959 397 | 100 % |
| `c2Add` | 4 392 221 | 100 % |
| `c2Sub` | 11 874 187 | 100 % |
| `c2Mulvs` | 9 193 217 | 100 % |
| `c2Div` | 3 960 586 | 100 % |
| `c2Norm` | 3 738 289 | 100 % |
| `c2Minv` | 1 845 743 | 100 % |
| `c2Maxv` | 1 845 743 | 100 % |
| `c2Skew` | 1 142 029 | 100 % |
| `c2Absv` | 1 070 409 | 100 % |
| `c2CCW90` | 1 813 409 | 100 % |
| `c2MulmvT` | 4 801 524 | 100 % |
| `c2AABBtoAABB` | 1 935 744 | 100 % |
| `c2AABBtoPoint` | 1 733 809 | 100 % |
| `c2CircleToPoint` | 1 673 006 | 100 % |
| `c2SignedDistPointToPlane_OneDimensional` (static) | 5 071 616 | 100 % |
| `c2RayToPlane_OneDimensional` (static) | 2 535 808 | **88 %** (dead `return 1.0f`) |
| `c2RaytoCircle` | 1 423 952 | 100 % |
| `c2RaytoAABB` | 1 215 743 | 100 % |
| `c2RaytoCapsule` | 1 293 409 | 100 % |
| `c2CastRay` | 1 648 073 | 100 % |
| `gen_ray` | 428 006 | 100 % |

## Row → test-function traceability

Verified mechanically by `./audit_rows.py`, which checks that every row ID below
names a test function that exists in `tests/` and that the function PASSES.

| rows | test function | file |
|------|---------------|------|
| B01 | `b01_c2v_construct` | `tests/t1_vector_ops.rs` |
| B02 | `b02_c2dot` | `tests/t1_vector_ops.rs` |
| B03 | `b03_c2len` | `tests/t1_vector_ops.rs` |
| B04 | `b04_c2add`, `b04_c2sub` | `tests/t1_vector_ops.rs` |
| B05 | `b05_c2mulvs` | `tests/t1_vector_ops.rs` |
| B06 | `b06_c2div` | `tests/t1_vector_ops.rs` |
| B07 | `b07_c2norm` | `tests/t1_vector_ops.rs` |
| B08 | `b08_c2minv`, `b08_c2maxv` | `tests/t1_vector_ops.rs` |
| B09 | `b09_c2skew`, `b09_c2absv`, `b09_c2ccw90` | `tests/t1_vector_ops.rs` |
| B10 | `b10_c2mulmvt` | `tests/t1_vector_ops.rs` |
| B11 | `b11_specials_cross_product` | `tests/t1_vector_ops.rs` |
| B12 | `b12_aabb_overlapping` | `tests/t2_overlap.rs` |
| B13 | `b13_e06_e09_aabb_separated_each_axis` | `tests/t2_overlap.rs` |
| B14 | `b14_aabb_touching` | `tests/t2_overlap.rs` |
| B15 | `b15_aabb_degenerate_inverted` | `tests/t2_overlap.rs` |
| B16 | `b16_e10_aabb_nan_inf` | `tests/t2_overlap.rs` |
| B17 | `b17_e16_aabb_to_point` | `tests/t2_overlap.rs` |
| B18 | `b18_e17_e19_circle_to_point` | `tests/t2_overlap.rs` |
| B19 | `b19_hit_generic` | `tests/t3_ray_circle.rs` |
| B20 | `b20_t_zero_boundary` | `tests/t3_ray_circle.rs` |
| B21 | `b21_e04_t_equals_ray_length` | `tests/t3_ray_circle.rs` |
| B22 | `b22_e03_origin_inside` | `tests/t3_ray_circle.rs` |
| B23 | `b23_e01_miss` | `tests/t3_ray_circle.rs` |
| B24 | `b24_tangent` | `tests/t3_ray_circle.rs` |
| B25, B26 | `b25_b26_ray_length_zero_negative` | `tests/t3_ray_circle.rs` |
| B27 | `b27_e33_radius_variants` | `tests/t3_ray_circle.rs` |
| B28 | `b28_non_normalized_direction` | `tests/t3_ray_circle.rs` |
| B29 | `b29_e30_degenerate_direction` | `tests/t3_ray_circle.rs` |
| B30 | `b30_e02_e05_special_in_each_field` | `tests/t3_ray_circle.rs` |
| B31 | `b31_fuzz` | `tests/t3_ray_circle.rs` |
| B32, B39 | `b32_b39_grid_sweep` | `tests/t4_ray_aabb.rs` |
| B33 | `b33_e14_ray_inside_box` | `tests/t4_ray_aabb.rs` |
| B34 | `b34_e12_separating_axis_reject` | `tests/t4_ray_aabb.rs` |
| B35 | `b35_axis_aligned` | `tests/t4_ray_aabb.rs` |
| B36 | `b36_e15_zero_length_ray` | `tests/t4_ray_aabb.rs` |
| B37 | `b37_degenerate_inverted_box` | `tests/t4_ray_aabb.rs` |
| B38 | `b38_endpoints_on_faces` | `tests/t4_ray_aabb.rs` |
| B40 | `b40_e11_e13_specials_per_field` | `tests/t4_ray_aabb.rs` |
| B41 | `b41_fuzz` | `tests/t4_ray_aabb.rs` |
| B42..B51 | `b42_b51_e20_e21_all_branches` (asserts all 10 branches hit ≥ 50×) | `tests/t5_ray_capsule.rs` |
| B52 | `b52_e22_degenerate_capsule` | `tests/t5_ray_capsule.rs` |
| B53 | `b53_inverted_capsule_bb` | `tests/t5_ray_capsule.rs` |
| B54 | `b54_e33_radius_variants` | `tests/t5_ray_capsule.rs` |
| B55 | `b55_specials_per_field` | `tests/t5_ray_capsule.rs` |
| B56 | `b56_fuzz` | `tests/t5_ray_capsule.rs` |
| B57 | `b57_dispatch_circle` | `tests/t6_castray.rs` |
| B58 | `b58_dispatch_aabb` | `tests/t6_castray.rs` |
| B59 | `b59_dispatch_capsule` | `tests/t6_castray.rs` |
| B60 | `b60_same_bytes_all_types` | `tests/t6_castray.rs` |
| B61 | `b61_out_write_parity` | `tests/t6_castray.rs` |
| B62 | `b62_e34_all_hit_masks` (asserts all 8 masks produced) | `tests/t7_gen_ray.rs` |
| B63 | `b63_zero_length_ray` | `tests/t7_gen_ray.rs` |
| B64 | `b64_extreme_scales` | `tests/t7_gen_ray.rs` |
| B65 | `b65_specials_per_parameter` | `tests/t7_gen_ray.rs` |
| B66 | `b66_aliased_out_pointers` | `tests/t7_gen_ray.rs` |
| B67 | `b67_fuzz_nice` | `tests/t7_gen_ray.rs` |
| B68 | `b68_fuzz_hostile` | `tests/t7_gen_ray.rs` |
| (all, adversarial) | `torture_leaf_ops`, `torture_ray_circle`, `torture_ray_aabb`, `torture_ray_capsule`, `torture_castray_and_gen_ray` | `tests/t10_torture.rs` |
