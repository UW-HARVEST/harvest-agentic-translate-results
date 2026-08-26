# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, so there is exactly one valid feature
combination: the empty feature set. It is selected with
`--no-default-features` (an empty `--features` value is omitted).

`c_src/CMakeLists.txt` declares no options or compile definitions. Its only
configuration builds `src/driver.c` as the shared library `driver`.

## Runtime Configurations

The public headers expose only `void driver(int x)`. The C implementation has
no runtime modes or flags and only one data-shape branch, `i < x`. Its
mechanically distinct loop cardinalities are:

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | No options; `x < 0`, so the loop executes zero times | [x] |
| 2 | `driver` | No options; `x == 0`, so the loop executes zero times at the boundary | [x] |
| 3 | `driver` | No options; `x == 1`, so the loop executes exactly once | [x] |
| 4 | `driver` | No options; `x > 1`, so the loop executes many times | [x] |

There are no lower-level public entry points beyond `driver`.
