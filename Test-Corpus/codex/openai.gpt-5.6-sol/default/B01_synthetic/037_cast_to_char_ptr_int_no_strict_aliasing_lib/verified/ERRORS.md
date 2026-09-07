# Error-Surface Table

Mechanical source scan covered `../c_src/include/driver.h` and
`../c_src/src/driver.c` for error returns, assertions, null/range checks,
error enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|

There are no rejection paths. The only public parameter is a by-value C
`int`; the API has no pointers, lengths, enums, ranges, allocation, or
failure return value. Consequently, the generic null, zero/oversized length,
and invalid-enum cases are not applicable.

Phase C status: **complete (0 applicable rows)**.
