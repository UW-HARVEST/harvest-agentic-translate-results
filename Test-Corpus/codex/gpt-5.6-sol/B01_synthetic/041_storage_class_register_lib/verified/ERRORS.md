# Error Surface

Mechanical scans of `c_src/src/` and `c_src/include/` found no error-return
statements or macros, assertions, explicit range checks, null checks, min/max
constants, pointer or length parameters, or enum parameters.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|

There are no rejection rows. `driver(int)` accepts the complete C `int`
domain and returns `void`, so no generic null-pointer, length, or invalid-enum
boundary applies to this API.
