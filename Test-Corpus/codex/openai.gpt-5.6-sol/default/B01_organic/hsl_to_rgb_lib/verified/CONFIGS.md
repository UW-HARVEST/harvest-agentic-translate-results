# Configuration Surface

The public header declares one entry point:

```c
void hsl_to_rgb(float *dest, const float *src);
```

There are no runtime options, modes, flags, compile-time feature branches,
element-type choices, counts, formats, or byte-order choices. Both input and
output have the fixed shape of three native `float` elements. The meaningful
cross-product is therefore the `s == 0` early-return state plus every distinct
hue branch reached when `s != 0`. Boundary and non-finite classes are listed
explicitly where C comparison semantics select a branch.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| 1 | `hsl_to_rgb` | Fixed 3-float input/output; `s == +0.0` or `s == -0.0`; arbitrary `h` and `l`; early grayscale return | [x] |
| 2 | `hsl_to_rgb` | Fixed 3-float input/output; `s != 0`; finite `h < 0`, including one ULP below zero; third C hue branch | [x] |
| 3 | `hsl_to_rgb` | Fixed 3-float input/output; `s != 0`; finite `0 <= h < 60`, including both neighboring boundary values | [x] |
| 4 | `hsl_to_rgb` | Fixed 3-float input/output; `s != 0`; finite `60 <= h < 120`, including both neighboring boundary values | [x] |
| 5 | `hsl_to_rgb` | Fixed 3-float input/output; `s != 0`; finite `120 <= h < 180`; final fallback branch (matching the C condition exactly) | [x] |
| 6 | `hsl_to_rgb` | Fixed 3-float input/output; `s != 0`; finite `180 <= h < 240`, including both neighboring boundary values | [x] |
| 7 | `hsl_to_rgb` | Fixed 3-float input/output; `s != 0`; finite `240 <= h < 300`, including both neighboring boundary values | [x] |
| 8 | `hsl_to_rgb` | Fixed 3-float input/output; `s != 0`; finite `300 <= h < 360`, including both neighboring boundary values | [x] |
| 9 | `hsl_to_rgb` | Fixed 3-float input/output; `s != 0`; finite `h >= 360`, including 360 and one ULP above; final fallback branch | [x] |
| 10 | `hsl_to_rgb` | Fixed 3-float input/output; `s != 0`; `h == -infinity`; third C hue branch | [x] |
| 11 | `hsl_to_rgb` | Fixed 3-float input/output; `s != 0`; `h == +infinity`; final fallback branch | [x] |
| 12 | `hsl_to_rgb` | Fixed 3-float input/output; `s != 0`; `h` is NaN (multiple signs/payloads); final fallback branch | [x] |
| 13 | `hsl_to_rgb` | Fixed 3-float input/output; nonzero `s` is finite (positive, negative, subnormal, and extreme values); finite `l` (positive, negative, signed zero, subnormal, and extreme values) | [x] |
| 14 | `hsl_to_rgb` | Fixed 3-float input/output; nonzero `s` is NaN or infinity; `h` spans every finite branch plus NaN/infinities; `l` spans finite and non-finite shapes | [x] |
| 15 | `hsl_to_rgb` | Fixed 3-float input/output; `l` is NaN or infinity; finite nonzero `s`; `h` spans every finite branch plus NaN/infinities | [x] |
| 16 | `hsl_to_rgb` | Exact in-place alias (`dest == src`) for all hue branches; C reads all three source floats before writing output | [x] |

There is one Cargo feature combination: the default build with no declared
features. Neither build system produces an executable driver.
