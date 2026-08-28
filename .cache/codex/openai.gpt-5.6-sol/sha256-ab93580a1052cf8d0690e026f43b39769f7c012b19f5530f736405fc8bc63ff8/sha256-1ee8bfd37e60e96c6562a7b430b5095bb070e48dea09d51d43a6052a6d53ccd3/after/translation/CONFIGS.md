# Configuration surface

The public header declares only `rgb_to_hsv(float *dest, const float *src)`.
There are no runtime options, flags, modes, feature macros, lengths, element
types, formats, or byte-order settings. The rows below are derived from every
comparison branch in `src/lib.c`: the min/max ternaries, the early return,
dominant-channel selection, and negative-hue adjustment. The tie and IEEE-754
rows cover comparison outcomes that change those branches.

Each row is exercised with deterministic randomized bit patterns or randomized
finite values, as appropriate.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `rgb_to_hsv` | finite achromatic input (`r == g == b`, `delta == 0`, nonzero `max`) | [x] |
| 2 | `rgb_to_hsv` | signed-zero achromatic input (`delta == 0`; preserves the selected `max` zero sign) | [x] |
| 3 | `rgb_to_hsv` | finite non-achromatic input with `max == 0` (zero plus negative channels) | [x] |
| 4 | `rgb_to_hsv` | finite, unique red maximum with `g >= b` (`r == max`, no negative-hue adjustment) | [x] |
| 5 | `rgb_to_hsv` | finite, unique red maximum with `g < b` (`r == max`, negative-hue adjustment) | [x] |
| 6 | `rgb_to_hsv` | finite, unique green maximum (`r != max`, `g == max`) | [x] |
| 7 | `rgb_to_hsv` | finite, unique blue maximum (`r != max`, `g != max`) | [x] |
| 8 | `rgb_to_hsv` | finite red/green maximum tie above blue (red branch wins) | [x] |
| 9 | `rgb_to_hsv` | finite red/blue maximum tie above green (red branch wins and hue is adjusted) | [x] |
| 10 | `rgb_to_hsv` | finite green/blue maximum tie above red (green branch wins) | [x] |
| 11 | `rgb_to_hsv` | subnormal finite channels, including a subnormal `delta` | [x] |
| 12 | `rgb_to_hsv` | `r` is NaN; first min/max comparisons are false | [x] |
| 13 | `rgb_to_hsv` | `g` is NaN; the ternaries select `g`, then the following comparisons are false | [x] |
| 14 | `rgb_to_hsv` | `b` is NaN; final min/max ternaries select `b` | [x] |
| 15 | `rgb_to_hsv` | positive infinity is the unique maximum (red, green, and blue placements) | [x] |
| 16 | `rgb_to_hsv` | negative infinity is the unique minimum (red, green, and blue placements) | [x] |
| 17 | `rgb_to_hsv` | multiple infinities, producing infinite-minus-infinite `delta` | [x] |
| 18 | `rgb_to_hsv` | separate non-overlapping source and destination arrays | [x] |
| 19 | `rgb_to_hsv` | exact in-place operation (`dest == src`) | [x] |
| 20 | `rgb_to_hsv` | partially overlapping source and destination arrays in either direction | [x] |

## Public entry-point audit

`rgb_to_hsv` is the only declaration in `include/lib.h` and the only global
symbol defined by the C shared object. It is both the lowest-level and the
only convenience-level entry point.

## Compile-time feature audit

`Cargo.toml` declares no features, and the C source has no conditional
compilation branches. The sole feature configuration is Cargo's empty default
feature set.
