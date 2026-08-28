# Error Surface

Mechanical searches covered `c_src/include/lib.h` and `c_src/src/lib.c` for
error-return statements/macros, assertions, null checks, range checks,
min/max constants, enums, and rejection branches.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no rows: `contrast_ratio` accepts two fixed-size `cb_rgb_255`
structures by value, and every bit pattern of each `unsigned char` field is
valid. The C source has no error return, assertion, explicit range check, null
check, pointer, length, or enum. Consequently, null pointers, zero/oversized
lengths, and out-of-range enum values are not representable at this FFI
boundary. Zero-valued channels are valid data and are covered by the valid-path
matrix in `CONFIGS.md`.

- [x] The empty rejection surface and all applicable ABI boundaries are
      covered by differential tests.
