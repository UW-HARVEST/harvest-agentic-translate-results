# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
options or compile definitions. The complete valid feature matrix therefore
contains one combination:

| # | Rust feature combination | C configuration |
|---|--------------------------|-----------------|
| 1 | no features (`--no-default-features`) | default |

## Runtime configurations

The public headers expose one entry point and no runtime options, modes, flags,
lengths, formats, element types, byte-order controls, or enums. Its input shape
is always three `float` values. The rows below are the complete cross-product
after pruning to control-flow combinations the C code distinguishes.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `hsv_to_rgb` | fixed 3-float input; `s == 0` early-return branch | [x] |
| 2 | `hsv_to_rgb` | fixed 3-float input; `s != 0`, `floor(h / 60) == 0` | [x] |
| 3 | `hsv_to_rgb` | fixed 3-float input; `s != 0`, `floor(h / 60) == 1` | [x] |
| 4 | `hsv_to_rgb` | fixed 3-float input; `s != 0`, `floor(h / 60) == 2` | [x] |
| 5 | `hsv_to_rgb` | fixed 3-float input; `s != 0`, `floor(h / 60) == 3` | [x] |
| 6 | `hsv_to_rgb` | fixed 3-float input; `s != 0`, `floor(h / 60) == 4` | [x] |
| 7 | `hsv_to_rgb` | fixed 3-float input; `s != 0`, switch `default` | [x] |
