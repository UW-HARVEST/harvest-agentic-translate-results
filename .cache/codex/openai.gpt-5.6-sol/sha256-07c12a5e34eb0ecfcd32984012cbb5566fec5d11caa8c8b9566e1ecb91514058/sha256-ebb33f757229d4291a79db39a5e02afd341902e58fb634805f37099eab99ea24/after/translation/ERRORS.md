# Error Surface

Mechanical searches covered `RETURN_ERROR`, `return -1`, `return NULL`,
`assert`, explicit `if`/`switch` rejection branches, null checks, range checks,
error enums, and min/max constants in `../c_src/include/` and
`../c_src/src/`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are no rows: `float2half(float)` accepts every possible 32-bit object
representation of its scalar `float` argument and always returns a `uint16_t`.
The public C API has no pointers, lengths, enums, option setters, assertions,
or error/sentinel returns, so generic null-pointer, zero/oversized-length, and
out-of-range-enum cases are not applicable.
