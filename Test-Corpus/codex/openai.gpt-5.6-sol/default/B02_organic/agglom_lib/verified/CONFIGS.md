# Configuration-surface table

Derived from the exported entry points and every `if`/`switch` branch in
`../c_src/src/lib.c`. There are no Cargo features, compile-time configuration
macros, runtime option setters, length-bearing APIs, or executable drivers.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | arbitrary finite and IEEE special `x`, `y` values | [x] |
| 2 | `c2Maxv` | each of the four independent `a.x > b.x` / `a.y > b.y` outcomes, including equality and NaN operands | [x] |
| 3 | `c2Minv` | each of the four independent `a.x < b.x` / `a.y < b.y` outcomes, including equality and NaN operands | [x] |
| 4 | `c2Clampv` | each component below, within, or above `[lo, hi]` (3 × 3 axis cross-product), plus equality | [x] |
| 5 | `c2Sub` | arbitrary finite vectors, signed zero, infinities, and NaNs | [x] |
| 6 | `c2Dot` | finite vectors including cancellation/overflow, signed zero, infinities, and NaNs | [x] |
| 7 | `c2CircletoCircle` | separated circles (`d2 > r2`) | [x] |
| 8 | `c2CircletoCircle` | exactly tangent circles (`d2 == r2`, strict comparison is false) | [x] |
| 9 | `c2CircletoCircle` | overlapping circles (`d2 < r2`), including negative radii as accepted by C | [x] |
| 10 | `c2CircletoAABB` | center independently below/inside/above each AABB axis; miss | [x] |
| 11 | `c2CircletoAABB` | center independently below/inside/above each AABB axis; tangent (`d2 == r2`) | [x] |
| 12 | `c2CircletoAABB` | center independently below/inside/above each AABB axis; hit | [x] |
| 13 | `c2AABBtoAABB` | overlap or edge touch (`d0|d1|d2|d3 == 0`) | [x] |
| 14 | `c2AABBtoAABB` | separated by each individual left/right/below/above predicate and combinations | [x] |
| 15 | `f2` | `CIRCLE` × `CIRCLE` valid type-tag dispatch | [x] |
| 16 | `f2` | `CIRCLE` × `AABB` valid type-tag dispatch | [x] |
| 17 | `f2` | `AABB` × `CIRCLE` valid type-tag dispatch (argument reversal) | [x] |
| 18 | `f2` | `AABB` × `AABB` valid type-tag dispatch | [x] |
| 19 | `f3` | `v1 >= 0`, `v2 > 0` direct quotient | [x] |
| 20 | `f3` | `v1 >= 0`, `v2 < 0`, `v2 != INT_MIN`, both remainder-sign outcomes | [x] |
| 21 | `f3` | `v1 >= 0`, `v2 == INT_MIN` | [x] |
| 22 | `f3` | `v1 < 0`, `v1 != INT_MIN`, `v2 > 0`, both remainder-sign outcomes | [x] |
| 23 | `f3` | `v1 < 0`, `v1 != INT_MIN`, `v2 < 0`, `v2 != INT_MIN`, both remainder-sign outcomes | [x] |
| 24 | `f3` | `v1 < 0`, `v1 != INT_MIN`, `v2 == INT_MIN` | [x] |
| 25 | `f3` | `v1 == INT_MIN`, `v2 > 0` | [x] |
| 26 | `f3` | `v1 == INT_MIN`, `v2 < 0`, `v2 != INT_MIN` | [x] |
| 27 | `f3` | `v1 == INT_MIN`, `v2 == INT_MIN` | [x] |
| 28 | `f4` | arbitrary two-word RNG state, including all-zero and maximum words; output and mutated state | [x] |
| 29 | `f5` | arbitrary `u32`; zero, low-16-bit patterns, and nonzero high 16 bits (which C discards) | [x] |
| 30 | `f7` | `channels == 2`, `bitdepth == 32`, boundary and wrapping `blocksize` | [x] |
| 31 | `f7` | `channels == 2`, `bitdepth != 32`, boundary and wrapping `blocksize` | [x] |
| 32 | `f7` | `channels != 2`, `bitdepth == 32`, including 0/1/many channels and wrapping products | [x] |
| 33 | `f7` | `channels != 2`, `bitdepth != 32`, including 0/1/many channels and wrapping products | [x] |
| 34 | `f9` | nondegenerate triangle and arbitrary point | [x] |
| 35 | `f9` | zero denominator from collinear/repeated vertices, producing IEEE infinities/NaNs | [x] |
| 36 | `f10` | positive/negative zero (`exponent == 0`, mantissa zero) | [x] |
| 37 | `f10` | positive/negative subnormal (`exponent == 0`, mantissa nonzero) | [x] |
| 38 | `f10` | positive/negative normal (`1 <= exponent <= 30`) | [x] |
| 39 | `f10` | positive/negative infinity (`exponent == 31`, mantissa zero) | [x] |
| 40 | `f10` | positive/negative NaN payload (`exponent == 31`, mantissa nonzero) | [x] |
| 41 | `f11` | `s == 0` grayscale early return | [x] |
| 42 | `f11` | `s != 0`, negative `h` (the C third branch accepts `h < 120`) | [x] |
| 43 | `f11` | `s != 0`, `0 <= h < 60` | [x] |
| 44 | `f11` | `s != 0`, `60 <= h < 120` | [x] |
| 45 | `f11` | `s != 0`, `120 <= h < 180` (falls to final `else` in C) | [x] |
| 46 | `f11` | `s != 0`, `180 <= h < 240` | [x] |
| 47 | `f11` | `s != 0`, `240 <= h < 300` | [x] |
| 48 | `f11` | `s != 0`, `300 <= h < 360` | [x] |
| 49 | `f11` | `s != 0`, `h >= 360` or NaN (final `else`) | [x] |
| 50 | `f12` | `s == 0` grayscale early return | [x] |
| 51 | `f12` | `s != 0`, `floor(h/60)` equals 0 | [x] |
| 52 | `f12` | `s != 0`, `floor(h/60)` equals 1 | [x] |
| 53 | `f12` | `s != 0`, `floor(h/60)` equals 2 | [x] |
| 54 | `f12` | `s != 0`, `floor(h/60)` equals 3 | [x] |
| 55 | `f12` | `s != 0`, `floor(h/60)` equals 4 | [x] |
| 56 | `f12` | `s != 0`, any other representable integer sector (default) | [x] |
| 57 | `f13` | `delta == 0` (equal RGB) early return | [x] |
| 58 | `f13` | `max == 0` with nonzero delta early return | [x] |
| 59 | `f13` | `r == max`, nonnegative computed hue | [x] |
| 60 | `f13` | `r == max`, negative computed hue requiring `+360` | [x] |
| 61 | `f13` | `g == max` | [x] |
| 62 | `f13` | `b == max` | [x] |
| 63 | `agglom` | randomized end-to-end composition across all argument families and ordinary finite values | [x] |
| 64 | `agglom` | end-to-end IEEE special values exercising each `isnan`-guarded accumulation | [x] |
