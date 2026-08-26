# Error Surface

Mechanical searches covered `return`, `RETURN_ERROR`, `assert`, `abort`,
`exit`, `errno`, `NULL`, min/max constants, comparisons, and all conditionals
in `c_src/include/driver.h` and `c_src/src/driver.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | Tested |
|---|----------|---------------------------------------------|-------------------|--------|

There are no rows: `driver(int)` accepts every value representable by the C
`int` parameter and has no error return or rejection path. The only comparison
in the implementation is the private `print_hex` loop bound
`i < sizeof(house_t)`; callers cannot control its pointer or length.

Generic FFI boundary audit:

- Null pointers: not applicable; the public API has no pointer parameters.
- Zero length: not applicable; the public API has no length parameter.
- Oversized length: not applicable; the public API has no length parameter.
- Out-of-range enums: not applicable; the public API has no enum parameter.
- One past a documented range: not applicable; no narrower range is documented
  for the `int` parameter.

- [x] Every C rejection branch has a differential test (vacuously: none exist).
