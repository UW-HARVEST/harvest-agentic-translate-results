# Error Surface

Mechanical scans of `../c_src/include/` and `../c_src/src/` found no
`RETURN_ERROR`, `return -1`, `return NULL`, error enum, `assert`, `if`,
`switch`, preprocessor branch, explicit range check, null check, or min/max
constant. The sole API takes four `lm_vec2` values by value and always returns
an `lm_vec2`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|

There are no pointer, length, enum, or documented-range parameters, so the
generic null-pointer, zero/oversized-length, out-of-range-enum, and one-past-
range cases are not applicable. Degenerate and non-finite floating-point
inputs are not rejected; they are valid arithmetic configurations covered in
`CONFIGS.md`.

- [x] Phase C confirms there are zero applicable rejection rows.
