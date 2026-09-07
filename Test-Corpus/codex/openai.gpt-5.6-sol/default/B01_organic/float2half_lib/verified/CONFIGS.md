# Configuration Surface

The sole public entry point is `float2half(float)`. It has no runtime options,
state, pointers, lengths, element counts, byte-order modes, or compile-time
feature branches. The input-shape axis is the IEEE-754 binary32 bit pattern.

Rows are derived mechanically from the consecutive runs in the C
`m__shift[512]` table, split by sign. The table index is
`(bits >> 23) & 0x1ff`, so these rows cover all 512 sign/exponent indices.
Within every row, tests exercise mantissa `0`, mantissa `0x7fffff`, and many
fixed-seed randomized exponent/mantissa combinations.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| 1 | `float2half` | sign 0; exponent 0..102; shift 24; zero/underflow result class | [x] |
| 2 | `float2half` | sign 0; exponent 103; shift 23; subnormal result class | [x] |
| 3 | `float2half` | sign 0; exponent 104; shift 22; subnormal result class | [x] |
| 4 | `float2half` | sign 0; exponent 105; shift 21; subnormal result class | [x] |
| 5 | `float2half` | sign 0; exponent 106; shift 20; subnormal result class | [x] |
| 6 | `float2half` | sign 0; exponent 107; shift 19; subnormal result class | [x] |
| 7 | `float2half` | sign 0; exponent 108; shift 18; subnormal result class | [x] |
| 8 | `float2half` | sign 0; exponent 109; shift 17; subnormal result class | [x] |
| 9 | `float2half` | sign 0; exponent 110; shift 16; subnormal result class | [x] |
| 10 | `float2half` | sign 0; exponent 111; shift 15; subnormal result class | [x] |
| 11 | `float2half` | sign 0; exponent 112; shift 14; subnormal result class | [x] |
| 12 | `float2half` | sign 0; exponent 113..142; shift 13; normalized finite result class | [x] |
| 13 | `float2half` | sign 0; exponent 143..254; shift 24; finite overflow/saturation class | [x] |
| 14 | `float2half` | sign 0; exponent 255; shift 13; infinity/NaN payload class | [x] |
| 15 | `float2half` | sign 1; exponent 0..102; shift 24; signed zero/underflow result class | [x] |
| 16 | `float2half` | sign 1; exponent 103; shift 23; signed subnormal result class | [x] |
| 17 | `float2half` | sign 1; exponent 104; shift 22; signed subnormal result class | [x] |
| 18 | `float2half` | sign 1; exponent 105; shift 21; signed subnormal result class | [x] |
| 19 | `float2half` | sign 1; exponent 106; shift 20; signed subnormal result class | [x] |
| 20 | `float2half` | sign 1; exponent 107; shift 19; signed subnormal result class | [x] |
| 21 | `float2half` | sign 1; exponent 108; shift 18; signed subnormal result class | [x] |
| 22 | `float2half` | sign 1; exponent 109; shift 17; signed subnormal result class | [x] |
| 23 | `float2half` | sign 1; exponent 110; shift 16; signed subnormal result class | [x] |
| 24 | `float2half` | sign 1; exponent 111; shift 15; signed subnormal result class | [x] |
| 25 | `float2half` | sign 1; exponent 112; shift 14; signed subnormal result class | [x] |
| 26 | `float2half` | sign 1; exponent 113..142; shift 13; signed normalized finite result class | [x] |
| 27 | `float2half` | sign 1; exponent 143..254; shift 24; signed finite overflow/saturation class | [x] |
| 28 | `float2half` | sign 1; exponent 255; shift 13; signed infinity/NaN payload class | [x] |
