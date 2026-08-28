# Error Surface

Mechanical inspection covered every `return`, `switch`, and conditional in
`../c_src/src/lib.c` and the declaration in `../c_src/include/lib.h`. The
source contains no error-return macro, error enum, assertion, null check,
range rejection, pointer parameter, length parameter, or min/max constraint
on its sole public entry point.

| # | function | trigger (the exact invalid input/condition) | expected C result | Tested |
|---|----------|----------------------------------------------|-------------------|--------|

There are no rejection rows. Every possible C `int` is accepted by
`get_predict_func`; values outside `0..=11`, including `INT_MIN` and
`INT_MAX`, take the `default` branch and return `0`.

Boundary acceptance is verified differentially by
`tests/differential.rs::generic_default_configuration_matches_randomized_inputs`.
