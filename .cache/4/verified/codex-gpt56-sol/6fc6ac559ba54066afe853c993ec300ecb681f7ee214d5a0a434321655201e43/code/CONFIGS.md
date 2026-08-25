# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no
`option`, conditional source, compile definition, or preprocessor-controlled
backend. There is exactly one valid feature combination:

| # | Cargo invocation | CMake configuration | [ ] |
|---|------------------|---------------------|-----|
| B1 | `--no-default-features` (empty feature set) | default, PIC enabled | [x] |

## Runtime and Input Configurations

The rows below come from the comparisons, switches, and composed call graph in
`c_src/src/lib.c`. IEEE-754 special values are included where C comparisons
route them differently. Invalid-input rejection rows are in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `c2V` | arbitrary finite and IEEE-754-special `x`, `y`; values are copied | [x] |
| 2 | `c2Maxv` | each component has `a > b` | [x] |
| 3 | `c2Maxv` | each component has `a <= b` | [x] |
| 4 | `c2Maxv` | mixed component comparison outcomes | [x] |
| 5 | `c2Maxv` | either operand is NaN, making the C comparison false | [x] |
| 6 | `c2Minv` | each component has `a < b` | [x] |
| 7 | `c2Minv` | each component has `a >= b` | [x] |
| 8 | `c2Minv` | mixed component comparison outcomes | [x] |
| 9 | `c2Minv` | either operand is NaN, making the C comparison false | [x] |
| 10 | `c2Clampv` | each component of `a` is below `lo` | [x] |
| 11 | `c2Clampv` | each component of `a` is within `[lo, hi]` | [x] |
| 12 | `c2Clampv` | each component of `a` is above `hi` | [x] |
| 13 | `c2Clampv` | components occupy different below/inside/above regions | [x] |
| 14 | `c2Clampv` | NaN in `a`, `lo`, or `hi` exercises false comparisons | [x] |
| 15 | `c2Sub`, `c2Dot` | finite vectors, including zero and mixed signs | [x] |
| 16 | `c2Sub`, `c2Dot` | infinities, NaNs, and signed zero | [x] |
| 17 | `c2CircletoCircle` | separated circles (`d2 > (rA+rB)^2`) | [x] |
| 18 | `c2CircletoCircle` | overlapping circles (`d2 < (rA+rB)^2`) | [x] |
| 19 | `c2CircletoCircle` | externally tangent circles (strict comparison is false) | [x] |
| 20 | `c2CircletoCircle` | negative radii and IEEE-754-special components | [x] |
| 21 | `c2CircletoAABB` | center below the box on both axes | [x] |
| 22 | `c2CircletoAABB` | center inside the box on both axes | [x] |
| 23 | `c2CircletoAABB` | center above the box on both axes | [x] |
| 24 | `c2CircletoAABB` | center lies in mixed below/inside/above axis regions | [x] |
| 25 | `c2CircletoAABB` | distance is below, equal to, and above `r^2` | [x] |
| 26 | `c2CircletoAABB` | negative radius and IEEE-754-special components | [x] |
| 27 | `c2AABBtoAABB` | overlap with positive area | [x] |
| 28 | `c2AABBtoAABB` | boxes touch on an edge or corner (non-strict overlap) | [x] |
| 29 | `c2AABBtoAABB` | separated by `B.max.x < A.min.x` | [x] |
| 30 | `c2AABBtoAABB` | separated by `A.max.x < B.min.x` | [x] |
| 31 | `c2AABBtoAABB` | separated by `B.max.y < A.min.y` | [x] |
| 32 | `c2AABBtoAABB` | separated by `A.max.y < B.min.y` | [x] |
| 33 | `c2AABBtoAABB` | inverted bounds and IEEE-754-special components | [x] |
| 34 | `f2` | circle/circle (`typeA=0`, `typeB=0`) | [x] |
| 35 | `f2` | circle/AABB (`typeA=0`, `typeB=1`) | [x] |
| 36 | `f2` | AABB/circle (`typeA=1`, `typeB=0`) | [x] |
| 37 | `f2` | AABB/AABB (`typeA=1`, `typeB=1`) | [x] |
| 38 | `f3` | `v1 >= 0`, `v2 > 0` | [x] |
| 39 | `f3` | `v1 >= 0`, `v2 < 0`, `v2 != INT_MIN`, with zero/nonzero remainder | [x] |
| 40 | `f3` | `v1 >= 0`, `v2 == INT_MIN` | [x] |
| 41 | `f3` | `v1 < 0`, `v1 != INT_MIN`, `v2 > 0`, with zero/nonzero remainder | [x] |
| 42 | `f3` | `v1 < 0`, `v1 != INT_MIN`, `v2 < 0`, `v2 != INT_MIN` | [x] |
| 43 | `f3` | `v1 < 0`, `v1 != INT_MIN`, `v2 == INT_MIN` | [x] |
| 44 | `f3` | `v1 == INT_MIN`, `v2 > 0` | [x] |
| 45 | `f3` | `v1 == INT_MIN`, `v2 < 0`, `v2 != INT_MIN` | [x] |
| 46 | `f3` | `v1 == INT_MIN`, `v2 == INT_MIN` | [x] |
| 47 | `f4` | arbitrary two-word RNG state, including all-zero and integer boundaries | [x] |
| 48 | `f5` | arbitrary 32-bit input; low 16 bits are reversed and high 16 bits discarded | [x] |
| 49 | `f7` | `channels == 2`, `bitdepth == 32` | [x] |
| 50 | `f7` | `channels == 2`, `bitdepth != 32` | [x] |
| 51 | `f7` | `channels != 2`, `bitdepth == 32` | [x] |
| 52 | `f7` | `channels != 2`, `bitdepth != 32`; include zero and wrapping operands | [x] |
| 53 | `f9` | nondegenerate basis (nonzero denominator) | [x] |
| 54 | `f9` | collinear/coincident basis (zero denominator) | [x] |
| 55 | `f9` | IEEE-754-special coordinates | [x] |
| 56 | `f10` | positive/negative signed zero (`exponent=0`, `mantissa=0`) | [x] |
| 57 | `f10` | positive/negative subnormal (`exponent=0`, `mantissa!=0`) | [x] |
| 58 | `f10` | positive/negative normal (`exponent=1..30`) | [x] |
| 59 | `f10` | positive/negative infinity (`exponent=31`, `mantissa=0`) | [x] |
| 60 | `f10` | positive/negative NaN payload (`exponent=31`, `mantissa!=0`) | [x] |
| 61 | `f11` | `s == 0`, all hue/lightness values | [x] |
| 62 | `f11` | `s != 0`, `0 <= h < 60` | [x] |
| 63 | `f11` | `s != 0`, `60 <= h < 120` | [x] |
| 64 | `f11` | `s != 0`, `h < 0`; unusual third branch | [x] |
| 65 | `f11` | `s != 0`, `120 <= h < 180`; falls through to gray | [x] |
| 66 | `f11` | `s != 0`, `180 <= h < 240` | [x] |
| 67 | `f11` | `s != 0`, `240 <= h < 300` | [x] |
| 68 | `f11` | `s != 0`, `300 <= h < 360` | [x] |
| 69 | `f11` | `s != 0`, `h >= 360` or NaN; falls through to gray | [x] |
| 70 | `f12` | `s == 0`, all hue/value values | [x] |
| 71 | `f12` | `s != 0`, `floor(h/60)` is `0` | [x] |
| 72 | `f12` | `s != 0`, `floor(h/60)` is `1` | [x] |
| 73 | `f12` | `s != 0`, `floor(h/60)` is `2` | [x] |
| 74 | `f12` | `s != 0`, `floor(h/60)` is `3` | [x] |
| 75 | `f12` | `s != 0`, `floor(h/60)` is `4` | [x] |
| 76 | `f12` | `s != 0`, integer selector is outside `0..=4`; default branch | [x] |
| 77 | `f12` | `s != 0`, `floor(h/60)` is NaN, infinite, or outside C `int` range | [x] |
| 78 | `f13` | `delta == 0` (equal channels) | [x] |
| 79 | `f13` | `max == 0` with unequal nonpositive channels | [x] |
| 80 | `f13` | `r == max`, computed hue nonnegative | [x] |
| 81 | `f13` | `r == max`, computed hue negative and adjusted by `+360` | [x] |
| 82 | `f13` | `g == max` | [x] |
| 83 | `f13` | `b == max` | [x] |
| 84 | `f13` | IEEE-754-special channels route C ternary comparisons | [x] |
| 85 | `agglom` | finite inputs spanning all composed low-level branches | [x] |
| 86 | `agglom` | integer boundaries, arithmetic wrapping inputs, and half-float classes | [x] |
| 87 | `agglom` | NaN/infinity components exercise each `isnan` accumulation guard | [x] |
