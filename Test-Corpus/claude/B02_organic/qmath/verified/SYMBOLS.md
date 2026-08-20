# SYMBOLS.md -- symbol parity between the C and the Rust shared objects

Generated mechanically with `nm -D --defined-only` on both shared objects; see
`./check_symbols.sh`, which regenerates and re-verifies the diff.

## How the two shared objects are built

`c_src/CMakeLists.txt` builds an **executable** from `src/q_math.c` +
`src/main.c` (`-Iinc -Isrc`, `-lm`).  For the differential tests the very same
two translation units are linked into a shared object instead
(`./build_c.sh` -> `cbuild/libcdriver.so`); nothing in `c_src/` is modified.
That is the symbol-parity reference, and it is why `main` appears below.

The Rust side is `target/debug/libdriver.so`, the `cdylib` of this crate.  The
`driver` binary is `#![no_main]` and reuses the exported `main`, exactly like the
C executable links `main.c`'s `main`.

## Result

| | count |
|---|---|
| symbols exported by `cbuild/libcdriver.so` | 63 |
| of those, exported by `target/debug/libdriver.so` | **63** |
| **missing from the Rust `.so`** | **0** |
| undefined (imported) non-libc symbols in the Rust `.so` | **0** |

`nm -D --undefined-only` on the Rust `.so` lists only libc/glibc and libgcc
imports (`memcpy`, `sin`, `cos`, `atan2`, `sqrt`, `write`, `malloc`,
`__libc_start_main`, ... ) plus the usual weak `_ITM_*`/`__gmon_start__` stubs;
the C `.so` likewise imports only `atan2 atof cos exit fprintf memset printf sin
sqrt stderr`.  No symbol of the library itself is left undefined on either side.

## Table 1 -- every symbol of `cbuild/libcdriver.so` (`q_math.c` + `main.c`)

| # | symbol | C `nm` type | kind | Rust `nm` type | present in Rust `.so` |
|---|--------|-------------|------|----------------|-----------------------|
| 1 | `AddPointToBounds` | T | function (text) | `T` | yes |
| 2 | `AngleDelta` | T | function (text) | `T` | yes |
| 3 | `AngleMod` | T | function (text) | `T` | yes |
| 4 | `AngleNormalize180` | T | function (text) | `T` | yes |
| 5 | `AngleNormalize360` | T | function (text) | `T` | yes |
| 6 | `AngleSubtract` | T | function (text) | `T` | yes |
| 7 | `AngleVectors` | T | function (text) | `T` | yes |
| 8 | `AnglesSubtract` | T | function (text) | `T` | yes |
| 9 | `AnglesToAxis` | T | function (text) | `T` | yes |
| 10 | `AxisClear` | T | function (text) | `T` | yes |
| 11 | `AxisCopy` | T | function (text) | `T` | yes |
| 12 | `BoxOnPlaneSide` | T | function (text) | `T` | yes |
| 13 | `ByteToDir` | T | function (text) | `T` | yes |
| 14 | `ClampChar` | T | function (text) | `T` | yes |
| 15 | `ClampShort` | T | function (text) | `T` | yes |
| 16 | `ClearBounds` | T | function (text) | `T` | yes |
| 17 | `ColorBytes3` | T | function (text) | `T` | yes |
| 18 | `ColorBytes4` | T | function (text) | `T` | yes |
| 19 | `DirToByte` | T | function (text) | `T` | yes |
| 20 | `LerpAngle` | T | function (text) | `T` | yes |
| 21 | `MakeNormalVectors` | T | function (text) | `T` | yes |
| 22 | `MatrixMultiply` | T | function (text) | `T` | yes |
| 23 | `NormalizeColor` | T | function (text) | `T` | yes |
| 24 | `PerpendicularVector` | T | function (text) | `T` | yes |
| 25 | `PlaneFromPoints` | T | function (text) | `T` | yes |
| 26 | `ProjectPointOnPlane` | T | function (text) | `T` | yes |
| 27 | `Q_crandom` | T | function (text) | `T` | yes |
| 28 | `Q_fabs` | T | function (text) | `T` | yes |
| 29 | `Q_log2` | T | function (text) | `T` | yes |
| 30 | `Q_rand` | T | function (text) | `T` | yes |
| 31 | `Q_random` | T | function (text) | `T` | yes |
| 32 | `Q_rsqrt` | T | function (text) | `T` | yes |
| 33 | `RadiusFromBounds` | T | function (text) | `T` | yes |
| 34 | `RotateAroundDirection` | T | function (text) | `T` | yes |
| 35 | `RotatePointAroundVector` | T | function (text) | `T` | yes |
| 36 | `SetPlaneSignbits` | T | function (text) | `T` | yes |
| 37 | `Vector4Scale` | T | function (text) | `T` | yes |
| 38 | `VectorNormalize` | T | function (text) | `T` | yes |
| 39 | `VectorNormalize2` | T | function (text) | `T` | yes |
| 40 | `VectorRotate` | T | function (text) | `T` | yes |
| 41 | `_DotProduct` | T | function (text) | `T` | yes |
| 42 | `_VectorAdd` | T | function (text) | `T` | yes |
| 43 | `_VectorCopy` | T | function (text) | `T` | yes |
| 44 | `_VectorMA` | T | function (text) | `T` | yes |
| 45 | `_VectorScale` | T | function (text) | `T` | yes |
| 46 | `_VectorSubtract` | T | function (text) | `T` | yes |
| 47 | `axisDefault` | D | data (.data) | `D` | yes |
| 48 | `bytedirs` | D | data (.data) | `D` | yes |
| 49 | `colorBlack` | D | data (.data) | `D` | yes |
| 50 | `colorBlue` | D | data (.data) | `D` | yes |
| 51 | `colorCyan` | D | data (.data) | `D` | yes |
| 52 | `colorDkGrey` | D | data (.data) | `D` | yes |
| 53 | `colorGreen` | D | data (.data) | `D` | yes |
| 54 | `colorLtGrey` | D | data (.data) | `D` | yes |
| 55 | `colorMagenta` | D | data (.data) | `D` | yes |
| 56 | `colorMdGrey` | D | data (.data) | `D` | yes |
| 57 | `colorRed` | D | data (.data) | `D` | yes |
| 58 | `colorWhite` | D | data (.data) | `D` | yes |
| 59 | `colorYellow` | D | data (.data) | `D` | yes |
| 60 | `g_color_table` | D | data (.data) | `D` | yes |
| 61 | `main` | T | function (text) | `T` | yes |
| 62 | `vec3_origin` | B | data (.bss) | `B` | yes |
| 63 | `vectoangles` | T | function (text) | `T` | yes |

### Not exported by either side (correctly so)

`c_src/inc/q_shared.h` declares many more names, but they either have internal
linkage or are never defined in these two translation units, so they are absent
from **both** shared objects and must stay absent:

* `static ID_INLINE` (internal linkage): `VectorCompare`, `VectorLength`,
  `VectorLengthSquared`, `Distance`, `DistanceSquared`, `VectorNormalizeFast`,
  `VectorInverse`, `CrossProduct`, `BigShort`, `BigLong`, `BigFloat`.
* Function-like macros (no linkage at all): `DotProduct`, `VectorSubtract`,
  `VectorAdd`, `VectorCopy`, `VectorScale`, `VectorMA`, `VectorClear`,
  `VectorNegate`, `VectorSet`, `Vector4Copy`, `SnapVector`, `SQRTFAST`,
  `IS_NAN`, `DEG2RAD`, `RAD2DEG`, `ANGLE2SHORT`, `SHORT2ANGLE`, `ColorIndex`,
  `Square`, `PlaneTypeForNormal`, `MAKERGB`, `MAKERGBA`, `random`, `crandom`,
  `Q_IsColorString`, `Com_Memset`/`Com_Memcpy`/`Snd_Memset` (macro forms).
* Declared but never defined anywhere in `c_src` (the rest of the Quake III
  code base): `ShortSwap`, `LongSwap`, `FloatSwap`, `Q_acos`, `Com_Clamp`, all
  `COM_*`, `Info_*`, `Q_str*`, `Q_is*`, `Hunk_Alloc`, `va`, `Com_Error`,
  `Com_Printf`, `Com_sprintf`, `SkipBracedSection`, `SkipRestOfLine`,
  `Parse[123]DMatrix`, `Snd_Memset`, `Com_Memset`, `Com_Memcpy`.
  Translating them is impossible (no C body exists) and they are not part of
  the built library, so parity is unaffected.

## Table 2 -- `w_*` test hooks

These are NOT part of the library.  `tests/csupport/wrappers.c` (built into
`cbuild/libcwrap.so` together with the unmodified `c_src/src/q_math.c`) gives
external linkage to the macros and `static ID_INLINE` functions of
`q_shared.h` so that the differential suite can reach them; `src/wrappers.rs`
exports the same names from the Rust side.  All 36 match.

| # | symbol | C `nm` type | Rust `nm` type | present in Rust `.so` |
|---|--------|-------------|----------------|-----------------------|
| 1 | `w_ANGLE2SHORT` | T | `T` | yes |
| 2 | `w_ColorIndex` | T | `T` | yes |
| 3 | `w_CrossProduct` | T | `T` | yes |
| 4 | `w_DEG2RAD` | T | `T` | yes |
| 5 | `w_Distance` | T | `T` | yes |
| 6 | `w_DistanceSquared` | T | `T` | yes |
| 7 | `w_DotProduct` | T | `T` | yes |
| 8 | `w_IS_NAN` | T | `T` | yes |
| 9 | `w_MAKERGB` | T | `T` | yes |
| 10 | `w_MAKERGBA` | T | `T` | yes |
| 11 | `w_M_PI` | T | `T` | yes |
| 12 | `w_PlaneTypeForNormal` | T | `T` | yes |
| 13 | `w_RAD2DEG` | T | `T` | yes |
| 14 | `w_SHORT2ANGLE` | T | `T` | yes |
| 15 | `w_SQRTFAST` | T | `T` | yes |
| 16 | `w_SnapVector` | T | `T` | yes |
| 17 | `w_Square` | T | `T` | yes |
| 18 | `w_Vector4Copy` | T | `T` | yes |
| 19 | `w_VectorAdd` | T | `T` | yes |
| 20 | `w_VectorClear` | T | `T` | yes |
| 21 | `w_VectorCompare` | T | `T` | yes |
| 22 | `w_VectorCopy` | T | `T` | yes |
| 23 | `w_VectorInverse` | T | `T` | yes |
| 24 | `w_VectorLength` | T | `T` | yes |
| 25 | `w_VectorLengthSquared` | T | `T` | yes |
| 26 | `w_VectorMA` | T | `T` | yes |
| 27 | `w_VectorNegate` | T | `T` | yes |
| 28 | `w_VectorNormalizeFast` | T | `T` | yes |
| 29 | `w_VectorScale` | T | `T` | yes |
| 30 | `w_VectorSet` | T | `T` | yes |
| 31 | `w_VectorSubtract` | T | `T` | yes |
| 32 | `w_angle_indexes` | T | `T` | yes |
| 33 | `w_layout` | T | `T` | yes |

## Symbols exported by the Rust `.so` but not by the C `.so`

None.  `nm -D --defined-only target/debug/libdriver.so` yields exactly
63 + 36 = 99 names and nothing else (no Rust runtime symbols leak out of the
cdylib).
