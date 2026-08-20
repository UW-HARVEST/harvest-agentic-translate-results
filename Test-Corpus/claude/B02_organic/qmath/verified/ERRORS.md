# ERRORS.md -- the error / rejection surface of the C library

Derived mechanically from `c_src/src/q_math.c`, `c_src/src/main.c` and
`c_src/inc/q_shared.h` by grepping for *every* early `return`, guard `if`,
`switch` `default:`, loop bound, `assert`, min/max constant and range check, and
by reading what gcc actually emits for the out-of-range conversions.

There are **no** error codes, `errno` values, error enums or `assert()`s active
in this library: the only `assert` in the C source is inside a `/* ... */`
comment in `Q_rsqrt` (q_math.c:510-516) and another commented-out one in
`VectorNormalize2` (q_math.c:826/834), so neither can ever fire.  "Rejection"
here therefore means: an early return with a sentinel value, a branch that
substitutes a default result, a saturating clamp, an out-of-range conversion
that yields the x86 "integer indefinite" value, a non-terminating loop, or
`exit(1)`.

Every row has a differential test that constructs exactly that condition and
asserts the C and the Rust `.so` return the *same* value (not merely "both
failed").  Tests live in `tests/errors.rs` unless noted.

| # | function | trigger (exact invalid input / condition) | expected C result | test | ✔ |
|---|----------|--------------------------------------------|-------------------|------|---|
| 1 | `DirToByte` | `dir == NULL` (q_math.c:186 `if ( !dir ) return 0;`) | returns `0`, writes nothing | `dir_to_byte_null` | [x] |
| 2 | `ByteToDir` | `b < 0` (q_math.c:206) | `dir` = `vec3_origin` = `{0,0,0}`, returns void | `byte_to_dir_out_of_range` | [x] |
| 3 | `ByteToDir` | `b >= NUMVERTEXNORMALS` (162), incl. `INT_MAX` | `dir` = `{0,0,0}` | `byte_to_dir_out_of_range` | [x] |
| 4 | `ByteToDir` | `b == 161` / `b == 162` (last valid / first invalid) | 161 -> `bytedirs[161]`, 162 -> zeros | `byte_to_dir_boundary` | [x] |
| 5 | `NormalizeColor` | `max == 0` after the two `>` tests, i.e. `in[0] <= 0 && in[1] <= 0 && in[2] <= 0` with the max being `+0.0` or `-0.0` (q_math.c:246 `if ( !max )`) | `out` = `{0,0,0}` (`VectorClear`), returns the max (`0.0` or `-0.0`, sign preserved) | `normalize_color_zero_max` | [x] |
| 6 | `ColorBytes3` | any input: byte 3 of the result is **never written** (q_math.c:214-222, `unsigned i;` is uninitialised) | low 24 bits defined, top 8 bits indeterminate | `color_bytes3` (masks 0xffffff) + `color_bytes3_top_byte_is_indeterminate` | [x] |
| 7 | `ColorBytes3`/`ColorBytes4` | component `< 0`, `> 1`, `>= 256`, `inf`, `NaN` -- `(byte)(x*255)` is `cvttss2si` + low byte, so out-of-range/NaN gives `0x80000000 & 0xff == 0` | 0 for NaN/±inf/overflow; wraps mod 256 otherwise | `color_bytes_out_of_range` | [x] |
| 8 | `ClampChar` | `i < -128` (down to `INT_MIN`) | `-128` | `clamp_char_saturation` | [x] |
| 9 | `ClampChar` | `i > 127` (up to `INT_MAX`) | `127` | `clamp_char_saturation` | [x] |
| 10 | `ClampShort` | `i < -32768` (down to `INT_MIN`) | `-32768` | `clamp_short_saturation` | [x] |
| 11 | `ClampShort` | `i > 0x7fff` (up to `INT_MAX`) | `32767` | `clamp_short_saturation` | [x] |
| 12 | `PlaneFromPoints` | degenerate triangle: `VectorNormalize(plane) == 0`, e.g. `a == b == c`, or collinear points (q_math.c:271) | returns `qfalse` (0); `plane[0..2]` left as the zeroed cross product, `plane[3]` **not** written.  Exception: if a cross-product term overflows (coordinates >~ 1.8e19) the difference is `inf - inf` = NaN, `VectorNormalize` returns NaN, `if (length)` is true for NaN and the function returns `qtrue` with an all-NaN plane | `plane_from_points_degenerate`, `plane_from_points` (tests/planes.rs) | [x] |
| 13 | `PlaneFromPoints` | non-degenerate | returns `qtrue` (1) -- the only two possible return values | `plane_from_points_degenerate` | [x] |
| 14 | `VectorNormalize` | `length == 0` (zero vector, or a vector whose squared length underflows to `0`, e.g. `{1e-30,0,0}`) | returns `0.0`, `v` left **unmodified** | `vector_normalize_zero` | [x] |
| 15 | `VectorNormalize2` | `length == 0` | returns `0.0`, `out` = `{0,0,0}` (`VectorClear`) -- note this differs from `VectorNormalize`, which leaves the vector alone | `vector_normalize2_zero` | [x] |
| 16 | `VectorNormalize`/`VectorNormalize2` | `length` is NaN (a NaN or `inf` component) -- `if (length)` is `ucomiss` + `jp`, so NaN counts as **non**-zero and the division is performed | returns NaN, components scaled by `1/NaN` | `vector_normalize_nan_length` | [x] |
| 17 | `BoxOnPlaneSide` | `p->type < 3` axial fast path, `p->dist <= emins[type]` | returns `1` (before any signbits work) | `box_on_plane_side_axial` | [x] |
| 18 | `BoxOnPlaneSide` | axial, `p->dist >= emaxs[type]` | returns `2` | `box_on_plane_side_axial` | [x] |
| 19 | `BoxOnPlaneSide` | axial, neither | returns `3` | `box_on_plane_side_axial` | [x] |
| 20 | `BoxOnPlaneSide` | `p->signbits > 7` (the `default:` label, q_math.c:737) -- reachable because `signbits` is a `byte`, so 8..255 are perfectly legal C inputs | `dist1 = dist2 = 0`, hence `sides = (0 >= dist) | (0 < dist ? 2 : 0)`; e.g. `dist = 0` -> `1`, `dist > 0` -> `2`, `dist < 0` -> `1` | `box_on_plane_side_invalid_signbits` | [x] |
| 21 | `BoxOnPlaneSide` | `p->type == 3..255` with `signbits == 0..7` | takes the general case (`type` only selects the fast path when `< 3`) | `box_on_plane_side_type_out_of_range` | [x] |
| 22 | `BoxOnPlaneSide` | NaN reaching **both** `dist1` and `dist2`: a NaN in `p->dist`, in `p->normal[i]`, or in the *same* slot of `emins` and `emaxs`; then `dist1 >= dist` and `dist2 < dist` are both false | returns `0` -- a value the doc comment ("Returns 1, 2, or 1 + 2") says is impossible.  A NaN in only ONE corner poisons only one of the two sums (each `switch` arm mixes the corners), so the other comparison still fires and the result is 1 or 2, never 3 | `box_on_plane_side_nan` | [x] |
| 23 | `SetPlaneSignbits` | `normal[j]` is `-0.0` or NaN -- `< 0` is false for both | that bit stays clear (`-0.0` is *not* treated as negative) | `set_plane_signbits_edge` | [x] |
| 24 | `AngleMod`, `AngleNormalize360` | `(int)(angle * 182.044...)` out of `int` range (|angle| >~ 1.1796e7) or NaN -> gcc's `cvttsd2si` returns `0x80000000`, `& 65535` -> `0` | returns `0.0` (**not** a wrapped angle) | `angle_normalize_int_overflow` | [x] |
| 25 | `AngleNormalize180` | result of `AngleNormalize360` `> 180.0` | subtracts `360.0` in `double` | `angle_normalize_int_overflow` | [x] |
| 26 | `ANGLE2SHORT` (macro) | `(int)(x*65536/360)` out of range / NaN | `0x80000000 & 65535 == 0` | `angle2short_overflow` (tests/qshared.rs) | [x] |
| 27 | `SnapVector` (macro) | component out of `int` range or NaN -> `(int)` is `cvttss2si` | that component becomes `-2147483648.0f` | `snap_vector_overflow` (tests/qshared.rs) | [x] |
| 28 | `AngleSubtract`, `AnglesSubtract` | \|a1-a2\| so large that `a -= 360` cannot change it: from `2^33 + 1 ulp` upwards half an ulp exceeds 360, so the subtraction rounds straight back to `a` (±inf likewise) | **never terminates** (`while (a > 180)` / `while (a < -180)`), so it cannot be compared; the Rust translation loops identically. Compared up to \|a\| = 2^25 (~93k iterations); the rounding that causes the hang is asserted instead. | documented, not executed (`angle_subtract_hangs_doc`) | [x] |
| 29 | `AngleSubtract` | `a1 - a2` is NaN | both `while` conditions are false -> returns NaN immediately | `angle_subtract_nan` | [x] |
| 30 | `Q_log2` | `val < 0` -- `val >>= 1` is an arithmetic shift, so the value sticks at `-1` and the loop never exits | **never terminates**; the Rust translation loops identically | documented, not executed (`q_log2_negative_hangs_doc`) | [x] |
| 31 | `Q_log2` | `val == 0` | returns `0` | `q_log2_zero` | [x] |
| 32 | `Q_rand` | signed overflow of `69069 * *seed + 1` (UB in C, wraps with gcc) | wraps modulo 2^32; the sequence is bit-identical | `q_rand_overflow` | [x] |
| 33 | `Q_rsqrt` | `number == 0.0` -> `i = 0x5f3759df`, `y` huge; `number < 0` -> nonsense; `number == inf`; `number == NaN` | no guard at all: returns the raw magic-constant result (`0`, `-inf`-ish, `NaN`, ...) exactly as computed | `q_rsqrt_special` | [x] |
| 34 | `Q_rsqrt` | denormal / negative-zero input (`i >> 1` on a sign-bit-set pattern) | bit-exact magic result | `q_rsqrt_all_shapes` (tests/scalar.rs) | [x] |
| 35 | `ProjectPointOnPlane`, `PerpendicularVector`, `RotatePointAroundVector` | zero `normal`/`src` -> `1.0f / 0.0f` = `+inf` -> products become `inf`/NaN | no guard: propagates `inf`/NaN into `dst` | `project_point_on_plane_zero_normal`, `perpendicular_vector_zero` | [x] |
| 36 | `AngleVectors` | `forward == NULL` (q_math.c:959) | that output is skipped, the others are still written | `angle_vectors_null_outputs` | [x] |
| 37 | `AngleVectors` | `right == NULL` (q_math.c:965) | ditto | `angle_vectors_null_outputs` | [x] |
| 38 | `AngleVectors` | `up == NULL` (q_math.c:971) | ditto | `angle_vectors_null_outputs` | [x] |
| 39 | `AngleVectors` | all three NULL | writes nothing at all, returns void | `angle_vectors_null_outputs` | [x] |
| 40 | `RotateAroundDirection` | `yaw == 0.0` or `-0.0` (`if ( yaw )`) | skips the rotation entirely; `axis[1]` stays the raw `PerpendicularVector` result | `rotate_around_direction_zero_yaw` | [x] |
| 41 | `vectoangles` | `value1[0] == 0 && value1[1] == 0` (incl. `-0.0`) | `yaw = 0`; `pitch = 90` if `value1[2] > 0` else `270` (so `+0.0` z gives 270) | `vectoangles_degenerate` | [x] |
| 42 | `vectoangles` | `value1[0] == 0` but `value1[1] != 0` | `yaw = 90` if `value1[1] > 0` else `270` (NaN y -> 270) | `vectoangles_degenerate` | [x] |
| 43 | `vectoangles` | `yaw < 0` / `pitch < 0` after `atan2` | `+= 360` | `vectoangles_degenerate` | [x] |
| 44 | `vectoangles` | NaN components | `==`/`>`/`<` comparisons all false -> `yaw` from `atan2(NaN,NaN)`, `pitch` from `atan2` too | `vectoangles_nan` | [x] |
| 45 | `LerpAngle` | `to - from` NaN (e.g. `inf,inf`) | both `if`s false, `a = from + frac*(to-from)` = NaN | `lerp_angle` (tests/scalar.rs) | [x] |
| 46 | `RadiusFromBounds` | NaN in `mins`/`maxs` -- `a > b ? a : b` with a NaN `a` picks `b` | NaN only if `maxs[i]` is NaN | `radius_from_bounds_nan` | [x] |
| 47 | `AddPointToBounds` | NaN in `v` -- both `<` and `>` false | bounds unchanged | `add_point_to_bounds_nan` | [x] |
| 48 | `main` (main.c:7) | `argc != 4` (0, 1, 2, 3, 5, ...) | `fprintf(stderr, "%s requires 4 inputs\n", argv[0])` and `exit(1)` -- stdout stays empty, exit status 1 | `driver_wrong_argc` (tests/driver_cli.rs) | [x] |
| 49 | `main` | unparseable arguments (`""`, `"abc"`, `"1e999"`, `"nan"`, `"inf"`, `"0x1p3"`, leading blanks, trailing junk) | `atof` returns `0.0` / `inf` / `nan` per C99 `strtod`; no error is reported, exit status 0 | `driver_odd_arguments` (tests/driver_cli.rs) | [x] |
| 50 | every pointer parameter except `DirToByte`'s and `AngleVectors`' outputs | `NULL` | unchecked dereference -> `SIGSEGV`.  Verified out-of-process for `VectorNormalize`, `_DotProduct`, `ByteToDir`, `SetPlaneSignbits`, `BoxOnPlaneSide`, `MatrixMultiply`, `vectoangles`, `AxisClear`: C always `SIGSEGV`; Rust `SIGSEGV` too with `--release`, and `SIGSEGV`/`SIGABRT` in the dev profile where rustc's `ub_checks` catch the null read before the load (see NOTES.md) | `null_pointer_crashes_match` | [x] |

## Generic boundaries also covered

* **Out-of-range "enum" values across FFI.** The library has no `enum`
  parameters, but it has their moral equivalent: `cplane_t.type` and
  `cplane_t.signbits` are `byte` fields whose documented domains are `0..3` and
  `0..7`.  Rows 20/21 pass every one of the 256 values of each.  `qboolean`
  return values are checked to be exactly `0`/`1` (row 12/13).
* **Zero / oversized lengths.** There is no length parameter anywhere (all
  vectors are fixed `vec3_t`/`vec4_t`), so the analogue is the zero vector and
  the `bytedirs` index (rows 2-4, 14, 15).
* **One step past a documented range.** Rows 4 (`b = 162`), 8-11 (`-129`,
  `128`, `-32769`, `32768`), 20 (`signbits = 8`), 21 (`type = 3`), 24
  (`11796480.0`).
* **Denormals, ±0, ±inf, NaN** are in the standard input pool
  (`tests/harness/mod.rs::INTERESTING`) of every valid-path test, so they hit
  every row above from the "valid" side too.
