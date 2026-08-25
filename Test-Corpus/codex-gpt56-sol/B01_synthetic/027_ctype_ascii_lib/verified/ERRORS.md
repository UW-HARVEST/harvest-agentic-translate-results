# Error Surface

The source scan covered `c_src/include/driver.h` and `c_src/src/driver.c` for
error returns, `return -1`, `return NULL`, assertions, error enums, explicit
range checks, null checks, and min/max constants. It found no rejection paths.
`driver` returns `void` and accepts one by-value `char`, so pointer, length, and
enum boundary cases do not exist in this API.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|

All 256 bit patterns are representable by the `char` parameter and are covered
as valid-path configurations in `CONFIGS.md`; there is no value one step beyond
the ABI type's range that can cross this FFI boundary.

Phase C status: complete (zero applicable rejection or generic boundary rows).
