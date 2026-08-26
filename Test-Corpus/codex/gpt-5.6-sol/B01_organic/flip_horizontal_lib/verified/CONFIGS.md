# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
options or conditional compilation. There is exactly one valid Cargo feature
combination:

```text
--no-default-features
```

## Runtime Configurations

The public header declares one entry point, `flip_horizontal`. The C body has
two data-dependent loops:

- `h / 2` selects no row pair, one row pair, or multiple row pairs; odd
  heights preserve a middle row while even heights do not.
- `w` selects zero, one, or multiple pixels per row pair.
- `cp_pixel_t` is four bytes (`r`, `g`, `b`, `a`) and all byte values are data,
  not modes.

The table is the cross-product of the width and height shapes distinguished by
those loops. Tests use many fixed-seed randomized pixel buffers for every
non-empty shape.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `flip_horizontal` | no options; width 0, height 0 (empty image) | [x] |
| 2 | `flip_horizontal` | no options; width 0, height 1 (one empty row) | [x] |
| 3 | `flip_horizontal` | no options; width 0, height 2 (one empty row pair) | [x] |
| 4 | `flip_horizontal` | no options; width 0, odd height >= 3 (multiple empty rows, middle preserved) | [x] |
| 5 | `flip_horizontal` | no options; width 0, even height >= 4 (multiple empty row pairs) | [x] |
| 6 | `flip_horizontal` | no options; width 1, height 0 (no rows) | [x] |
| 7 | `flip_horizontal` | no options; width 1, height 1 (one pixel, no swap) | [x] |
| 8 | `flip_horizontal` | no options; width 1, height 2 (one single-pixel row pair) | [x] |
| 9 | `flip_horizontal` | no options; width 1, odd height >= 3 (single-pixel rows, middle preserved) | [x] |
| 10 | `flip_horizontal` | no options; width 1, even height >= 4 (multiple single-pixel row pairs) | [x] |
| 11 | `flip_horizontal` | no options; width > 1, height 0 (no rows) | [x] |
| 12 | `flip_horizontal` | no options; width > 1, height 1 (one multi-pixel row, no swap) | [x] |
| 13 | `flip_horizontal` | no options; width > 1, height 2 (one multi-pixel row pair) | [x] |
| 14 | `flip_horizontal` | no options; width > 1, odd height >= 3 (multiple rows, middle preserved) | [x] |
| 15 | `flip_horizontal` | no options; width > 1, even height >= 4 (multiple row pairs) | [x] |
