# Error Surface

Mechanical search covered every C source and public header for error-return
macros/statements, `return -1`, `return NULL`, error enums, assertions,
conditionals, range checks, null checks, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are no rows: `helloworld(void)` accepts no input and the C implementation
contains no rejection or error path. Generic FFI null, length, boundary, and
out-of-range-enum cases are not constructible for a function with no
parameters.
