# Configuration surface

Rows come from every dynamic entry point and each `if`, `switch`, enum case,
count shape, optional pointer, cache mode, and numeric boundary in
`../c_src/src/lib.c`. Randomized rows include finite values, zeros, signed
zeros, and representative IEEE-754 infinities/NaNs where the operation has
defined C behavior.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---:|---|---|:---:|
| 1 | `c2V` | Arbitrary scalar components | [x] |
| 2 | `c2Mulvs` | Arbitrary vector and scalar | [x] |
| 3 | `c2Maxv` | All four independent x/y `a > b` selection combinations, including equal values | [x] |
| 4 | `c2Minv` | All four independent x/y `a < b` selection combinations, including equal values | [x] |
| 5 | `c2Clampv` | Components below, inside, and above `[lo, hi]`, including reversed bounds | [x] |
| 6 | `c2Sub` | Arbitrary vectors | [x] |
| 7 | `c2Dot` | Arbitrary vectors, including cancellation and zero | [x] |
| 8 | `c2RotIdentity` | No inputs | [x] |
| 9 | `c2xIdentity` | No inputs | [x] |
| 10 | `c2BBVerts` | Ordinary and reversed AABB extents | [x] |
| 11 | `c2MakeProxy` | Circle: one vertex and radius | [x] |
| 12 | `c2MakeProxy` | AABB: four generated vertices and zero radius | [x] |
| 13 | `c2MakeProxy` | Capsule: two vertices and radius | [x] |
| 14 | `c2MakeProxy` | Out-of-range enum: no switch arm and unchanged proxy | [x] |
| 15 | `c2Len` | Nonzero, zero, large, and non-finite vectors | [x] |
| 16 | `c2Det2` | Positive, negative, and zero determinant | [x] |
| 17 | `c2GJKSimplexMetric` | `count == 1` or any default count | [x] |
| 18 | `c2GJKSimplexMetric` | `count == 2`: segment length | [x] |
| 19 | `c2GJKSimplexMetric` | `count == 3`: signed triangle determinant | [x] |
| 20 | `c2Mulrv` | Arbitrary rotation coefficients and vector | [x] |
| 21 | `c2Add` | Arbitrary vectors | [x] |
| 22 | `c2Mulxv` | Arbitrary rotation, translation, and vector | [x] |
| 23 | `c22` | `v <= 0`: reduce to vertex A | [x] |
| 24 | `c22` | `v > 0 && u <= 0`: reduce to vertex B | [x] |
| 25 | `c22` | `v > 0 && u > 0`: retain edge AB | [x] |
| 26 | `c23` | `vAB <= 0 && uCA <= 0`: vertex A region | [x] |
| 27 | `c23` | `uAB <= 0 && vBC <= 0`: vertex B region | [x] |
| 28 | `c23` | `uBC <= 0 && vCA <= 0`: vertex C region | [x] |
| 29 | `c23` | `uAB > 0 && vAB > 0 && wABC <= 0`: edge AB region | [x] |
| 30 | `c23` | `uBC > 0 && vBC > 0 && uABC <= 0`: edge BC region | [x] |
| 31 | `c23` | `uCA > 0 && vCA > 0 && vABC <= 0`: edge CA region | [x] |
| 32 | `c23` | Remaining condition: triangle ABC region | [x] |
| 33 | `c2Neg` | Arbitrary vector | [x] |
| 34 | `c2Skew` | Arbitrary vector | [x] |
| 35 | `c2CCW90` | Arbitrary vector | [x] |
| 36 | `c2D` | `count == 1` | [x] |
| 37 | `c2D` | `count == 2` and determinant `> 0` | [x] |
| 38 | `c2D` | `count == 2` and determinant `<= 0` | [x] |
| 39 | `c2D` | `count == 3` or any default count | [x] |
| 40 | `c2Support` | One vertex | [x] |
| 41 | `c2Support` | Many vertices with later strict maxima | [x] |
| 42 | `c2Support` | Many vertices with tied maxima; first maximum wins | [x] |
| 43 | `c2Witness` | `count == 1`: direct witnesses | [x] |
| 44 | `c2Witness` | `count == 2`: weighted edge witnesses | [x] |
| 45 | `c2Witness` | `count == 3`: weighted triangle witnesses | [x] |
| 46 | `c2Witness` | Default count: both witnesses become zero | [x] |
| 47 | `c2Div` | Finite nonzero divisor | [x] |
| 48 | `c2Div` | Positive/negative zero divisor and IEEE-754 propagation | [x] |
| 49 | `c2Norm` | Nonzero vector | [x] |
| 50 | `c2Norm` | Zero vector | [x] |
| 51 | `c2L` | `count == 1` | [x] |
| 52 | `c2L` | `count == 2`: weighted edge point | [x] |
| 53 | `c2L` | Default count: zero vector | [x] |
| 54 | `c2MulrvT` | Arbitrary rotation coefficients and vector | [x] |
| 55 | `c2GJK` | Circle vs circle; randomized transforms, radius modes, fresh/warm cache, optional outputs | [x] |
| 56 | `c2GJK` | Circle vs AABB; randomized transforms, radius modes, fresh/warm cache, optional outputs | [x] |
| 57 | `c2GJK` | Circle vs capsule; randomized transforms, radius modes, fresh/warm cache, optional outputs | [x] |
| 58 | `c2GJK` | AABB vs circle; randomized transforms, radius modes, fresh/warm cache, optional outputs | [x] |
| 59 | `c2GJK` | AABB vs AABB; randomized transforms, radius modes, fresh/warm cache, optional outputs | [x] |
| 60 | `c2GJK` | AABB vs capsule; randomized transforms, radius modes, fresh/warm cache, optional outputs | [x] |
| 61 | `c2GJK` | Capsule vs circle; randomized transforms, radius modes, fresh/warm cache, optional outputs | [x] |
| 62 | `c2GJK` | Capsule vs AABB; randomized transforms, radius modes, fresh/warm cache, optional outputs | [x] |
| 63 | `c2GJK` | Capsule vs capsule; randomized transforms, radius modes, fresh/warm cache, optional outputs | [x] |
| 64 | `c2GJK` | Transform pointers `(NULL,NULL)`, `(set,NULL)`, `(NULL,set)`, and `(set,set)` | [x] |
| 65 | `c2GJK` | `use_radius == 0` (all nonzero values are equivalent to true) | [x] |
| 66 | `c2GJK` | Radius mode with `dist > rA+rB` and `dist > FLT_EPSILON` | [x] |
| 67 | `c2GJK` | Radius mode with overlap/touching: midpoint witnesses and zero distance | [x] |
| 68 | `c2GJK` | Non-null cache with `count == 0`: default simplex then cache write | [x] |
| 69 | `c2GJK` | Warm cache accepted and read, for resulting simplex counts 1, 2, and 3 | [x] |
| 70 | `c2GJK` | Warm cache metric check rejects stale negative triangle metric | [x] |
| 71 | `c2GJK` | Each of `outA`, `outB`, and `iterations` null independently and all null together | [x] |
| 72 | `c2GJK` | Separated, touching, overlapping, coincident, duplicate-support, and tiny-direction geometries | [x] |
| 73 | `gjk_cache` | `reverse == 0`: AABB is A and capsule is B | [x] |
| 74 | `gjk_cache` | `reverse != 0`: capsule is A and AABB is B | [x] |
| 75 | `gjk_cache` | `a9`/`b9` non-null canaries and null; pointers remain unused | [x] |

Cargo features declared in `Cargo.toml`: **none**. The only feature set is the
empty set (equivalently default and `--no-default-features`).
