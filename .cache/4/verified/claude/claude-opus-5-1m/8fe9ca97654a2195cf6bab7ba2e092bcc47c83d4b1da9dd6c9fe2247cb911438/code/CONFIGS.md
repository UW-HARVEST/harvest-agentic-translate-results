# CONFIGS.md -- the configuration surface of the C library

Mirror image of `ERRORS.md`: every *valid* configuration the C code branches on.
Derived from the source, not from guesswork -- the axes below are exactly the
`#if`/`#ifdef`s of the build, the `if`/`switch` branches the exported functions
take on their arguments, and the input shapes those branches distinguish.

Each row is checked by calling **both** shared objects through `dlsym` with many
randomized inputs (fixed seed per test, `tests/harness/mod.rs::Rng`) plus the
hand-picked boundary values of that row, and comparing every output byte
(`f32::to_bits()`) and every returned integer.

## Axis 0 -- build configuration (compile time)

`c_src/CMakeLists.txt` has no options: it always compiles `src/q_math.c` and
`src/main.c` with `-Iinc -Isrc` and links `m`.  The preprocessor axes inside the
sources are therefore all pinned, and there is exactly **one** build
configuration:

| C axis | where | value in this build | consequence |
|---|---|---|---|
| `idppc` | q_shared.h:70, q_math.c:490 | `0` | `#if !idppc` is taken -> `Q_rsqrt` and `Q_fabs` are compiled (the only alternative would be a PowerPC intrinsic version) |
| `id386` / `idppc_altivec` | q_shared.h:69,71 | `0` | no asm paths |
| `Q3_VM` | q_shared.h:50 | undefined | system headers, not `bg_lib.h`; the commented-out `assert`s stay out |
| `__linux__` | q_shared.h:85 | defined | `ID_INLINE` = `inline`, `PATH_SEP='/'`, `BigShort/BigLong/BigFloat` inline decls (never defined -> unused) |
| `#if 1` | q_shared.h:363 | taken | `DotProduct`, `VectorSubtract`, `VectorAdd`, `VectorCopy`, `VectorScale`, `VectorMA` are the **macro** forms; the `_`-prefixed functions are compiled but never called internally |
| `M_PI` | q_shared.h:276 | already defined by `<math.h>` | `M_PI` is a `double`, so `DEG2RAD`/`RAD2DEG`/`AngleVectors`/`vectoangles` compute in `f64` |
| `PATH_MAX` | q_shared.h:148 | defined | `MAX_OSPATH` = `PATH_MAX` (unused here) |
| `_DEBUG`/`BSPC`/`HUNK_DEBUG` | q_shared.h:218 | undefined | `Hunk_Alloc` decl only (never defined) |
| `__VECTORC` | q_shared.h:243 | undefined | `Com_Memset`/`Com_Memcpy` decls only (never defined) |

`Cargo.toml` mirrors this with a single, empty `[features] default = []`, so the
complete list of Rust feature combinations to verify is:

| # | cargo invocation | meaning |
|---|---|---|
| 1 | `cargo test --no-default-features` | the only configuration |
| 2 | `cargo test --features=` (the default, empty) | identical to #1 |
| 3 | `cargo test --all-features` | identical to #1 (there are no features) |
| 4 | `cargo test --release` | same code, `-Cdebug-assertions=off`: the only profile-visible difference is that rustc's `ub_checks` no longer intercept the NULL dereferences (ERRORS.md row 50) |

`./run_all.sh` builds and runs all four, and re-checks symbol parity in each of
the three dev configurations.

## Axis 1 -- entry points, by level

Tested bottom-up; the low-level entry points are driven **directly**, not only
through the convenience wrappers.

| level | entry points |
|---|---|
| 0 -- `q_shared.h` macros | `DotProduct` `VectorSubtract` `VectorAdd` `VectorCopy` `VectorScale` `VectorMA` `VectorClear` `VectorNegate` `VectorSet` `Vector4Copy` `SnapVector` `IS_NAN` `SQRTFAST` `DEG2RAD` `RAD2DEG` `ANGLE2SHORT` `SHORT2ANGLE` `ColorIndex` `Square` `PlaneTypeForNormal` `MAKERGB` `MAKERGBA` `Q_IsColorString` `random` `crandom` (via the `w_*` hooks) |
| 0 -- `q_shared.h` `static ID_INLINE` | `VectorCompare` `VectorLength` `VectorLengthSquared` `Distance` `DistanceSquared` `VectorNormalizeFast` `VectorInverse` `CrossProduct` (via the `w_*` hooks) |
| 1 -- leaf functions | `Q_rsqrt` `Q_fabs` `ClampChar` `ClampShort` `Q_rand` `Q_log2` `ColorBytes3` `ColorBytes4` `AngleMod` `AngleNormalize360` `LerpAngle` `AngleSubtract` `SetPlaneSignbits` `BoxOnPlaneSide` `ClearBounds` `AddPointToBounds` `MatrixMultiply` `AngleVectors` `AxisClear` `AxisCopy` `_DotProduct` `_VectorAdd` `_VectorSubtract` `_VectorCopy` `_VectorScale` `_VectorMA` `Vector4Scale` `VectorRotate` `NormalizeColor` `DirToByte` `ByteToDir` |
| 2 -- one level of composition | `Q_random`(`Q_rand`) `Q_crandom`(`Q_random`) `VectorNormalize` `VectorNormalize2` `AngleNormalize180`(`AngleNormalize360`) `AngleDelta`(`AngleNormalize180`) `AnglesSubtract`(`AngleSubtract`) `RadiusFromBounds`(`VectorLength`) `ProjectPointOnPlane`(`DotProduct`) `vectoangles`(`atan2`,`sqrt`) |
| 3 -- two levels | `PerpendicularVector`(`ProjectPointOnPlane`,`VectorNormalize`) `PlaneFromPoints`(`CrossProduct`,`VectorNormalize`,`DotProduct`) `MakeNormalVectors`(`VectorMA`,`VectorNormalize`,`CrossProduct`) `AnglesToAxis`(`AngleVectors`) |
| 4 -- full pipeline | `RotatePointAroundVector`(`PerpendicularVector`,`CrossProduct`,`MatrixMultiply` x2,`cos`,`sin`) `RotateAroundDirection`(`PerpendicularVector`,`RotatePointAroundVector`,`CrossProduct`) |
| 5 -- program | `main` (`atof` -> `VectorNormalizeFast` -> `Q_rsqrt` -> `printf("%f")`), compared by running both executables |
| data | `vec3_origin` `axisDefault` `bytedirs` `g_color_table` `colorBlack` `colorRed` `colorGreen` `colorBlue` `colorYellow` `colorMagenta` `colorCyan` `colorWhite` `colorLtGrey` `colorMdGrey` `colorDkGrey` |

## Axis 2 -- input shapes used throughout

`S0` zero vector `{0,0,0}` &nbsp;|&nbsp; `S-0` negative zero &nbsp;|&nbsp;
`SA` axial unit vector &nbsp;|&nbsp; `SN` normalized random &nbsp;|&nbsp;
`SR` arbitrary finite random (full exponent range) &nbsp;|&nbsp;
`SH` huge (`1e30`, `FLT_MAX`: squares overflow to `inf`) &nbsp;|&nbsp;
`ST` tiny/denormal (`1e-30`, `1e-45`: squares underflow to `0`) &nbsp;|&nbsp;
`SI` ±`inf` &nbsp;|&nbsp; `SNaN` NaN &nbsp;|&nbsp;
`SAL` aliased in/out pointers &nbsp;|&nbsp; `SEQ` equal inputs.

## The configuration rows

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|-------------------------------------------|------|---|
| 1 | `Q_rand` | seed = 0, ±1, `INT_MAX`, `INT_MIN`, random; 200-step sequences (multiplication overflow every step) | `q_rand_sequences` | [x] |
| 2 | `Q_random` | same seeds, 100-step sequences; result is `(rand & 0xffff) / 65536.0f` | `q_random_and_crandom_sequences` | [x] |
| 3 | `Q_crandom` | same, plus the `f64` `2.0*(x-0.5)` round trip | `q_random_and_crandom_sequences` | [x] |
| 4 | `ClampChar` | full `-300..300` sweep + `INT_MIN`/`INT_MAX` + 2000 random | `clamp_char` | [x] |
| 5 | `ClampShort` | `±32000..±40000` sweep + `INT_MIN`/`INT_MAX` + 2000 random | `clamp_short` | [x] |
| 6 | `Q_rsqrt` | every exponent 0..255 x {mant 0, 1, 0x400000, 0x7fffff} x both signs; `S0 S-0 SI SNaN ST SH` + 20000 random | `q_rsqrt_all_shapes` | [x] |
| 7 | `Q_fabs` | 20000 arbitrary bit patterns incl. all NaN payloads (pure bit twiddling, so payloads must survive) | `q_fabs_all_shapes` | [x] |
| 8 | `Q_log2` | `0..1024`, every `2^k`/`2^k±1`, `INT_MAX`, 2000 random non-negative | `q_log2_non_negative` | [x] |
| 9 | `LerpAngle` | all 3 branches (`to-from > 180`, `< -180`, neither) x `frac` in {0,0.5,1,-1,2,NaN,inf} x `SI SNaN SH` | `lerp_angle` | [x] |
| 10 | `AngleSubtract` | both `while` loops iterating 0, 1 and many times, at the `±180` boundaries, `f32_mag(1e5)` random | `angle_subtract` | [x] |
| 11 | `AnglesSubtract` | 3 independent components, each hitting row 10's branches | `angles_subtract` | [x] |
| 12 | `AngleMod` | in-range, exactly at the `cvttsd2si` overflow boundary (`±11796480`), `SNaN SI`, 20000 random | `angle_mod_normalize_delta` | [x] |
| 13 | `AngleNormalize360` | same as row 12 | `angle_mod_normalize_delta` | [x] |
| 14 | `AngleNormalize180` | result `> 180` and `<= 180` (the `f64` `-360.0` branch) | `angle_mod_normalize_delta` | [x] |
| 15 | `AngleDelta` | 20000 random pairs (composition of rows 13+14) | `angle_mod_normalize_delta` | [x] |
| 16 | `ColorBytes3` | components `< 0`, `0..1` ramp, `> 1`, `>= 256`, `SI SNaN`; low 24 bits only (ERRORS.md row 6) | `color_bytes3` | [x] |
| 17 | `ColorBytes4` | full 0..255 ramp, rounding boundaries, `SI SNaN`, 20000 random; all 32 bits | `color_bytes4` | [x] |
| 18 | `_DotProduct` | `S0 SA SN SR SH ST SI SNaN`, 20000 random pairs, plus `SEQ` (same pointer twice) | `dot_product` | [x] |
| 19 | `_VectorAdd`, `_VectorSubtract` | 20000 random pairs; `SH` (overflow to inf), `SI` (inf-inf), `S-0` (0+-0) | `vector_add_sub` | [x] |
| 20 | `_VectorAdd`, `_VectorSubtract`, `_VectorScale`, `_VectorCopy`, `_VectorMA`, `Vector4Scale` | `SAL`: output aliases the first input, the second input, or both | `vector_ops_aliasing` | [x] |
| 21 | `_VectorScale`, `Vector4Scale` | scale = `0`, `-0`, `1`, `-1`, `inf`, NaN, denormal x `S0 SR SH` | `vector_scale` | [x] |
| 22 | `_VectorMA` | scale x vector cross product incl. `0*inf` | `vector_ma` | [x] |
| 23 | `_VectorCopy` | copies every bit pattern verbatim (incl. all NaN payloads) | `vector_copy_bits` | [x] |
| 24 | `VectorNormalize` | `length != 0` (`SN SR SA`), `length == 0` (`S0 S-0 ST`-underflow), `length` NaN (`SNaN SI`), `SAL` (in-place by definition) | `vector_normalize` | [x] |
| 25 | `VectorNormalize2` | same three length classes, separate output, plus `SAL` (`out == v`) | `vector_normalize2` | [x] |
| 26 | `NormalizeColor` | `max` from each of the 3 components, `max == 0`, `max < 0`, `SNaN` (comparisons false), `SAL` (`out == in`) | `normalize_color` | [x] |
| 27 | `RadiusFromBounds` | `mins > maxs`, `mins == maxs`, mixed signs, `S0 SH SI SNaN`, `SAL` | `radius_from_bounds` | [x] |
| 28 | `ClearBounds` | fresh buffers and aliased `mins == maxs` (the chained assignment reads back what it wrote) | `clear_bounds` | [x] |
| 29 | `AddPointToBounds` | point inside / outside on each of the 6 faces, after `ClearBounds`, `SNaN`, aliased `mins == maxs` | `add_point_to_bounds` | [x] |
| 30 | `VectorRotate` | identity / zero / random / singular matrix x `S0 SN SR SNaN`; `SAL` (`out == in`) | `vector_rotate` | [x] |
| 31 | `MatrixMultiply` | identity, zero, permutation, random, `SH` (overflow), `SNaN`; `SAL` (`out == in1`, `out == in2`, `in1 == in2`) | `matrix_multiply` | [x] |
| 32 | `AngleVectors` | all 8 NULL/non-NULL combinations of `forward`/`right`/`up` x angle shapes (0, 90, 180, 270, 360, negative, huge, `SNaN SI`, random) | `angle_vectors` | [x] |
| 33 | `AnglesToAxis` | composition of row 32 with `VectorSubtract(vec3_origin, right, axis[1])`; random + boundary angles | `angles_to_axis` | [x] |
| 34 | `AxisClear` | writes the 9 constants; aliased/overlapping `axis` array | `axis_clear` | [x] |
| 35 | `AxisCopy` | distinct arrays, `SAL` (`in == out`), arbitrary bit patterns | `axis_copy` | [x] |
| 36 | `ProjectPointOnPlane` | `normal` = `S0` (`1/0` -> inf), `SA`, `SN`, `SR`, `SH`, `SNaN`; `dst` aliasing `p` or `normal` | `project_point_on_plane` | [x] |
| 37 | `PerpendicularVector` | min-magnitude component at index 0, 1, 2; all components `>= 1` (`pos` stays 0); `S0` (inf/NaN path); `SA SN SR SNaN`; `dst == src` aliasing | `perpendicular_vector` | [x] |
| 38 | `MakeNormalVectors` | `SA SN SR S0 SNaN`; `right`/`up` aliasing `forward` (the C code reads `forward` after writing `right`) | `make_normal_vectors` | [x] |
| 39 | `CrossProduct` (w_) | `SA` pairs (all 9 axis pairs), parallel vectors, `SEQ`, `S0`, `SR`, `SNaN`, aliasing | `cross_product` | [x] |
| 40 | `PlaneFromPoints` | non-degenerate triangles (`qtrue`), degenerate (`a==b`, `b==c`, collinear, all equal -> `qfalse`), `SH` (overflow), `SNaN`; checks `plane[3]` written only on success | `plane_from_points` | [x] |
| 41 | `SetPlaneSignbits` | all 8 sign combinations of `normal`, `-0.0`, `SNaN`, and a pre-set `signbits` (overwritten) | `set_plane_signbits` | [x] |
| 42 | `BoxOnPlaneSide` | `type` = 0, 1, 2 (axial) x each of the 3 returns (1, 2, 3) | `box_on_plane_side_axial` | [x] |
| 43 | `BoxOnPlaneSide` | `type` = 3 x `signbits` = 0..7 (all 8 general-case expressions) x random boxes/planes, each producing `sides` 0, 1, 2 and 3 | `box_on_plane_side_general` | [x] |
| 44 | `BoxOnPlaneSide` | `type` = 4..255 (invalid) x `signbits` = 0..255 (incl. the `default:` label) | `box_on_plane_side_type_out_of_range`, `box_on_plane_side_invalid_signbits` | [x] |
| 45 | `vectoangles` | `x==0&&y==0` with `z>0` / `z<=0` / `z==-0`; `x==0,y>0`; `x==0,y<0`; all four quadrants of `atan2`; `yaw<0` and `pitch<0` fix-ups; `SH ST SI SNaN`; random | `vectoangles` | [x] |
| 46 | `RotatePointAroundVector` | `degrees` = 0, ±90, ±180, 360, huge, `SNaN SI`; `dir` = `SA S0 SN SR`; `point` = `S0 SR SH`; `dst` aliasing `dir` or `point` | `rotate_point_around_vector` | [x] |
| 47 | `RotateAroundDirection` | `yaw == 0` (skips rotation) and `yaw != 0`; `axis[0]` = `SA S0 SN SR`; full 3x3 output | `rotate_around_direction` | [x] |
| 48 | `DirToByte` | `dir` = each of the 162 `bytedirs` entries (must return that index), negated entries, `S0` (no `d > bestd`, returns 0), `SR SNaN SI`, 20000 random | `dir_to_byte` | [x] |
| 49 | `ByteToDir` | `b` = 0..161 (every entry), `-1`, `162`, `INT_MIN`, `INT_MAX` | `byte_to_dir` | [x] |
| 50 | `VectorCompare` (w_) | equal, differing in each single component, `0` vs `-0` (compares equal), NaN vs itself (compares unequal) | `vector_compare` | [x] |
| 51 | `VectorLength`, `VectorLengthSquared`, `Distance`, `DistanceSquared` (w_) | `S0 SA SN SR SH`(overflow) `ST`(underflow) `SI SNaN`, `SEQ` | `lengths_and_distances` | [x] |
| 52 | `VectorNormalizeFast` (w_) | `S0` (`Q_rsqrt(0)`), `SA SN SR SH ST SI SNaN` -- the exact path `main.c` uses | `vector_normalize_fast` | [x] |
| 53 | `VectorInverse`, `VectorNegate` (w_) | `S0 S-0` (sign flips), `SI SNaN` (sign bit of NaN), random | `inverse_and_negate` | [x] |
| 54 | `SnapVector` (w_) | in-range, exactly `±2^31`, out of range, NaN, denormal (truncation toward zero) | `snap_vector` | [x] |
| 55 | `DotProduct`, `VectorSubtract`, `VectorAdd`, `VectorCopy`, `VectorScale`, `VectorMA`, `VectorClear`, `VectorSet`, `Vector4Copy`, `MAKERGB`, `MAKERGBA` (w_) | macro forms vs. the `_`-prefixed function forms on identical inputs, 20000 random each | `macro_forms` | [x] |
| 56 | `SQRTFAST`, `IS_NAN`, `Square` (w_) | `S0 SA SR SH ST SI SNaN`, every NaN exponent pattern for `IS_NAN` (incl. `inf`, which `IS_NAN` also reports) | `sqrtfast_isnan_square` | [x] |
| 57 | `DEG2RAD`, `RAD2DEG`, `ANGLE2SHORT`, `SHORT2ANGLE`, `ColorIndex` (w_) | `f64` results; the `(int)` overflow of `ANGLE2SHORT`; `SHORT2ANGLE` over `INT_MIN..INT_MAX`; `ColorIndex` over all 256 chars and negative ints | `deg_rad_and_short_angles` | [x] |
| 58 | `PlaneTypeForNormal` (w_) | normal `{1,..}`, `{_,1,_}`, `{_,_,1}`, none (`PLANE_NON_AXIAL`), `1.0` vs `0.99999994`, `SNaN` | `plane_type_for_normal` | [x] |
| 59 | data symbols | byte-for-byte compare of `vec3_origin`, `axisDefault`, `bytedirs` (162x3), `g_color_table` (8x4) and the 11 `colorXxx` vec4s in both `.so`s | `data_symbols_match` | [x] |
| 60 | struct layout | `sizeof(cplane_t)` and the offsets of all five members, `sizeof(vec_t)`, `sizeof(qboolean)`, `NUMVERTEXNORMALS`, `nanmask`, `PITCH/YAW/ROLL`, `PLANE_*`, `qfalse/qtrue`, `M_PI` | `layout_and_constants` | [x] |
| 61 | `main` (executable) | 3 valid numeric args: `S0 SA SN SR SH ST SI SNaN` spellings, plus hex/exponent/whitespace/garbage forms; stdout, stderr and exit status compared byte-for-byte | `driver_valid_arguments` | [x] |
| 62 | `main` (executable) | `argc` = 1, 2, 3, 5, 6 (the `!= 4` branch) | `driver_wrong_argc` | [x] |
| 63 | `AngleVectors`, `AnglesToAxis` | the full 18x18x18 cross product of NaN / -NaN / ±inf / huge / tiny / plain angles, comparing the NaN **payloads** bit for bit (this is the one entry point where gcc's folding of `-1*x` into a sign flip is observable -- see NOTES.md Deviation 1) | `angle_vectors_nan_payloads_are_exact`, `angles_to_axis_nan_payloads_are_exact` | [x] |
| 64 | `Q_rsqrt`, `Q_fabs`, `w_SQRTFAST`, `w_Square`, `_VectorCopy` | every NaN payload class incl. signalling NaNs and both signs, 2000 random payloads: single-NaN propagation must be payload-exact | `single_nan_payloads_are_exact` | [x] |
| 65 | `MakeNormalVectors`, `ProjectPointOnPlane`, `LerpAngle` | the three shapes where two *different* NaN patterns provably meet in one expression: both sides must return a NaN in the same lanes and agree bit-for-bit on every non-NaN lane | `documented_nan_payload_divergences_are_nan_on_both_sides` | [x] |
| 66 | `Q_IsColorString` (w_) | `NULL`, empty string, `"^"` (escape at the end), `"^^"` (escaped escape), every one of the 256 bytes after the escape, leading/trailing junk | `q_is_color_string` | [x] |
| 67 | `random`, `crandom` (w_) | both expand to libc `rand()`, whose state is process global: 8 seeds x 200-step sequences, compared after `srand(seed)` on each side | `random_and_crandom_macros` | [x] |
