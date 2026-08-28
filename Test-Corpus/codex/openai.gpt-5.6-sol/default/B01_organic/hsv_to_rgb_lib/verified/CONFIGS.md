# Configuration Surface

The public API has no runtime options, flags, modes, compile-time features,
lengths, formats, element-type choices, or byte-order choices. Its only
control-flow axes are `s == 0` and, when `s != 0`, the integer produced by
`(int)floorf(h / 60.0f)`. All inputs and outputs have the fixed shape of three
contiguous `float` values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `hsv_to_rgb` | `s == 0`; fixed three-float source/destination; hue ignored | [x] |
| 2 | `hsv_to_rgb` | `s != 0`; `floorf(h / 60) == 0`; switch case 0 | [x] |
| 3 | `hsv_to_rgb` | `s != 0`; `floorf(h / 60) == 1`; switch case 1 | [x] |
| 4 | `hsv_to_rgb` | `s != 0`; `floorf(h / 60) == 2`; switch case 2 | [x] |
| 5 | `hsv_to_rgb` | `s != 0`; `floorf(h / 60) == 3`; switch case 3 | [x] |
| 6 | `hsv_to_rgb` | `s != 0`; `floorf(h / 60) == 4`; switch case 4 | [x] |
| 7 | `hsv_to_rgb` | `s != 0`; all other conversion results; switch default | [x] |

Each row is exercised with separate source/destination buffers and in-place
aliasing. Randomized inputs include ordinary values, branch boundaries,
subnormal values, signed zero, infinities, and NaNs where the branch permits.
