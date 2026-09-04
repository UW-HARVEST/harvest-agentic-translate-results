# Error-surface table

Mechanically derived from null checks, range checks, and limit constants in
`../c_src/src/driver.c`. There are no error-return macros, `return -1`,
`return NULL`, assertions, public enums, lengths, or error codes.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `printLine` | `line == NULL` | returns `void` and writes zero bytes to stdout | [x] |
| 2 | `good` (internal `goodB2G`) | fixed `data == CHAR_MAX`, therefore `data >= CHAR_MAX / 2` | rejects the multiplication and writes `data value is too large to perform arithmetic safely.\n` after the preceding `goodG2B` output | [x] |

## Non-rejection guards and generic-boundary audit

- `bad`: `data > 0` is always true because `data` is fixed to `CHAR_MAX`
  (`127` on this build).
- `goodG2B`: `data > 0` is always true because `data` is fixed to `2`.
- `goodB2G`: `data > 0` is always true because `data` is fixed to `CHAR_MAX`.
- No API accepts a length, count, allocation size, or enum. Zero/oversized
  lengths and out-of-range enum discriminants are therefore not applicable.
- `driver(int useGood)` deliberately accepts the full C `int` domain. Zero
  selects `bad`; every nonzero value, including `INT_MIN` and `INT_MAX`,
  selects `good`.
- `printHexCharLine(char)` deliberately accepts the full signed-`char` domain
  (`SCHAR_MIN` through `SCHAR_MAX`).
