# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table. There is therefore one effective
feature combination:

| # | Cargo feature combination | CMake configuration | [x] |
|---|---------------------------|---------------------|-----|
| 1 | empty set (`--no-default-features`) | default; `POSITION_INDEPENDENT_CODE=ON`, host `idppc=0`, `Q3_VM` unset | [x] |

## Runtime Configurations

Rows are derived from every exported `q_math.c` entry point and its active
`if`, loop, `switch`, nullable-output, and input-shape branches. Error-only
branches are tracked in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | exported globals | all 16 globals, including all 162 `bytedirs` vectors | [x] |
| 2 | `Q_rand` | arbitrary signed 32-bit seed, including wraparound | [x] |
| 3 | `Q_random` | arbitrary signed 32-bit seed | [x] |
| 4 | `Q_crandom` | arbitrary signed 32-bit seed | [x] |
| 5 | `ClampChar` | `-128 <= i <= 127` | [x] |
| 6 | `ClampShort` | `-32768 <= i <= 32767` | [x] |
| 7 | `DirToByte` | non-null direction whose dot products never exceed initial `bestd=0` | [x] |
| 8 | `DirToByte` | non-null direction selecting a positive best dot product among 162 entries | [x] |
| 9 | `ByteToDir` | boundary/inner valid index `0 <= b < 162` | [x] |
| 10 | `ColorBytes3` | three finite channels in byte-producing range | [x] |
| 11 | `ColorBytes4` | four finite channels in byte-producing range | [x] |
| 12 | `NormalizeColor` | `in[0]` remains nonzero maximum | [x] |
| 13 | `NormalizeColor` | `in[1] > in[0]` and remains maximum | [x] |
| 14 | `NormalizeColor` | `in[2]` exceeds the prior maximum | [x] |
| 15 | `NormalizeColor` | selected maximum is exactly zero | [x] |
| 16 | `PlaneFromPoints` | nondegenerate triangle | [x] |
| 17 | `RotatePointAroundVector` | normalized direction, arbitrary point and degrees | [x] |
| 18 | `RotateAroundDirection` | `yaw == 0` | [x] |
| 19 | `RotateAroundDirection` | `yaw != 0` | [x] |
| 20 | `vectoangles` | `x == 0 && y == 0 && z > 0` | [x] |
| 21 | `vectoangles` | `x == 0 && y == 0 && z <= 0` | [x] |
| 22 | `vectoangles` | `x != 0`, computed yaw and pitch both nonnegative | [x] |
| 23 | `vectoangles` | `x != 0`, computed yaw negative and adjusted by `+360` | [x] |
| 24 | `vectoangles` | `x != 0`, computed pitch negative and adjusted by `+360` | [x] |
| 25 | `vectoangles` | `x == 0 && y > 0` | [x] |
| 26 | `vectoangles` | `x == 0 && y < 0` | [x] |
| 27 | `AnglesToAxis` | arbitrary three-angle input | [x] |
| 28 | `AxisClear` | writable 3x3 axis | [x] |
| 29 | `AxisCopy` | arbitrary 3x3 axis | [x] |
| 30 | `ProjectPointOnPlane` | nonzero normal and arbitrary point | [x] |
| 31 | `MakeNormalVectors` | normalized forward vector | [x] |
| 32 | `VectorRotate` | arbitrary vector and 3x3 matrix | [x] |
| 33 | `Q_rsqrt` | positive finite input | [x] |
| 34 | `Q_rsqrt` | zero, negative, infinity, and NaN bit shapes | [x] |
| 35 | `Q_fabs` | positive/positive-zero bit shape | [x] |
| 36 | `Q_fabs` | negative/negative-zero bit shape | [x] |
| 37 | `LerpAngle` | `to - from > 180` | [x] |
| 38 | `LerpAngle` | `to - from < -180` | [x] |
| 39 | `LerpAngle` | `-180 <= to - from <= 180` | [x] |
| 40 | `AngleSubtract` | initial delta `> 180`, including multiple loop iterations | [x] |
| 41 | `AngleSubtract` | initial delta `< -180`, including multiple loop iterations | [x] |
| 42 | `AngleSubtract` | initial delta in `[-180, 180]` | [x] |
| 43 | `AnglesSubtract` | arbitrary three-angle vectors covering all scalar branches | [x] |
| 44 | `AngleMod` | arbitrary finite angle within defined C float-to-int conversion range | [x] |
| 45 | `AngleNormalize360` | arbitrary finite angle within defined C float-to-int conversion range | [x] |
| 46 | `AngleNormalize180` | normalized intermediate `<= 180` | [x] |
| 47 | `AngleNormalize180` | normalized intermediate `> 180` | [x] |
| 48 | `AngleDelta` | arbitrary finite pair within defined conversion range | [x] |
| 49 | `SetPlaneSignbits` | normal signs `+++` -> `signbits=0` | [x] |
| 50 | `SetPlaneSignbits` | normal signs `-++` -> `signbits=1` | [x] |
| 51 | `SetPlaneSignbits` | normal signs `+-+` -> `signbits=2` | [x] |
| 52 | `SetPlaneSignbits` | normal signs `--+` -> `signbits=3` | [x] |
| 53 | `SetPlaneSignbits` | normal signs `++-` -> `signbits=4` | [x] |
| 54 | `SetPlaneSignbits` | normal signs `-+-` -> `signbits=5` | [x] |
| 55 | `SetPlaneSignbits` | normal signs `+--` -> `signbits=6` | [x] |
| 56 | `SetPlaneSignbits` | normal signs `---` -> `signbits=7` | [x] |
| 57 | `BoxOnPlaneSide` | axial `type=0`, `dist <= emins[0]` | [x] |
| 58 | `BoxOnPlaneSide` | axial `type=0`, `dist >= emaxs[0]` | [x] |
| 59 | `BoxOnPlaneSide` | axial `type=0`, `emins[0] < dist < emaxs[0]` | [x] |
| 60 | `BoxOnPlaneSide` | axial `type=1`, `dist <= emins[1]` | [x] |
| 61 | `BoxOnPlaneSide` | axial `type=1`, `dist >= emaxs[1]` | [x] |
| 62 | `BoxOnPlaneSide` | axial `type=1`, `emins[1] < dist < emaxs[1]` | [x] |
| 63 | `BoxOnPlaneSide` | axial `type=2`, `dist <= emins[2]` | [x] |
| 64 | `BoxOnPlaneSide` | axial `type=2`, `dist >= emaxs[2]` | [x] |
| 65 | `BoxOnPlaneSide` | axial `type=2`, `emins[2] < dist < emaxs[2]` | [x] |
| 66 | `BoxOnPlaneSide` | nonaxial `signbits=0`, front/outside | [x] |
| 67 | `BoxOnPlaneSide` | nonaxial `signbits=0`, back/outside | [x] |
| 68 | `BoxOnPlaneSide` | nonaxial `signbits=0`, straddling | [x] |
| 69 | `BoxOnPlaneSide` | nonaxial `signbits=1`, front/outside | [x] |
| 70 | `BoxOnPlaneSide` | nonaxial `signbits=1`, back/outside | [x] |
| 71 | `BoxOnPlaneSide` | nonaxial `signbits=1`, straddling | [x] |
| 72 | `BoxOnPlaneSide` | nonaxial `signbits=2`, front/outside | [x] |
| 73 | `BoxOnPlaneSide` | nonaxial `signbits=2`, back/outside | [x] |
| 74 | `BoxOnPlaneSide` | nonaxial `signbits=2`, straddling | [x] |
| 75 | `BoxOnPlaneSide` | nonaxial `signbits=3`, front/outside | [x] |
| 76 | `BoxOnPlaneSide` | nonaxial `signbits=3`, back/outside | [x] |
| 77 | `BoxOnPlaneSide` | nonaxial `signbits=3`, straddling | [x] |
| 78 | `BoxOnPlaneSide` | nonaxial `signbits=4`, front/outside | [x] |
| 79 | `BoxOnPlaneSide` | nonaxial `signbits=4`, back/outside | [x] |
| 80 | `BoxOnPlaneSide` | nonaxial `signbits=4`, straddling | [x] |
| 81 | `BoxOnPlaneSide` | nonaxial `signbits=5`, front/outside | [x] |
| 82 | `BoxOnPlaneSide` | nonaxial `signbits=5`, back/outside | [x] |
| 83 | `BoxOnPlaneSide` | nonaxial `signbits=5`, straddling | [x] |
| 84 | `BoxOnPlaneSide` | nonaxial `signbits=6`, front/outside | [x] |
| 85 | `BoxOnPlaneSide` | nonaxial `signbits=6`, back/outside | [x] |
| 86 | `BoxOnPlaneSide` | nonaxial `signbits=6`, straddling | [x] |
| 87 | `BoxOnPlaneSide` | nonaxial `signbits=7`, front/outside | [x] |
| 88 | `BoxOnPlaneSide` | nonaxial `signbits=7`, back/outside | [x] |
| 89 | `BoxOnPlaneSide` | nonaxial `signbits=7`, straddling | [x] |
| 90 | `BoxOnPlaneSide` | nonaxial out-of-range `signbits` takes `default` arm | [x] |
| 91 | `RadiusFromBounds` | arbitrary ordered bounds with each corner selected by larger absolute magnitude | [x] |
| 92 | `ClearBounds` | arbitrary prior writable bounds | [x] |
| 93 | `AddPointToBounds` | per-axis relation `(below, below, below)` | [x] |
| 94 | `AddPointToBounds` | per-axis relation `(below, below, inside)` | [x] |
| 95 | `AddPointToBounds` | per-axis relation `(below, below, above)` | [x] |
| 96 | `AddPointToBounds` | per-axis relation `(below, inside, below)` | [x] |
| 97 | `AddPointToBounds` | per-axis relation `(below, inside, inside)` | [x] |
| 98 | `AddPointToBounds` | per-axis relation `(below, inside, above)` | [x] |
| 99 | `AddPointToBounds` | per-axis relation `(below, above, below)` | [x] |
| 100 | `AddPointToBounds` | per-axis relation `(below, above, inside)` | [x] |
| 101 | `AddPointToBounds` | per-axis relation `(below, above, above)` | [x] |
| 102 | `AddPointToBounds` | per-axis relation `(inside, below, below)` | [x] |
| 103 | `AddPointToBounds` | per-axis relation `(inside, below, inside)` | [x] |
| 104 | `AddPointToBounds` | per-axis relation `(inside, below, above)` | [x] |
| 105 | `AddPointToBounds` | per-axis relation `(inside, inside, below)` | [x] |
| 106 | `AddPointToBounds` | per-axis relation `(inside, inside, inside)` | [x] |
| 107 | `AddPointToBounds` | per-axis relation `(inside, inside, above)` | [x] |
| 108 | `AddPointToBounds` | per-axis relation `(inside, above, below)` | [x] |
| 109 | `AddPointToBounds` | per-axis relation `(inside, above, inside)` | [x] |
| 110 | `AddPointToBounds` | per-axis relation `(inside, above, above)` | [x] |
| 111 | `AddPointToBounds` | per-axis relation `(above, below, below)` | [x] |
| 112 | `AddPointToBounds` | per-axis relation `(above, below, inside)` | [x] |
| 113 | `AddPointToBounds` | per-axis relation `(above, below, above)` | [x] |
| 114 | `AddPointToBounds` | per-axis relation `(above, inside, below)` | [x] |
| 115 | `AddPointToBounds` | per-axis relation `(above, inside, inside)` | [x] |
| 116 | `AddPointToBounds` | per-axis relation `(above, inside, above)` | [x] |
| 117 | `AddPointToBounds` | per-axis relation `(above, above, below)` | [x] |
| 118 | `AddPointToBounds` | per-axis relation `(above, above, inside)` | [x] |
| 119 | `AddPointToBounds` | per-axis relation `(above, above, above)` | [x] |
| 120 | `VectorNormalize` | zero vector | [x] |
| 121 | `VectorNormalize` | nonzero vector | [x] |
| 122 | `VectorNormalize2` | zero vector | [x] |
| 123 | `VectorNormalize2` | nonzero vector | [x] |
| 124 | `_VectorMA` | arbitrary vectors and scale | [x] |
| 125 | `_DotProduct` | arbitrary vectors | [x] |
| 126 | `_VectorSubtract` | arbitrary vectors | [x] |
| 127 | `_VectorAdd` | arbitrary vectors | [x] |
| 128 | `_VectorCopy` | arbitrary vector | [x] |
| 129 | `_VectorScale` | arbitrary vector and scale | [x] |
| 130 | `Vector4Scale` | arbitrary four-vector and scale | [x] |
| 131 | `Q_log2` | `val == 0` | [x] |
| 132 | `Q_log2` | `val == 1` | [x] |
| 133 | `Q_log2` | positive `val > 1`, including powers and non-powers of two | [x] |
| 134 | `MatrixMultiply` | arbitrary 3x3 matrices | [x] |
| 135 | `AngleVectors` | outputs `(forward=NULL, right=NULL, up=NULL)` | [x] |
| 136 | `AngleVectors` | outputs `(forward=set, right=NULL, up=NULL)` | [x] |
| 137 | `AngleVectors` | outputs `(forward=NULL, right=set, up=NULL)` | [x] |
| 138 | `AngleVectors` | outputs `(forward=NULL, right=NULL, up=set)` | [x] |
| 139 | `AngleVectors` | outputs `(forward=set, right=set, up=NULL)` | [x] |
| 140 | `AngleVectors` | outputs `(forward=set, right=NULL, up=set)` | [x] |
| 141 | `AngleVectors` | outputs `(forward=NULL, right=set, up=set)` | [x] |
| 142 | `AngleVectors` | outputs `(forward=set, right=set, up=set)` | [x] |
| 143 | `PerpendicularVector` | axis 0 has uniquely smallest source magnitude | [x] |
| 144 | `PerpendicularVector` | axis 1 has uniquely smallest source magnitude | [x] |
| 145 | `PerpendicularVector` | axis 2 has uniquely smallest source magnitude | [x] |
| 146 | `PerpendicularVector` | tied smallest magnitudes exercise strict `<` first-winner behavior | [x] |

All rows pass through both shared objects with fixed-seed differential inputs.
