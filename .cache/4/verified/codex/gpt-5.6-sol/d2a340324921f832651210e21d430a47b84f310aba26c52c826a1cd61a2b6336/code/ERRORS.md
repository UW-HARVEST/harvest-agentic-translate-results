# Error Surface

Mechanical searches covered both `c_src/include/driver.h` and
`c_src/src/driver.c` for error returns, `return`, `assert`, conditionals,
switches, null checks, range checks, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|

There are no rows: neither exported function returns a value or accepts a
pointer, enum, buffer, or length, and the C implementation contains no input
rejection, conditional, assertion, range check, error enum, or error-return
statement. The `int` parameter has no documented restricted range. Inputs that
cause signed C arithmetic overflow are undefined behavior, not C rejection
paths, and are outside the differential contract.

- [x] Every mechanically identified C rejection path has a differential test
  (the set is empty).
- [x] Generic pointer, length, and enum boundaries are not applicable to this
  scalar-only API.
