# Configuration Surface

Mechanically derived from the public declaration in `c_src/include/lib.h` and
the twelve explicit `case` branches in the public `get_predict_func` switch in
`c_src/src/lib.c`. The crate declares no Cargo features, and neither build
produces a binary executable.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `get_predict_func` | scalar `int` predictor selector `pfcn = 0` | [x] |
| 2 | `get_predict_func` | scalar `int` predictor selector `pfcn = 1` | [x] |
| 3 | `get_predict_func` | scalar `int` predictor selector `pfcn = 2` | [x] |
| 4 | `get_predict_func` | scalar `int` predictor selector `pfcn = 3` | [x] |
| 5 | `get_predict_func` | scalar `int` predictor selector `pfcn = 4` | [x] |
| 6 | `get_predict_func` | scalar `int` predictor selector `pfcn = 5` | [x] |
| 7 | `get_predict_func` | scalar `int` predictor selector `pfcn = 6` | [x] |
| 8 | `get_predict_func` | scalar `int` predictor selector `pfcn = 7` | [x] |
| 9 | `get_predict_func` | scalar `int` predictor selector `pfcn = 8` | [x] |
| 10 | `get_predict_func` | scalar `int` predictor selector `pfcn = 9` | [x] |
| 11 | `get_predict_func` | scalar `int` predictor selector `pfcn = 10` | [x] |
| 12 | `get_predict_func` | scalar `int` predictor selector `pfcn = 11` | [x] |
