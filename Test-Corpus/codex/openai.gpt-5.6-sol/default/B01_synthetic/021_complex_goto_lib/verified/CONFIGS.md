# Configuration surface

The public API has one entry point:

```c
void driver(int x, int y);
```

There are no runtime options, modes, flags, element types, formats, byte-order
choices, compile-time feature gates, or lower-level public entry points. The
rows below are the mechanically pruned cross-product of the C branches
`x > 0 || y > 0`, `x == 1 && y == 4`, `x > 0`, `y == 0`, and `x < 3`, plus
empty/one/many repetition shapes.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|---|---|:---:|
| 1 | `driver` | `x <= 0`, `y <= 0`: outer loop is empty | [x] |
| 2 | `driver` | `x == 1`, `y == 0`: one outer iteration, then `y == 0` continue | [x] |
| 3 | `driver` | `x >= 2`, `y == 0`: many outer iterations through the `y == 0` continue | [x] |
| 4 | `driver` | `x <= 0`, `y == 1`: one `y` operation reached through `x < 3` | [x] |
| 5 | `driver` | `x <= 0`, `y >= 2`: many inner `y` operations through `goto label1` | [x] |
| 6 | `driver` | exact `x == 1`, `y == 4`: special `goto label2` skips the first `x` operation | [x] |
| 7 | `driver` | `x == 1`, `y > 0`, `y != 4`: normal immediate `x < 3` inner path | [x] |
| 8 | `driver` | `x` is `2` or `3`, `y > 0`: first `x` decrement makes `x < 3`, entering the inner path | [x] |
| 9 | `driver` | `x == 4`, `y == 1`: one paired outer iteration exhausts `y`, then the `y == 0` path drains `x` | [x] |
| 10 | `driver` | `x == 4`, `y >= 2`: one paired outer iteration leaves `y`, then the low-`x` inner path drains both | [x] |
| 11 | `driver` | `x >= 5`, `1 <= y <= x - 3`: repeated paired iterations exhaust `y` before low-`x`, then `y == 0` drains `x` | [x] |
| 12 | `driver` | `x >= 5`, `y >= x - 2`: repeated paired iterations reach low `x` while `y` remains, then the inner path drains both | [x] |

All rows are for inputs on which the C implementation has defined, terminating
behavior. `x > 0 && y < 0` repeatedly decrements a negative signed `int` and
cannot normally return before signed underflow, so it is not a valid-path
configuration.

## Build configurations

`Cargo.toml` defines no features, so the only feature combination is the
default/no-feature build. There is no binary target in either build system.
