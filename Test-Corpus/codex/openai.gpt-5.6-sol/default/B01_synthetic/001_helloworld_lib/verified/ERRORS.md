# Error Surface

Mechanically inspected `../c_src/include/hello.h` and
`../c_src/src/hello.c` for error-return statements, error macros, assertions,
range checks, null checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | Status |
|---|----------|---------------------------------------------|-------------------|--------|

There are no rows: `helloworld` takes no arguments, performs no input checks,
and unconditionally returns `0`. Generic pointer, length, range, and enum
boundaries do not apply to this API.
