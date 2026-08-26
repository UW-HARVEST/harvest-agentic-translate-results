# Dynamic Symbol Surface

Derived from:

```sh
nm -D --defined-only c_src/build/libqmath_c.so
```

The C shared object has 62 public definitions: 47 functions and 15 writable
globals. At initial inventory time every row was missing from the Rust binary
because the Rust crate translated only `main.c`, not `q_math.c`.

| # | kind | symbol | Rust parity |
|---|------|--------|-------------|
| 1 | function | `AddPointToBounds` | [x] exported and tested |
| 2 | function | `AngleDelta` | [x] exported and tested |
| 3 | function | `AngleMod` | [x] exported and tested |
| 4 | function | `AngleNormalize180` | [x] exported and tested |
| 5 | function | `AngleNormalize360` | [x] exported and tested |
| 6 | function | `AngleSubtract` | [x] exported and tested |
| 7 | function | `AngleVectors` | [x] exported and tested |
| 8 | function | `AnglesSubtract` | [x] exported and tested |
| 9 | function | `AnglesToAxis` | [x] exported and tested |
| 10 | function | `AxisClear` | [x] exported and tested |
| 11 | function | `AxisCopy` | [x] exported and tested |
| 12 | function | `BoxOnPlaneSide` | [x] exported and tested |
| 13 | function | `ByteToDir` | [x] exported and tested |
| 14 | function | `ClampChar` | [x] exported and tested |
| 15 | function | `ClampShort` | [x] exported and tested |
| 16 | function | `ClearBounds` | [x] exported and tested |
| 17 | function | `ColorBytes3` | [x] exported and tested |
| 18 | function | `ColorBytes4` | [x] exported and tested |
| 19 | function | `DirToByte` | [x] exported and tested |
| 20 | function | `LerpAngle` | [x] exported and tested |
| 21 | function | `MakeNormalVectors` | [x] exported and tested |
| 22 | function | `MatrixMultiply` | [x] exported and tested |
| 23 | function | `NormalizeColor` | [x] exported and tested |
| 24 | function | `PerpendicularVector` | [x] exported and tested |
| 25 | function | `PlaneFromPoints` | [x] exported and tested |
| 26 | function | `ProjectPointOnPlane` | [x] exported and tested |
| 27 | function | `Q_crandom` | [x] exported and tested |
| 28 | function | `Q_fabs` | [x] exported and tested |
| 29 | function | `Q_log2` | [x] exported and tested |
| 30 | function | `Q_rand` | [x] exported and tested |
| 31 | function | `Q_random` | [x] exported and tested |
| 32 | function | `Q_rsqrt` | [x] exported and tested |
| 33 | function | `RadiusFromBounds` | [x] exported and tested |
| 34 | function | `RotateAroundDirection` | [x] exported and tested |
| 35 | function | `RotatePointAroundVector` | [x] exported and tested |
| 36 | function | `SetPlaneSignbits` | [x] exported and tested |
| 37 | function | `Vector4Scale` | [x] exported and tested |
| 38 | function | `VectorNormalize` | [x] exported and tested |
| 39 | function | `VectorNormalize2` | [x] exported and tested |
| 40 | function | `VectorRotate` | [x] exported and tested |
| 41 | function | `_DotProduct` | [x] exported and tested |
| 42 | function | `_VectorAdd` | [x] exported and tested |
| 43 | function | `_VectorCopy` | [x] exported and tested |
| 44 | function | `_VectorMA` | [x] exported and tested |
| 45 | function | `_VectorScale` | [x] exported and tested |
| 46 | function | `_VectorSubtract` | [x] exported and tested |
| 47 | function | `vectoangles` | [x] exported and tested |
| 48 | global | `axisDefault` | [x] exported and tested |
| 49 | global | `bytedirs` | [x] exported and tested |
| 50 | global | `colorBlack` | [x] exported and tested |
| 51 | global | `colorBlue` | [x] exported and tested |
| 52 | global | `colorCyan` | [x] exported and tested |
| 53 | global | `colorDkGrey` | [x] exported and tested |
| 54 | global | `colorGreen` | [x] exported and tested |
| 55 | global | `colorLtGrey` | [x] exported and tested |
| 56 | global | `colorMagenta` | [x] exported and tested |
| 57 | global | `colorMdGrey` | [x] exported and tested |
| 58 | global | `colorRed` | [x] exported and tested |
| 59 | global | `colorWhite` | [x] exported and tested |
| 60 | global | `colorYellow` | [x] exported and tested |
| 61 | global | `g_color_table` | [x] exported and tested |
| 62 | global | `vec3_origin` | [x] exported and tested |

- [x] Final `nm -D` comparison has zero missing C symbols.
- [x] Rust has zero undefined project-library symbols.
