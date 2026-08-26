# Configuration Surface

## Build-Time Matrix

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no CMake
options or conditional source selection. There is exactly one valid feature
combination:

| # | Cargo invocation feature arguments | C configuration | [ ] |
|---|------------------------------------|-----------------|-----|
| 1 | `--no-default-features` (no named features) | default | [x] |

## Runtime Matrix

The public API has one entry point, `driver(int, int, int)`. Its inputs are
three scalar C `int` values with no pointer, length, element-type, format,
byte-order, mode, or flag axes. The following rows are the complete pruned
cross-product of the equality branches in `c_src/src/driver.c:33-49`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | `x != 1`; arbitrary `local_y`; arbitrary `z` (first-stage failure) | [x] |
| 2 | `driver` | `x == 1`; `local_y != 2`; arbitrary `z` (second-stage failure) | [x] |
| 3 | `driver` | `x == 1`; `local_y == 2`; `z != 3` (third-stage failure) | [x] |
| 4 | `driver` | `x == 1`; `local_y == 2`; `z == 3` (success) | [x] |

`multi_stage` is `static` and therefore is neither a public C entry point nor
an exported dynamic symbol.
