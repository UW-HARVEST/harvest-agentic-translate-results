# Error Surface

Mechanical searches covered `RETURN_ERROR`, `return -1`, `return NULL`,
`assert`, `if`, `switch`, preprocessor conditionals, null checks, range
checks, enum declarations, and min/max constants in `../c_src/include` and
`../c_src/src`.

The sole public entry point accepts two by-value C `int` arguments and returns
`void`. The C implementation has no rejection branch, error return, assertion,
pointer, length, range check, enum, or sentinel result. Therefore there are no
C error-surface rows to test.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| — | — | No invalid representation exists for two by-value C `int` arguments | — | [x] |

Generic boundary audit:

- Null pointers: not applicable; there are no pointer parameters.
- Zero lengths: not applicable; there are no length parameters.
- Oversized lengths: not applicable; there are no length parameters.
- Out-of-range enums: not applicable; there are no enum parameters.
- One-past-range values: not representable through the declared `int` ABI.

