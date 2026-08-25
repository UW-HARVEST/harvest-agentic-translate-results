# Error Surface

The mechanical scan covered `c_src/include/driver.h` and
`c_src/src/driver.c` for error returns, null checks, assertions, range checks,
enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no rows: `driver(int, int)` returns `void`, accepts no pointers,
lengths, or enums, and contains no rejection or error path.

Generic FFI boundary coverage still probes `INT_MIN`, `-1`, `0`, `1`, and
`INT_MAX`. Null-pointer, length, and invalid-enum probes do not apply to this
signature.

