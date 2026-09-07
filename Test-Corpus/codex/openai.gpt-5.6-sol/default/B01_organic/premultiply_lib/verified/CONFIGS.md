# Configuration surface

Mechanical inspection of the public header and implementation found:

- one public entry point: `premultiply`;
- no runtime options, modes, flags, enums, formats, or byte-order choices;
- one fixed four-byte RGBA pixel format;
- no compile-time Cargo features;
- one control-flow condition: the loop runs while
  `i < (w * sizeof(cp_pixel_t)) * h`.

The rows below enumerate the distinct safe, defined input shapes that affect
whether the loop runs zero, one, or many times. Positive row/column/rectangle
shapes are kept separate so randomized tests exercise width and height
independently.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|---|---|---|
| C1 | `premultiply` | One pixel: `w = 1`, `h = 1`; randomized RGBA bytes | [x] |
| C2 | `premultiply` | One row: `w > 1`, `h = 1`; randomized widths and RGBA bytes | [x] |
| C3 | `premultiply` | One column: `w = 1`, `h > 1`; randomized heights and RGBA bytes | [x] |
| C4 | `premultiply` | Rectangle: `w > 1`, `h > 1`; randomized dimensions and RGBA bytes | [x] |
| C5 | `premultiply` | Empty: `w == 0` or `h == 0`; includes null `pix` because C does not read it | [x] |
| C6 | `premultiply` | Negative bound: exactly one of `w`, `h` is negative; C performs zero iterations | [x] |
| C7 | `premultiply` | Positive bound from two negative dimensions; randomized safe magnitudes and backing pixels | [x] |

All non-empty rows include alpha values `0`, `255`, and randomized interior
values across repeated fixed-seed cases. Signed-overflowing dimension products
are excluded because their C behavior is undefined.

## Feature combinations

`Cargo.toml` declares no features, so the only distinct code configuration is
the default build. A `--no-default-features` test run is still used as a parity
check and is behaviorally identical.

All rows pass in both the default and `--no-default-features` test runs.
