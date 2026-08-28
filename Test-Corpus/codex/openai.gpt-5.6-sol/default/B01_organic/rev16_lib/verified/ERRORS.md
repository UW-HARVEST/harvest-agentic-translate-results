# Error Surface

The mechanical scan covered `../c_src/include/` and `../c_src/src/` for error
returns, assertions, branches, null checks, range checks, enums, and min/max
constants. The public API accepts one `uint32_t` by value and contains no
rejection or error path.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no rows to test in Phase C. Pointer, length, and enum boundary cases
do not apply to this API.
