# Configuration surface

Public entry points: `flip_horizontal` (the only declaration in
`c_src/include/lib.h`).

There are no runtime options, modes, flags, enums, compile-time feature
branches, convenience wrappers, or lower-level public functions. The C code
branches only through the outer condition `i < h / 2` and the inner condition
`j < w`. The rows below are the pruned cross-product of the distinct valid
height and width shapes those loops treat differently.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `flip_horizontal` | empty image: `h == 0`, `w == 0` | [x] |
| 2 | `flip_horizontal` | one row: `h == 1`, randomized positive `w`; outer loop does not run | [x] |
| 3 | `flip_horizontal` | one row pair and zero columns: `h == 2`, `w == 0` | [x] |
| 4 | `flip_horizontal` | one row pair and one column: `h == 2`, `w == 1` | [x] |
| 5 | `flip_horizontal` | one row pair and many columns: `h == 2`, `w > 1` | [x] |
| 6 | `flip_horizontal` | odd multi-row image and zero columns: odd `h >= 3`, `w == 0`; middle row is unpaired | [x] |
| 7 | `flip_horizontal` | odd multi-row image and one column: odd `h >= 3`, `w == 1`; middle row is unchanged | [x] |
| 8 | `flip_horizontal` | odd multi-row image and many columns: odd `h >= 3`, `w > 1`; middle row is unchanged | [x] |
| 9 | `flip_horizontal` | even multi-row image and zero columns: even `h >= 4`, `w == 0` | [x] |
| 10 | `flip_horizontal` | even multi-row image and one column: even `h >= 4`, `w == 1` | [x] |
| 11 | `flip_horizontal` | even multi-row image and many columns: even `h >= 4`, `w > 1` | [x] |

Feature combinations from `Cargo.toml`: default only (`[features]` is absent).
