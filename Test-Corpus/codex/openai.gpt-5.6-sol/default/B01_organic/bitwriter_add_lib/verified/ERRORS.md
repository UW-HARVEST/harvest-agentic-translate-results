# Error Surface

Mechanical searches covered `RETURN_ERROR`, negative and null returns,
`assert`, null checks, range checks, comparison branches, and min/max tokens
in `../c_src/include` and `../c_src/src`.

The C API contains no explicit rejection or error branch. `bitwriter_add`
unconditionally returns `0`; therefore the mechanically derived table has no
rows.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|

Generic FFI boundary behavior (null `bw`, zero `bits`, and values beyond the
64-bit width) is covered separately by the differential tests. There are no
length parameters or enum parameters in this API.

- [x] Null `bw`: C and Rust terminate with the same process status.
- [x] Zero width: C and Rust return and mutate state identically.
- [x] Widths above 64 and out-of-range `bw.bits`: C and Rust return and mutate
  state identically.
- [x] Oversized lengths: not applicable; the API has no length parameter.
- [x] Out-of-range enums: not applicable; the API has no enum parameter.
