# Configuration Surface

The public surface consists only of `int get_predict_func(int pfcn)`. There
are no pointers, buffers, lengths, state objects, flags, compile-time feature
branches, or other public entry points. Each explicit public `switch` case and
its default path form the complete set of configurations distinguished by the
C source.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `get_predict_func` | `pfcn == 0`; specialized predictor pointer selected and recognized | [x] |
| 2 | `get_predict_func` | `pfcn == 1`; specialized predictor pointer selected and recognized | [x] |
| 3 | `get_predict_func` | `pfcn == 2`; specialized predictor pointer selected and recognized | [x] |
| 4 | `get_predict_func` | `pfcn == 3`; specialized predictor pointer selected and recognized | [x] |
| 5 | `get_predict_func` | `pfcn == 4`; specialized predictor pointer selected and recognized | [x] |
| 6 | `get_predict_func` | `pfcn == 5`; specialized predictor pointer selected and recognized | [x] |
| 7 | `get_predict_func` | `pfcn == 6`; specialized predictor pointer selected and recognized | [x] |
| 8 | `get_predict_func` | `pfcn == 7`; specialized predictor pointer selected and recognized | [x] |
| 9 | `get_predict_func` | `pfcn == 8`; specialized predictor pointer selected and recognized | [x] |
| 10 | `get_predict_func` | `pfcn == 9`; specialized predictor pointer selected and recognized | [x] |
| 11 | `get_predict_func` | `pfcn == 10`; specialized predictor pointer selected and recognized | [x] |
| 12 | `get_predict_func` | `pfcn == 11`; specialized predictor pointer selected and recognized | [x] |
| 13 | `get_predict_func` | `pfcn < 0` or `pfcn > 11`; generic predictor selected, public default returns `0` | [x] |

Cargo feature combinations: default only (the manifest defines no features).

Coverage: `tests/differential.rs::specialized_predictor_configurations_match`
and
`tests/differential.rs::generic_default_configuration_matches_randomized_inputs`.
