# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` defines no
options or conditional compilation. There is one valid Rust feature
combination:

| # | enabled features | check command | result |
|---|------------------|---------------|--------|
| 1 | none | `cargo check --no-default-features` | [x] pass |

## Runtime Axes

The public surface contains one entry point:

```c
void hsl_to_rgb(float *dest, const float *src);
```

There are no runtime options, modes, flags, formats, lengths, counts, enums, or
byte-order settings. Both buffers have a fixed shape of three native `float`
elements. The meaningful combinations below are the complete set of distinct
branches selected by `s == 0` and the ordered hue comparisons in `lib.c`.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|--------|
| 1 | `hsl_to_rgb` | fixed 3-float input/output; `s == +0.0` or `s == -0.0`; `h` and `l` arbitrary; achromatic early return | [x] |
| 2 | `hsl_to_rgb` | fixed 3-float input/output; `s != 0`; `0 <= h < 60`; first hue branch | [x] |
| 3 | `hsl_to_rgb` | fixed 3-float input/output; `s != 0`; `60 <= h < 120`; second hue branch | [x] |
| 4 | `hsl_to_rgb` | fixed 3-float input/output; `s != 0`; `h < 0`; effective third branch (`h < 120 && h < 180` after the first two branches) | [x] |
| 5 | `hsl_to_rgb` | fixed 3-float input/output; `s != 0`; `180 <= h < 240`; fourth hue branch | [x] |
| 6 | `hsl_to_rgb` | fixed 3-float input/output; `s != 0`; `240 <= h < 300`; fifth hue branch | [x] |
| 7 | `hsl_to_rgb` | fixed 3-float input/output; `s != 0`; `300 <= h < 360`; sixth hue branch | [x] |
| 8 | `hsl_to_rgb` | fixed 3-float input/output; `s != 0`; `120 <= h < 180`, `h >= 360`, or unordered/NaN `h`; fallback branch | [x] |

Each row must pass byte-for-byte differential tests over fixed boundary cases,
special IEEE-754 values where applicable, and many pseudorandom inputs.
