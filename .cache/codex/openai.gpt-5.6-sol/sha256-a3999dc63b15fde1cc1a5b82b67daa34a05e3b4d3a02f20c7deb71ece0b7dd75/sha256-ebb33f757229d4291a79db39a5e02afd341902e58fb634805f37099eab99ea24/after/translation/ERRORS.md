# Error surface

Mechanical scans of `../c_src/include/` and `../c_src/src/` found no
error-return statements or macros, error enums, assertions, explicit input
range rejection, null checks, pointer parameters, length parameters, or enum
parameters.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no rejection rows to test. `tritanopia` takes a three-byte struct by
value, and every possible bit pattern of that struct is a valid input. Generic
null-pointer, zero/oversized-length, and out-of-range-enum cases are therefore
not representable at this FFI boundary.

Completion check: [x] the complete error surface is covered (empty set).
