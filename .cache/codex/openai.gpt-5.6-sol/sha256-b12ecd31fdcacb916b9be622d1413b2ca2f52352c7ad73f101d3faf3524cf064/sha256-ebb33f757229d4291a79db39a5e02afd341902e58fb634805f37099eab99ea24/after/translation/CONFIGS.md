# Configuration surface

Public entry points from `include/match.h`:

- `spectral_contrast(a, b, length)` (lowest level)
- `match(test, reference, bins, threshold)` (calls preprocessing and then
  `spectral_contrast`)

There are no Cargo features, C preprocessor feature switches, runtime mode
flags, enums, or binary drivers. The meaningful axes are:

- `spectral_contrast`: one/many elements, odd/even element count (important
  because the public header exposes `double *` while its C implementation gets
  `float_t == float` from `<math.h>`), and zero/nonzero magnitudes.
- `match`: the three control-flow outcomes (early rejection, full pipeline
  false, full pipeline true), plus sizes below/equal/above `N_SMOOTH == 16`
  and odd/even sizes where applicable.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `spectral_contrast` | `length == 1`; finite, nonzero interpreted-float magnitudes | [x] |
| 2 | `spectral_contrast` | even `length >= 2`; finite, nonzero interpreted-float magnitudes | [x] |
| 3 | `spectral_contrast` | odd `length >= 3`; finite, nonzero interpreted-float magnitudes | [x] |
| 4 | `spectral_contrast` | positive length; `a` has zero magnitude, `b` nonzero | [x] |
| 5 | `spectral_contrast` | positive length; `a` nonzero, `b` has zero magnitude | [x] |
| 6 | `match` | `bins == 1`; energy gate rejects | [x] |
| 7 | `match` | `bins == 1`; full pipeline returns false | [x] |
| 8 | `match` | even `2 <= bins < 16`; energy gate rejects | [x] |
| 9 | `match` | even `2 <= bins < 16`; full pipeline returns false | [x] |
| 10 | `match` | even `2 <= bins < 16`; full pipeline returns true | [x] |
| 11 | `match` | odd `3 <= bins < 16`; energy gate rejects | [x] |
| 12 | `match` | odd `3 <= bins < 16`; full pipeline returns false | [x] |
| 13 | `match` | odd `3 <= bins < 16`; full pipeline returns true | [x] |
| 14 | `match` | `bins == 16`; energy gate rejects | [x] |
| 15 | `match` | `bins == 16`; full pipeline returns false | [x] |
| 16 | `match` | `bins == 16`; full pipeline returns true | [x] |
| 17 | `match` | even `bins > 16`; energy gate rejects | [x] |
| 18 | `match` | even `bins > 16`; full pipeline returns false | [x] |
| 19 | `match` | even `bins > 16`; full pipeline returns true | [x] |
| 20 | `match` | odd `bins > 16`; energy gate rejects | [x] |
| 21 | `match` | odd `bins > 16`; full pipeline returns false | [x] |
| 22 | `match` | odd `bins > 16`; full pipeline returns true | [x] |
