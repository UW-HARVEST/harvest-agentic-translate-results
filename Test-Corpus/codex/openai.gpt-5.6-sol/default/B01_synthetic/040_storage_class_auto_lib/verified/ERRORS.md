# Error Surface

Mechanical searches of `../c_src/include/` and `../c_src/src/` found no
error-return statements or macros, assertions, explicit range checks, null
checks, error enums, or min/max constants. The sole public API accepts an
`int` by value, returns `void`, and has no rejection path.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|

Generic pointer, length, and enum error boundaries do not apply to this API.
The full C `int` boundary is valid input and is covered by `CONFIGS.md`.

Phase C status: complete; the mechanically derived error surface has zero rows.
