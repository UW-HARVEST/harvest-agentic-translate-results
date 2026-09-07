# Configuration surface

The header exposes one entry point and no runtime options, modes, flags,
element-type choices, counts, formats, or byte-order controls. The input shape
is always three `float` values and the output shape is always three `float`
values. The rows below mechanically cover the conditions selected by the
comparison and `if` branches in `src/lib.c`, including IEEE-754 values that
alter those comparisons. Pointer overlap is included because the ABI does not
declare `restrict` and the C implementation reads all three inputs before its
first output write.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `rgb_to_hsv` | all channels `+0.0`; both `delta == 0` and `max == 0` are true | [x] |
| 2 | `rgb_to_hsv` | signed-zero permutations; zero comparisons are equal but the selected `v` sign is byte-observable | [x] |
| 3 | `rgb_to_hsv` | all channels equal and finite nonzero; `delta == 0` early return | [x] |
| 4 | `rgb_to_hsv` | unequal finite nonpositive channels with `max == 0` and `delta != 0` | [x] |
| 5 | `rgb_to_hsv` | finite unique red maximum with `g >= b`; red branch and no negative-hue wrap | [x] |
| 6 | `rgb_to_hsv` | finite unique red maximum with `g < b`; red branch and negative-hue wrap | [x] |
| 7 | `rgb_to_hsv` | finite `r == g > b`; tied maximum resolves to the red branch | [x] |
| 8 | `rgb_to_hsv` | finite `r == b > g`; tied maximum resolves to the red branch and wraps negative hue | [x] |
| 9 | `rgb_to_hsv` | finite unique green maximum; green branch | [x] |
| 10 | `rgb_to_hsv` | finite `g == b > r`; tied maximum resolves to the green branch | [x] |
| 11 | `rgb_to_hsv` | finite unique blue maximum; final `else` branch | [x] |
| 12 | `rgb_to_hsv` | finite unique maximum below zero; negative `max` and saturation arithmetic | [x] |
| 13 | `rgb_to_hsv` | infinities among the three channels, exercising infinite `delta` and NaN arithmetic | [x] |
| 14 | `rgb_to_hsv` | NaN in red, with the ternary comparisons replacing it from `min`/`max` | [x] |
| 15 | `rgb_to_hsv` | NaN in green, with later blue comparisons replacing it from `min`/`max` | [x] |
| 16 | `rgb_to_hsv` | NaN in blue, forcing NaN `min`, `max`, `delta`, and the final hue branch | [x] |
| 17 | `rgb_to_hsv` | finite IEEE-754 boundaries: subnormals, minimum normals, and `FLT_MAX` magnitudes | [x] |
| 18 | `rgb_to_hsv` | ordinary finite values with disjoint source and destination arrays | [x] |
| 19 | `rgb_to_hsv` | exact alias: `dest == src` | [x] |
| 20 | `rgb_to_hsv` | partial overlap with `dest == src + 1` in a four-float allocation | [x] |
| 21 | `rgb_to_hsv` | partial overlap with `src == dest + 1` in a four-float allocation | [x] |

Feature combinations: Cargo.toml declares no features, so the only build
configuration is the default/no-feature configuration.
