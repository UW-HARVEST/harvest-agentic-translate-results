# Error Surface

The mechanical scan covered `c_src/include/driver.h`,
`c_src/src/driver.c`, and `c_src/CMakeLists.txt` for error returns, null
checks, range checks, assertions, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no rejection paths. Both exported functions return `void` and accept
one by-value `char`, whose complete domain is exercised by the valid-path
tests. Generic pointer, length, and enum error boundaries do not apply to this
API because it exposes none of those input types.

Phase C status: **complete; 0 mechanically derived rejection rows**.
