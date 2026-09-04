# Error surface

Mechanical searches covered `RETURN_ERROR`, `return -1`, `return NULL`,
`assert`, enums, null checks, explicit range checks, and min/max constants in
`../c_src/include` and `../c_src/src`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

There are no rejection branches. The only public API, `tritanopia`, accepts a
three-byte struct by value and returns a three-byte struct by value. It has no
pointers, lengths, enums, option fields, or documented restricted ranges, so
the generic null-pointer, zero/oversized-length, and out-of-range-enum cases
are not applicable.

Phase C status: complete; there are zero applicable error rows.
