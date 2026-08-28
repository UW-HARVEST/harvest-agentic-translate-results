# Configuration Surface

The public API has one entry point, no runtime options, no state, and no Cargo
features. Its behavioral axes are the sign bit, exponent lookup region, and
mantissa. Rows below partition all 512 values of the C expression
`(bits >> 23) & 0x1ff`. Tests must cover every exponent in each range with
zero, boundary, and many randomized mantissas.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `float2half` | no options; positive; exponent 0; float zero or subnormal; any mantissa | [x] |
| 2 | `float2half` | no options; negative; exponent 0; float zero or subnormal; any mantissa | [x] |
| 3 | `float2half` | no options; positive; exponent 1..102; normal values below half-subnormal range | [x] |
| 4 | `float2half` | no options; negative; exponent 1..102; normal values below half-subnormal range | [x] |
| 5 | `float2half` | no options; positive; exponent 103..112; result in half subnormal range | [x] |
| 6 | `float2half` | no options; negative; exponent 103..112; result in half subnormal range | [x] |
| 7 | `float2half` | no options; positive; exponent 113..142; result in half normal range | [x] |
| 8 | `float2half` | no options; negative; exponent 113..142; result in half normal range | [x] |
| 9 | `float2half` | no options; positive; exponent 143..254; finite overflow/saturation region | [x] |
| 10 | `float2half` | no options; negative; exponent 143..254; finite overflow/saturation region | [x] |
| 11 | `float2half` | no options; positive; exponent 255 and mantissa 0; positive infinity | [x] |
| 12 | `float2half` | no options; negative; exponent 255 and mantissa 0; negative infinity | [x] |
| 13 | `float2half` | no options; positive; exponent 255 and nonzero mantissa; NaN payload propagation | [x] |
| 14 | `float2half` | no options; negative; exponent 255 and nonzero mantissa; NaN payload propagation | [x] |

Build feature combinations: one (the empty/default feature set). Cargo.toml
declares no `[features]` table.
