# Error Surface

Mechanical search covered every C source/header occurrence of `return`,
`RETURN_ERROR`, `NULL`, `assert`, comparisons, `if`, `switch`, `case`, enums,
and min/max constants. The sole public function accepts one `double`, returns
`void`, dereferences no caller pointer, and contains no rejection or error
branch. Consequently, the error surface has zero rows.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

Generic FFI boundaries for null pointers, lengths, and enum discriminants do
not apply to `void driver(double)`. Every one of the 2^64 IEEE-754 bit patterns
is a valid C `double` input and is covered by the valid-path surface.
