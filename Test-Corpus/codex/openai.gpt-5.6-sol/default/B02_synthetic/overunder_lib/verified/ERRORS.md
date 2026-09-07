# Error and rejection surface

Mechanically derived from every explicit `if`/`else if` range or special-value
check, every error-style `switch` default, and the pointer-only ABI boundary in
`src/lib.c`. There are no `assert` calls, error enums, length parameters,
explicit null checks, `RETURN_ERROR` uses, `return NULL`, or `return -1`
statements outside the `process_with_fallthrough` default branch.

The null-pointer rows are required generic FFI-boundary cases. The C source
does not reject them explicitly: its `memcpy` call has undefined C-language
semantics, so the differential test isolates each call in a subprocess and
requires Rust to reproduce the actual C shared object's process result on this
platform.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|-|
| 1 | `safe_double_to_int` | `d > (double)INT_MAX` (including `+INFINITY`) | `INT_MAX` | [x] |
| 2 | `safe_double_to_int` | `d < (double)INT_MIN` (including `-INFINITY`) | `INT_MIN` | [x] |
| 3 | `safe_double_to_int` | `isnan(d)` after both range comparisons are false | `0` | [x] |
| 4 | `process_with_fallthrough` | `code < 0` or `code > 5` (the `default` branch; includes `-1` and `6`, one step beyond the accepted cases) | `-1` | [x] |
| 5 | `copy_data_block` | `dest == NULL`, `src` points to a valid `DataBlock` | abnormal process termination matching the C `.so` | [x] |
| 6 | `copy_data_block` | `src == NULL`, `dest` points to a valid `DataBlock` | abnormal process termination matching the C `.so` | [x] |
| 7 | `copy_data_block` | `dest == NULL && src == NULL` | abnormal process termination matching the C `.so` | [x] |

Generic boundary applicability:

- Zero and oversized lengths: not applicable; no exported function accepts a
  length.
- Out-of-range enum representations: not applicable; the ABI contains no enum.
  The unrestricted integer `code` default is covered by row 4.
- Null pointers: only `copy_data_block` accepts pointers; rows 5-7 cover all
  null combinations.
