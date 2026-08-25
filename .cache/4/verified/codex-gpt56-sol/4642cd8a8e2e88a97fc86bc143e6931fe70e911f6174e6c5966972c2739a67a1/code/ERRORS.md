# Error-Surface Table

Mechanical searches covered `c_src/include/driver.h` and
`c_src/src/driver.c` for return statements, error macros, assertions, null
checks, range checks, min/max constants, conditionals, switches, and enums.
The sole public function returns `void` and contains none of those constructs,
so the C library has no rejection paths.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|

## Generic FFI Boundaries

`driver` accepts one by-value C `int`. It has no pointers, lengths, enum
parameters, documented restricted range, or error result. Consequently null
pointers, zero/oversized lengths, and out-of-range enum values are not
constructible for this API. Zero and the full representable `c_int` boundary
are valid inputs and are covered by the configuration-surface test.

