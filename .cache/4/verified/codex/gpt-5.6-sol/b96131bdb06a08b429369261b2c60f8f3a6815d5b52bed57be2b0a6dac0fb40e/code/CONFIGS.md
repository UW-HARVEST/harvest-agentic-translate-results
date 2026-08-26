# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no
configuration option or conditional source. There is exactly one valid
combination:

| # | Cargo invocation | CMake configuration | [ ] |
|---|------------------|---------------------|-----|
| B1 | `--no-default-features` (empty feature set) | default | [x] |

## Runtime and Input Configurations

The public surface contains only `rgb_to_hsv(float *dest, const float *src)`.
It has no runtime options, modes, flags, sizes, formats, element-type choices,
or byte-order choices. The rows below are the cross-product pruned to the
control-flow and IEEE-754 input shapes the C source actually distinguishes.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| V1 | `rgb_to_hsv` | finite nonzero grayscale: `delta == 0`, `max != 0` | [x] |
| V2 | `rgb_to_hsv` | all zero, including signed-zero variants: `delta == 0`, `max == 0` | [x] |
| V3 | `rgb_to_hsv` | unequal nonpositive channels with maximum zero: `delta != 0`, `max == 0` | [x] |
| V4 | `rgb_to_hsv` | unique red maximum and `g >= b`: red branch, no hue adjustment | [x] |
| V5 | `rgb_to_hsv` | unique red maximum and `g < b`: red branch, negative hue adjusted by 360 | [x] |
| V6 | `rgb_to_hsv` | unique green maximum: green branch | [x] |
| V7 | `rgb_to_hsv` | unique blue maximum: blue branch | [x] |
| V8 | `rgb_to_hsv` | red/green tied maximum: first (`r == max`) branch wins | [x] |
| V9 | `rgb_to_hsv` | red/blue tied maximum: first (`r == max`) branch wins | [x] |
| V10 | `rgb_to_hsv` | green/blue tied maximum: second (`g == max`) branch wins | [x] |
| V11 | `rgb_to_hsv` | all finite negative, unequal channels: nonzero negative `max` | [x] |
| V12 | `rgb_to_hsv` | finite subnormal and extreme values, including overflowing `delta` | [x] |
| V13 | `rgb_to_hsv` | NaN in red: ordered comparisons with `r` are false | [x] |
| V14 | `rgb_to_hsv` | NaN in green: ternary min/max select the green operand | [x] |
| V15 | `rgb_to_hsv` | NaN in blue: ternary min/max select the blue operand | [x] |
| V16 | `rgb_to_hsv` | positive or negative infinity in at least one channel | [x] |
| V17 | `rgb_to_hsv` | exact in-place operation: `dest == src` | [x] |
| V18 | `rgb_to_hsv` | partially overlapping three-float source/destination regions | [x] |
