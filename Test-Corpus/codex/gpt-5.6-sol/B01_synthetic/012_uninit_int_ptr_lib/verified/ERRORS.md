# Error Surface

Mechanical search covered `c_src/include/driver.h` and
`c_src/src/driver.c` for error-return statements/macros, `assert`, null
checks, range checks, enum validation, and min/max constants.

The C source contains **no explicit input rejection or error branch**.
Consequently, the source-derived error-surface table has zero rows:

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|

## Mandatory generic boundary probes

These are required boundary probes rather than source-derived rejection rows.
The API has no lengths, documented numeric ranges, or enum parameters, so
zero/oversized lengths, one-past-range values, and invalid enums do not apply.

| # | function | boundary input | expected C result | status |
|---|----------|----------------|-------------------|--------|
| G1 | `printIntPtrLine` | `intNumber == NULL` | unconditional null dereference; compare process termination and output | [x] |

## Completion

- [x] Every source-derived row passes (zero rows).
- [x] Every applicable generic boundary probe passes.
