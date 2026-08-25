# Configuration Surface

Build-time configuration:

- `Cargo.toml` has no `[features]` table. The only valid Rust feature
  combination is `--no-default-features` with no named features.
- `c_src/CMakeLists.txt` has no options, source-selection conditionals, or
  compile definitions. The C library has one default build configuration.

Runtime configuration:

- `nm -D` exposes only `call_predict(int pfcn)`.
- `call_predict` branches on each exact `pfcn` value from 0 through 11 and has
  one default branch for every other C `int`.
- The API has no input buffers, sizes, element types, formats, byte order,
  mutable state, options, or flags.
- The helper predictors and `BTAC1C2_GetPredictFunc` are `static` and are not
  entry points in the C shared object.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|--------|
| 1 | `call_predict` | scalar `pfcn = 0`; dedicated switch case | [x] |
| 2 | `call_predict` | scalar `pfcn = 1`; dedicated switch case | [x] |
| 3 | `call_predict` | scalar `pfcn = 2`; dedicated switch case | [x] |
| 4 | `call_predict` | scalar `pfcn = 3`; dedicated switch case | [x] |
| 5 | `call_predict` | scalar `pfcn = 4`; dedicated switch case | [x] |
| 6 | `call_predict` | scalar `pfcn = 5`; dedicated switch case | [x] |
| 7 | `call_predict` | scalar `pfcn = 6`; dedicated switch case | [x] |
| 8 | `call_predict` | scalar `pfcn = 7`; dedicated switch case | [x] |
| 9 | `call_predict` | scalar `pfcn = 8`; dedicated switch case | [x] |
| 10 | `call_predict` | scalar `pfcn = 9`; dedicated switch case | [x] |
| 11 | `call_predict` | scalar `pfcn = 10`; dedicated switch case | [x] |
| 12 | `call_predict` | scalar `pfcn = 11`; dedicated switch case | [x] |
| 13 | `call_predict` | scalar `pfcn` outside `0..=11`; default switch branch, including `INT_MIN`, `-1`, `12`, and `INT_MAX` | [x] |
