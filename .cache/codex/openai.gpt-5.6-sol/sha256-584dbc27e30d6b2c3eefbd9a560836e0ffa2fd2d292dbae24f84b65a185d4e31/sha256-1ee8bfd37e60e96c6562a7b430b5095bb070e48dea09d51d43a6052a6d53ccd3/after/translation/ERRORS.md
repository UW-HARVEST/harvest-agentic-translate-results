# Error Surface

The source scan covered `include/lib.h` and `src/lib.c` for error returns,
sentinels, assertions, null/range checks, enums, and min/max constants. There
are no rejection paths: `encode_quant` accepts six scalar C `int` values and
always returns a C `int`. Every conditional in `src/lib.c` selects a valid
calculation path rather than rejecting input.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|

Generic FFI boundary cases involving pointers, lengths, or enums are not
applicable because the API has none. Full-width integer boundaries, including
`INT_MIN` and `INT_MAX`, are covered by the differential boundary test.

