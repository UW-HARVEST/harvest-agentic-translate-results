# Configuration-surface table

Derived mechanically from the public header and the ordered `if` branches in
`../c_src/src/driver.c`. There are no runtime flags, feature-controlled C
branches, element types, sizes, formats, or lower-level public entry points.
The only public entry point is `driver(int x, int local_y, int z)`.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `driver` | `x != 1`; all C `int` values of `local_y` and `z` are ignored by branch selection | [x] |
| 2 | `driver` | `x == 1`, `local_y != 2`; all C `int` values of `z` are ignored by branch selection | [x] |
| 3 | `driver` | `x == 1`, `local_y == 2`, `z != 3` | [x] |
| 4 | `driver` | `x == 1`, `local_y == 2`, `z == 3` | [x] |

Cargo feature combinations: one (the default build). `Cargo.toml` declares no
features, so there are no additional feature combinations.
