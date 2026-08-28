# Error Surface

Mechanical searches of `../c_src/include/lib.h` and
`../c_src/src/lib.c` found no error-return macros, error sentinels, assertions,
explicit range checks, null checks, enums, pointer parameters, length
parameters, or min/max constants. The only public function accepts every value
of its complete `uint16_t` domain, so there are no C rejection rows.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

Generic boundary audit: `0`, `UINT16_MAX`, and every value between them pass
the exhaustive FFI differential test. Null pointers, lengths, oversized
lengths, enums, and one-past-range values are not representable in this API.
