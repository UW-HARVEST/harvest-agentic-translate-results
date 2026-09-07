# Error Surface

Mechanical searches covered `c_src/include`, `c_src/src`, and
`c_src/CMakeLists.txt` for error returns, null returns, assertions, enums,
conditionals, range comparisons, null checks, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|

There are no rejection paths in the C API. `max_size_frame` accepts all three
arguments as the complete `uint32_t` domain and always returns a `uint32_t`.
The API contains no pointers, lengths, enum parameters, documented scalar
ranges, assertions, or error sentinels. Generic scalar boundaries
(`0`, `1`, equality boundaries, `UINT32_MAX`, and overflow-producing operands)
are therefore valid inputs and are covered by the valid-path matrix.

- [x] Phase C complete: the error table has no C rejection rows, and
  `phase_c_generic_scalar_boundaries_are_valid_and_match` passes through both
  shared-library FFI boundaries.
