# Configuration Surface

The public API has one entry point, no runtime options, no compile-time Cargo
features, and a fixed input/output shape of three `float` values. The rows
below are the complete cross-product after pruning by the branches actually
present in `c_src/src/lib.c`: the saturation early return and every arm of the
`switch (floorf(h / 60.0f))`.

For nonzero saturation, `i = floorf(h / 60.0f)` and the switch selects the
listed path. The default row includes every `i` other than 0 through 4,
including sector 5, negative/out-of-range finite hues, and the implementation's
observed integer-conversion result for non-finite hues.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `hsv_to_rgb` | fixed three-float input/output; `s == +0.0` or `s == -0.0`; grayscale early return | [x] |
| 2 | `hsv_to_rgb` | fixed three-float input/output; `s != 0`; `floorf(h / 60) == 0` | [x] |
| 3 | `hsv_to_rgb` | fixed three-float input/output; `s != 0`; `floorf(h / 60) == 1` | [x] |
| 4 | `hsv_to_rgb` | fixed three-float input/output; `s != 0`; `floorf(h / 60) == 2` | [x] |
| 5 | `hsv_to_rgb` | fixed three-float input/output; `s != 0`; `floorf(h / 60) == 3` | [x] |
| 6 | `hsv_to_rgb` | fixed three-float input/output; `s != 0`; `floorf(h / 60) == 4` | [x] |
| 7 | `hsv_to_rgb` | fixed three-float input/output; `s != 0`; switch `default` (`i` not in 0..=4) | [x] |
