# Error Surface

The following rejection patterns were searched mechanically in
`../c_src/include` and `../c_src/src`: error-return macros, `return -1`,
`return NULL`, uppercase error-enum returns, assertions, null checks, explicit
range checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|

There are no rows: `encode_quant` accepts six `int` values by value and the C
source contains no rejection path, pointer, length, enum, assertion, or
documented valid range. Integer boundary values are therefore valid-path
inputs and are covered in Phase B rather than represented as invented errors.

- [x] Phase C is complete: there are no C rejection rows to test.
- [x] Generic pointer, length, and enum error cases are inapplicable.
- [x] Zero and integer-extreme valid inputs pass the differential boundary
  corpus.
