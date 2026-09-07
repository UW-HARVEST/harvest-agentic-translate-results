# Error and invalid-input surface

Rows 1-2 are the explicit rejection branches mechanically found in
`c_src/src/lib.c`. Rows 3-9 are the generic FFI boundary cases mandated by the
verification protocol. The C API does not report an error code for these
generic cases: it falls back, returns the allocator outcome, or faults.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `divide_operation` | `b == 0` | returns `0` | [x] |
| 2 | `modulo_operation` | `b == 0` | returns `0` | [x] |
| 3 | `select_operation` | enum value `0` (one below `OP_ADD`) | returns `add_operation` | [x] |
| 4 | `select_operation` | enum value `6` (one above `OP_MODULO`) | returns `add_operation` | [x] |
| 5 | `perform_computation_with_history` | enum value outside `1..=5` | computes with `add_operation` | [x] |
| 6 | `allocate_results` | zero length (`count == 0`) | returns exactly the platform `calloc(0, sizeof(ComputationResult))` outcome | [x] |
| 7 | `allocate_results` | oversized/negative lengths (`INT_MAX`, `-1`) | returns exactly the platform `calloc` null/non-null outcome | [x] |
| 8 | `perform_computation_with_history` | outer `history == NULL` | process terminates from invalid pointer dereference | [x] |
| 9 | `perform_computation_with_history` | `history_count == NULL` | process terminates from invalid pointer dereference | [x] |
