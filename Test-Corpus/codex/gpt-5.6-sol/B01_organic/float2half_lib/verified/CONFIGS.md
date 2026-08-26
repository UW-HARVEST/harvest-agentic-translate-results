# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
options or conditional sources. There is one valid combination:

| # | Cargo features | CMake configuration |
|---|----------------|---------------------|
| 1 | none (`--no-default-features`) | default |

## Runtime Configurations

The public header declares only `float2half(float)`. It has no runtime options,
state, element counts, byte-order controls, or alternate entry points. The C
implementation derives a 9-bit lookup index from sign and exponent:
`(bits >> 23) & 0x1ff`. The two 512-entry tables partition that index into the
following sign/exponent regimes. Each row covers every exponent in its stated
range, boundary mantissas, and many fixed-seed randomized mantissas.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `float2half` | positive; exponent 0..102; underflow/very-small regime, including +zero and float subnormals | [x] |
| 2 | `float2half` | positive; exponent 103..112; half-subnormal regime with exponent-dependent shift | [x] |
| 3 | `float2half` | positive; exponent 113..142; finite half-normal regime | [x] |
| 4 | `float2half` | positive; exponent 143..254; overflow-to-infinity regime | [x] |
| 5 | `float2half` | positive; exponent 255; infinity/NaN payload regime | [x] |
| 6 | `float2half` | negative; exponent 0..102; underflow/very-small regime, including -zero and float subnormals | [x] |
| 7 | `float2half` | negative; exponent 103..112; half-subnormal regime with exponent-dependent shift | [x] |
| 8 | `float2half` | negative; exponent 113..142; finite half-normal regime | [x] |
| 9 | `float2half` | negative; exponent 143..254; overflow-to-infinity regime | [x] |
| 10 | `float2half` | negative; exponent 255; infinity/NaN payload regime | [x] |
