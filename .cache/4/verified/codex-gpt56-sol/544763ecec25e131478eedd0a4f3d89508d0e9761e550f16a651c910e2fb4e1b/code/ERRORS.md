# Error Surface

Mechanical searches covered every C source/header for `RETURN_ERROR`, error
returns, `assert`, null checks, range checks, enums, `if`, `switch`, and
min/max constants. The only conditional is the valid-path luminance ordering
branch in `cbContrastRatio`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|---|---|---|---|

There are no rejection paths. The sole public API accepts two three-byte
structures by value, whose fields span the complete `unsigned char` domain.
Consequently, pointer-nullability, length, oversized-length, and invalid-enum
boundary cases do not exist at this FFI boundary.

