# Error surface

Mechanical searches of `../c_src/include/` and `../c_src/src/` found no
`RETURN_ERROR`, negative/sentinel returns, null returns, error enums, asserts,
conditionals, switches, null checks, range checks, or min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

There are **0 explicit C rejection paths**.

Error-path table: [x] complete (no rows).

The generic pointer, length, and enum boundary cases are not applicable:
`to_barycentric` accepts four fixed-size `lm_vec2` structs by value and has no
pointer, length, option, or enum arguments. Every possible pair of `float` bit
patterns is accepted across the ABI and evaluated by the same arithmetic path.
