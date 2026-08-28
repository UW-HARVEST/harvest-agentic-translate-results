# Configuration Surface

Source-derived axes:

- Public entry points: `hsl_to_rgb` only.
- Runtime options, modes, flags, compile-time features: none.
- Input shape: two pointers to three contiguous `float` values each.
- Control-flow axes: `s == 0` versus `s != 0`, then the ordered hue tests in
  `c_src/src/lib.c`.

For nonzero saturation, randomized values include ordinary finite values,
subnormals, infinities, and NaNs for `s` and `l`. Each reachable hue class
includes exact branch boundaries and randomized bit patterns belonging to that
class. Outputs are compared as all 12 bytes, preserving float bit patterns.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|--------------------------------------------|--------|
| 1 | `hsl_to_rgb` | `s == +0.0`; any `h` and `l`; early grayscale return | [x] |
| 2 | `hsl_to_rgb` | `s == -0.0`; any `h` and `l`; early grayscale return | [x] |
| 3 | `hsl_to_rgb` | `s != 0`; finite `h < 0` or `h == -INFINITY`; third ordered branch (`h < 120 && h < 180`) | [x] |
| 4 | `hsl_to_rgb` | `s != 0`; `0 <= h < 60`; first hue branch | [x] |
| 5 | `hsl_to_rgb` | `s != 0`; `60 <= h < 120`; second hue branch | [x] |
| 6 | `hsl_to_rgb` | `s != 0`; `120 <= h < 180`; final `else` | [x] |
| 7 | `hsl_to_rgb` | `s != 0`; `180 <= h < 240`; fourth hue branch | [x] |
| 8 | `hsl_to_rgb` | `s != 0`; `240 <= h < 300`; fifth hue branch | [x] |
| 9 | `hsl_to_rgb` | `s != 0`; `300 <= h < 360`; sixth hue branch | [x] |
| 10 | `hsl_to_rgb` | `s != 0`; finite `h >= 360` or `h == +INFINITY`; final `else` | [x] |
| 11 | `hsl_to_rgb` | `s != 0`; `h` is NaN; every comparison is false, final `else` | [x] |
