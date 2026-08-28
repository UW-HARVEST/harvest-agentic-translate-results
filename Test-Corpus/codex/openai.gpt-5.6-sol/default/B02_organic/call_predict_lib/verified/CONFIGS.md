# Configuration Surface

The dynamic API has one entry point and one by-value `int` option. The C
`switch` distinguishes each value from 0 through 11 by selecting and comparing
a different predictor function. There are no data buffers, sizes, formats,
flags, mutable state, compile-time feature branches, or input-shape axes on the
exported boundary.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `call_predict` | `pfcn = 0`; scalar C `int` | [x] |
| 2 | `call_predict` | `pfcn = 1`; scalar C `int` | [x] |
| 3 | `call_predict` | `pfcn = 2`; scalar C `int` | [x] |
| 4 | `call_predict` | `pfcn = 3`; scalar C `int` | [x] |
| 5 | `call_predict` | `pfcn = 4`; scalar C `int` | [x] |
| 6 | `call_predict` | `pfcn = 5`; scalar C `int` | [x] |
| 7 | `call_predict` | `pfcn = 6`; scalar C `int` | [x] |
| 8 | `call_predict` | `pfcn = 7`; scalar C `int` | [x] |
| 9 | `call_predict` | `pfcn = 8`; scalar C `int` | [x] |
| 10 | `call_predict` | `pfcn = 9`; scalar C `int` | [x] |
| 11 | `call_predict` | `pfcn = 10`; scalar C `int` | [x] |
| 12 | `call_predict` | `pfcn = 11`; scalar C `int` | [x] |

The out-of-range `default` branch is tracked in `ERRORS.md`.
