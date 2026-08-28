# Configuration surface

The crate declares no Cargo features. The only build configuration is the
default/no-feature build. Rows below are derived from every exported function
and every source branch that distinguishes valid runtime options or input
shapes.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C001 | `c2V` | finite values, signed zero, and finite extrema | [x] |
| C002 | `c2Mulvs` | scalar is negative, zero, or positive | [x] |
| C003 | `c2Maxv` | all x/y combinations of `a > b` and `a <= b` | [x] |
| C004 | `c2Minv` | all x/y combinations of `a < b` and `a >= b` | [x] |
| C005 | `c2Clampv` | each coordinate below, within, or above `[lo, hi]` (3x3) | [x] |
| C006 | `c2Sub` | mixed-sign finite vectors | [x] |
| C007 | `c2Dot` | negative, zero, and positive dot products | [x] |
| C008 | `c2Dist` | points on the negative, zero, and positive side of a plane | [x] |
| C009 | `c2PlaneAt` | valid polygon indices `0` and `count-1` | [x] |
| C010 | `c2RotIdentity`, `c2xIdentity` | no-input identity constructors | [x] |
| C011 | `c2BBVerts` | regular, degenerate, and inverted min/max AABBs | [x] |
| C012 | `c2MakeProxy` | circle: one vertex and radius copied | [x] |
| C013 | `c2MakeProxy` | AABB: four generated vertices and zero radius | [x] |
| C014 | `c2MakeProxy` | capsule: two vertices and radius copied | [x] |
| C015 | `c2Len` | zero and nonzero vectors | [x] |
| C016 | `c2Det2` | clockwise, collinear, and counter-clockwise vectors | [x] |
| C017 | `c2GJKSimplexMetric` | simplex count `1` or unsupported/default count | [x] |
| C018 | `c2GJKSimplexMetric` | simplex count `2` | [x] |
| C019 | `c2GJKSimplexMetric` | simplex count `3` | [x] |
| C020 | `c2Mulrv` | identity and nontrivial rotations | [x] |
| C021 | `c2MulrvT` | identity and nontrivial inverse rotations | [x] |
| C022 | `c2Add` | mixed-sign finite vectors | [x] |
| C023 | `c2Mulxv` | identity and translated/rotated transforms | [x] |
| C024 | `c2MulxvT` | identity and translated/rotated inverse transforms | [x] |
| C025 | `c2Intersect` | opposite plane signs and either endpoint exactly on-plane (`da != db`) | [x] |
| C026 | `c2Div` | positive and negative nonzero divisor | [x] |
| C027 | `c2Div` | positive and negative zero divisor | [x] |
| C028 | `c2Norm` | nonzero vector in each quadrant | [x] |
| C029 | `c2Norm` | zero vector | [x] |
| C030 | `c2Neg`, `c2CCW90`, `c2Skew`, `c2Absv` | mixed-sign vectors, zero components | [x] |
| C031 | `c22` | vertex-A region: `v <= 0` | [x] |
| C032 | `c22` | vertex-B region: `v > 0 && u <= 0` | [x] |
| C033 | `c22` | edge region: `v > 0 && u > 0` | [x] |
| C034 | `c23` | vertex-A region: `vAB <= 0 && uCA <= 0` | [x] |
| C035 | `c23` | vertex-B region: first condition false, `uAB <= 0 && vBC <= 0` | [x] |
| C036 | `c23` | vertex-C region: prior conditions false, `uBC <= 0 && vCA <= 0` | [x] |
| C037 | `c23` | edge-AB region: `uAB > 0 && vAB > 0 && wABC <= 0` | [x] |
| C038 | `c23` | edge-BC region: `uBC > 0 && vBC > 0 && uABC <= 0` | [x] |
| C039 | `c23` | edge-CA region: `uCA > 0 && vCA > 0 && vABC <= 0` | [x] |
| C040 | `c23` | triangle interior/default region | [x] |
| C041 | `c2D` | simplex count `1` | [x] |
| C042 | `c2D` | count `2`, determinant `> 0` | [x] |
| C043 | `c2D` | count `2`, determinant `<= 0` | [x] |
| C044 | `c2D` | count `3` and unsupported/default count | [x] |
| C045 | `c2Support` | one vertex | [x] |
| C046 | `c2Support` | multiple vertices, unique maximum and tied maximum (first wins) | [x] |
| C047 | `c2Witness` | simplex count `1` | [x] |
| C048 | `c2Witness` | simplex count `2` weighted by `u/div` | [x] |
| C049 | `c2Witness` | simplex count `3` weighted by `u/div` | [x] |
| C050 | `c2Witness` | unsupported/default simplex count | [x] |
| C051 | `c2L` | simplex count `1` | [x] |
| C052 | `c2L` | simplex count `2` weighted by `u/div` | [x] |
| C053 | `c2L` | simplex count `3` and unsupported/default count | [x] |
| C054 | `c2GJK` | circle-circle, identity transforms, radius disabled, no cache | [x] |
| C055 | `c2GJK` | circle-AABB, identity transforms, radius disabled, no cache | [x] |
| C056 | `c2GJK` | circle-capsule, identity transforms, radius disabled, no cache | [x] |
| C057 | `c2GJK` | AABB-circle, identity transforms, radius disabled, no cache | [x] |
| C058 | `c2GJK` | AABB-AABB, identity transforms, radius disabled, no cache | [x] |
| C059 | `c2GJK` | AABB-capsule, identity transforms, radius disabled, no cache | [x] |
| C060 | `c2GJK` | capsule-circle, identity transforms, radius disabled, no cache | [x] |
| C061 | `c2GJK` | capsule-AABB, identity transforms, radius disabled, no cache | [x] |
| C062 | `c2GJK` | capsule-capsule, identity transforms, radius disabled, no cache | [x] |
| C063 | `c2GJK` | `ax_ptr`/`bx_ptr`: null/null, set/null, null/set, set/set | [x] |
| C064 | `c2GJK` | `use_radius == 0` and nonzero, separated shapes | [x] |
| C065 | `c2GJK` | nonzero `use_radius`, raw distance `<= rA+rB` | [x] |
| C066 | `c2GJK` | `outA`/`outB`: null/null, set/null, null/set, set/set | [x] |
| C067 | `c2GJK` | `iterations`: null and non-null | [x] |
| C068 | `c2GJK` | cache: null, zero-count cache, and warm nonzero-count cache | [x] |
| C069 | `c2CircletoCircleManifold` | separated or exactly tangent | [x] |
| C070 | `c2CircletoCircleManifold` | overlap with distinct centers | [x] |
| C071 | `c2CircletoCircleManifold` | overlap with coincident centers (`l == 0`) | [x] |
| C072 | `c2CircletoAABBManifold` | outside or exactly tangent (`d2 >= r2`) | [x] |
| C073 | `c2CircletoAABBManifold` | external overlap (`d2 != 0`) | [x] |
| C074 | `c2CircletoAABBManifold` | center inside, `x_overlap < y_overlap`, either x sign | [x] |
| C075 | `c2CircletoAABBManifold` | center inside, `x_overlap >= y_overlap`, either y sign | [x] |
| C076 | `c2CircletoCapsuleManifold` | separated or exactly tangent | [x] |
| C077 | `c2CircletoCapsuleManifold` | overlap with nonzero GJK distance | [x] |
| C078 | `c2CircletoCapsuleManifold` | zero GJK distance | [x] |
| C079 | `c2AABBtoAABBManifold` | x-separated | [x] |
| C080 | `c2AABBtoAABBManifold` | x-overlap but y-separated | [x] |
| C081 | `c2AABBtoAABBManifold` | overlap with `dx < dy`, either x normal sign | [x] |
| C082 | `c2AABBtoAABBManifold` | overlap with `dx >= dy`, either y normal sign | [x] |
| C089 | `c2Norms` | counts `1`, `2`, and `8`, including degenerate edges | [x] |
| C090 | `c2AABBtoCapsuleManifold` | separated, rounded contact, and deep clipped contact | [x] |
| C091 | `c2CapsuletoCapsuleManifold` | separated/tangent, nonzero-distance overlap, zero-distance overlap | [x] |
| C092 | `c2Collide` | ordered pair circle-circle | [x] |
| C093 | `c2Collide` | ordered pair circle-AABB | [x] |
| C094 | `c2Collide` | ordered pair circle-capsule | [x] |
| C095 | `c2Collide` | ordered pair AABB-circle (normal reversal) | [x] |
| C096 | `c2Collide` | ordered pair AABB-AABB | [x] |
| C097 | `c2Collide` | ordered pair AABB-capsule | [x] |
| C098 | `c2Collide` | ordered pair capsule-circle (normal reversal) | [x] |
| C099 | `c2Collide` | ordered pair capsule-AABB (normal reversal) | [x] |
| C100 | `c2Collide` | ordered pair capsule-capsule | [x] |
| C101 | `ptr_from_parts` | circle layout | [x] |
| C102 | `ptr_from_parts` | AABB layout | [x] |
| C103 | `ptr_from_parts` | capsule layout | [x] |
| C104 | `omni_manifold` | ordered pair circle-circle | [x] |
| C105 | `omni_manifold` | ordered pair circle-AABB | [x] |
| C106 | `omni_manifold` | ordered pair circle-capsule | [x] |
| C107 | `omni_manifold` | ordered pair AABB-circle | [x] |
| C108 | `omni_manifold` | ordered pair AABB-AABB | [x] |
| C109 | `omni_manifold` | ordered pair AABB-capsule | [x] |
| C110 | `omni_manifold` | ordered pair capsule-circle | [x] |
| C111 | `omni_manifold` | ordered pair capsule-AABB | [x] |
| C112 | `omni_manifold` | ordered pair capsule-capsule | [x] |

## Enumerated undefined configurations

`c2CapsuletoPolyManifold` and `c2GJK` with `C2_TYPE_POLY` are exported, but
there is no defined C configuration to place in the valid-input table:
`c2MakeProxy` has no `C2_TYPE_POLY` case, so `c2GJK` immediately reads an
uninitialized `c2Proxy`. Under the prescribed CMake build, repeated direct
differential attempts produced both data-dependent results and SIGSEGV. These
entry points remain in `SYMBOLS.md`; they cannot have a byte-identical expected
result without changing the C ground truth.
