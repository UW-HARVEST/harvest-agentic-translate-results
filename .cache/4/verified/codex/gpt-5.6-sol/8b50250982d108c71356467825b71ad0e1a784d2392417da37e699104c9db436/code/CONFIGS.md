# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no
options or conditional source selection. There is exactly one valid build
configuration:

| # | Cargo invocation | CMake configuration |
|---|------------------|---------------------|
| 1 | `--no-default-features --features ''` | default |

## Runtime Configurations

The public header declares only `get_predict_func(int pfcn)`. Its two C
switches distinguish each exact value from `0` through `11` and one default
class containing every other C `int`. There are no data buffers, sizes,
formats, state objects, or additional public entry points.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|-------------------------------------------|--------|
| 1 | `get_predict_func` | `pfcn == 0` | [x] |
| 2 | `get_predict_func` | `pfcn == 1` | [x] |
| 3 | `get_predict_func` | `pfcn == 2` | [x] |
| 4 | `get_predict_func` | `pfcn == 3` | [x] |
| 5 | `get_predict_func` | `pfcn == 4` | [x] |
| 6 | `get_predict_func` | `pfcn == 5` | [x] |
| 7 | `get_predict_func` | `pfcn == 6` | [x] |
| 8 | `get_predict_func` | `pfcn == 7` | [x] |
| 9 | `get_predict_func` | `pfcn == 8` | [x] |
| 10 | `get_predict_func` | `pfcn == 9` | [x] |
| 11 | `get_predict_func` | `pfcn == 10` | [x] |
| 12 | `get_predict_func` | `pfcn == 11` | [x] |
| 13 | `get_predict_func` | `pfcn < 0` or `pfcn > 11`, including `INT_MIN`, `-1`, `12`, `INT_MAX`, and randomized values from both ranges | [x] |
