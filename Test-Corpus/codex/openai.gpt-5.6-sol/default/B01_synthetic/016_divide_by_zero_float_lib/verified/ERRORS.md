# Error-Surface Table

Mechanically derived from every null/range/rejection branch in
`c_src/src/driver.c`. The API has no error enums, error-return macros,
assertions, lengths, or enum parameters.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| E1 | `printLine` | `line == NULL` | Return `void` without calling `printf`; emit zero bytes. | [x] |
| E2 | `good` (through static `goodB2G`) | `!(fabs(data) > 0.000001)`, including `-0.000001 <= data <= 0.000001` and NaN | Emit `50\nThis would result in a divide by zero\n`; return `void`. | [x] |
| E3 | `driver` (through `good`/static `goodB2G`) | `!(fabs(goodData) > 0.000001)`, including `-0.000001 <= goodData <= 0.000001` and NaN | Emit the normal driver framing, `50\n`, and the divide-by-zero warning for the good call; then continue to `bad(badData)`. | [x] |

## Generic FFI boundary audit

The only pointer parameter is covered by E1. There are no length or enum
parameters. Float zero, infinities, NaNs, threshold-adjacent values, and
division results outside the C `int` range are exercised by E2/E3 and the
configuration rows.
