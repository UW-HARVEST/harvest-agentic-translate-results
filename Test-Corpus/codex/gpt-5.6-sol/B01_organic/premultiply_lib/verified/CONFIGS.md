# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `cargo metadata` reports an empty
feature map. The complete feature powerset is therefore one combination:

| # | Cargo invocation feature set | matching C configuration |
|---|------------------------------|--------------------------|
| 1 | `--no-default-features` (empty set) | default CMake build; no project options or conditional compilation |

`c_src/CMakeLists.txt` defines one shared-library target from `src/lib.c` and
contains no `option`, compile definition, platform branch, or configurable
backend.

## Runtime Configurations

The public header exposes only `premultiply`. It has no runtime option, mode,
flag, format, enum, byte-order selection, or alternate entry point. The C
implementation branches only at the `for` loop condition. Its loop bound is
`(w * sizeof(cp_pixel_t)) * h`, with four-byte pixels and signed `int`
dimensions.

Every processing row is tested with many fixed-seed randomized pixel arrays.
Those arrays include zero, interior, and maximum RGB channel values and alpha
values `0`, `1..254`, and `255`.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|--------|
| 1 | `premultiply` | no options; `w = 0`, `h = 0`; empty image | [x] |
| 2 | `premultiply` | no options; `w = 0`, `h > 0`; zero-width image | [x] |
| 3 | `premultiply` | no options; `w = 0`, `h < 0`; zero-width, negative-height image | [x] |
| 4 | `premultiply` | no options; `w > 0`, `h = 0`; zero-height image | [x] |
| 5 | `premultiply` | no options; `w < 0`, `h = 0`; negative-width, zero-height image | [x] |
| 6 | `premultiply` | no options; `w > 0`, `h < 0`; mixed signs, non-positive loop bound | [x] |
| 7 | `premultiply` | no options; `w < 0`, `h > 0`; mixed signs, non-positive loop bound | [x] |
| 8 | `premultiply` | no options; `w = 1`, `h = 1`; one pixel | [x] |
| 9 | `premultiply` | no options; `w > 1`, `h = 1`; one row, many pixels | [x] |
| 10 | `premultiply` | no options; `w = 1`, `h > 1`; one column, many pixels | [x] |
| 11 | `premultiply` | no options; `w > 1`, `h > 1`; many rows and columns | [x] |
| 12 | `premultiply` | no options; `w = -1`, `h = -1`; two negatives process one pixel | [x] |
| 13 | `premultiply` | no options; `w < -1`, `h = -1`; two negatives process one row of many pixels | [x] |
| 14 | `premultiply` | no options; `w = -1`, `h < -1`; two negatives process one column of many pixels | [x] |
| 15 | `premultiply` | no options; `w < -1`, `h < -1`; two negatives process many pixels | [x] |
