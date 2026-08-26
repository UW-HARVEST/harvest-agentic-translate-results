# Error Surface

Mechanical searches covered every C source and header for `RETURN_ERROR`,
`return -1`, `return NULL`, error identifiers/enums, `assert`, null checks,
explicit range checks, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|

There are **no rows** because the C implementation contains no rejection,
error-return, assertion, null check, explicit range check, error enum, or
min/max constant. Its sole `return` returns the normal unsigned result from
`update_md5`.

## Generic FFI Boundary Audit

- The API has no length arguments and no enum arguments.
- Zero and maximum scalar values are accepted inputs, not errors; they are in
  `CONFIGS.md` and the valid-path differential tests.
- [x] Each pointer is dereferenced or written without a null check. Passing
  null invokes undefined behavior in C and therefore has no C error code or
  sentinel; isolated subprocess tests verify matching C/Rust process
  termination for every pointer position without crashing the test runner.
- Passing storage shorter than the fixed accesses also invokes undefined
  behavior and has no deterministic C result to compare.
- The fixed valid storage requirements are eight writable bytes for
  `tflac_pack_u64le`, one writable `tflac_md5` for `tflac_md5_addsample`, and
  one writable `tflac` plus readable samples through index 135 for
  `update_md5`.
