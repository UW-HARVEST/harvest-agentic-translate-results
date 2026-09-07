# Error Surface

Mechanical search covered `return`, `assert`, `if`, `switch`, `case`,
preprocessor conditionals, `NULL`, min/max constants, and error-related names
in `../c_src/include/` and `../c_src/src/`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are **0 rejection branches**. The only explicit `return` is the
unconditional `return;` at the end of the `void long_exec(unsigned int)`
function.

Generic FFI-boundary audit:

- Neither function accepts pointers, lengths, or enums.
- `perform_expensive_operations(void)` accepts no input arguments.
- `long_exec(unsigned int)` accepts the full `unsigned int` range, including
  `0` and `UINT_MAX`; these are valid-path configurations, not error cases.
- The fixed array extent and loop counts are internal constants and cannot be
  supplied out of range by an external caller.

Phase C status: **complete (no applicable error rows)**.
