# Error Surface

The source scan covered `c_src/include/` and `c_src/src/` for error-return
statements/macros, assertions, conditionals, switches, null checks, enums, and
min/max constants. The C public API accepts only by-value `int` arguments and
contains no rejection or error path.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are zero rows to check. Null pointers, lengths, and enum discriminants do
not occur in this API. The full C `int` domain, including zero, `INT_MIN`, and
`INT_MAX`, is valid and is covered as valid input in `CONFIGS.md`.

- [x] Phase C complete: the source has zero rejection rows, and none of the
      generic invalid-input categories exist in this by-value `int` API.
