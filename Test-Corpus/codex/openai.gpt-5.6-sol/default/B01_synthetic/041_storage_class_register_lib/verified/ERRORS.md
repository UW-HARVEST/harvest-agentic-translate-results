# Error Surface

Mechanical search covered `c_src/include/driver.h` and
`c_src/src/driver.c` for error-return statements/macros, assertions, null
checks, range checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|

No rejection paths exist. `driver` accepts one by-value C `int`, returns
`void`, and performs no validation. Pointer, length, and enum boundary cases
are not applicable to this API.

## Completion

- [x] Every C rejection branch is represented (0 total).
- [x] Generic null-pointer, length, and invalid-enum cases are inapplicable.
