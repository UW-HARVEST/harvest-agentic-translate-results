# Error Surface

The complete public API is `void driver(int x)`. Mechanical inspection of
`../c_src/include/driver.h` and `../c_src/src/driver.c` found no error-return
statements or macros, assertions, explicit range checks, null checks, enums,
pointer parameters, length parameters, or min/max constants. Every possible
C `int` bit pattern is accepted, so the error-surface table has no rows.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|

Generic FFI boundaries do not add cases here: this API has no pointer, length,
or enum inputs.
