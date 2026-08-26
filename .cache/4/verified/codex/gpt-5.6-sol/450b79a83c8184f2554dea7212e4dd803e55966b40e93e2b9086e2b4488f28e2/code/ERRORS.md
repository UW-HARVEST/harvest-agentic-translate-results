# Error Surface

Source scan: `c_src/include/driver.h` and `c_src/src/driver.c`, including every
null check, return statement, assertion, range check, and error macro.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `printLine` | `line == NULL` | Returns `void` immediately and writes no bytes to stdout | [x] |

There are no error-return macros, error enums, assertions, explicit range
checks, length parameters, or min/max constants. No public API accepts an enum,
so an out-of-range enum case does not exist. The null pointer above is the only
applicable generic FFI boundary; zero and oversized lengths are inapplicable
because no entry point accepts a length.
