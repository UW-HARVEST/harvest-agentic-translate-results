# Configuration Surface

The public API has one entry point, no runtime options, no flags, no modes, no
element-type choices, and no compile-time Cargo features. `cp_pixel_t` is
always four bytes in RGBA order. The rows below are the defined input-shape
classes distinguished by the C loop condition and iteration count.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `premultiply` | zero computed extent: `w == 0` or `h == 0`; null `pix` is permitted because the loop does not execute | [x] |
| C2 | `premultiply` | negative computed extent: exactly one of `w` and `h` is negative; non-null data remains untouched | [x] |
| C3 | `premultiply` | one pixel: `w == 1`, `h == 1`; randomized RGBA values including alpha boundaries | [x] |
| C4 | `premultiply` | many pixels: positive `w`, positive `h`; randomized dimensions and RGBA values | [x] |
| C5 | `premultiply` | positive extent from two negative dimensions; randomized dimensions and RGBA values | [x] |

The last row follows the C arithmetic literally: when both dimensions are
negative and the final signed `stride * h` multiplication is representable,
the byte extent is positive and the linear pixel loop executes. Inputs that
overflow that signed multiplication are excluded because the C behavior is
undefined.

Feature combinations to verify:

| Cargo feature selection | [ ] |
|-------------------------|-----|
| default (no features declared) | [x] |
| `--no-default-features` (no features declared) | [x] |
