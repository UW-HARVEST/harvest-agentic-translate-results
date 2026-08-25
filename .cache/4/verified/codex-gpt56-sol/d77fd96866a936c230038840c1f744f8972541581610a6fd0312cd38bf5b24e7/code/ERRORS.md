# Error Surface

The source contains no assertions, error macros, `-1`/`NULL` error returns, or
public min/max constants. Rows 1-5 are the mechanically identified rejection
branches. Rows 6-10 track the additional generic FFI boundaries required by the
verification protocol; the C API does not turn pointer misuse into an error
code.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|---|
| 1 | `is_valid_operation` | `op_char == 0` | `false` | [x] |
| 2 | `is_valid_operation` | `op_char != 0 && op_char < '1'` | `false` | [x] |
| 3 | `is_valid_operation` | `op_char > '5'` | `false` | [x] |
| 4 | `divide_operation` | `b == 0` | `0` | [x] |
| 5 | `modulo_operation` | `b == 0` | `0` | [x] |
| 6 | `perform_computation_with_history` | outer `history == NULL` | invalid dereference; process terminates by signal | [x] |
| 7 | `perform_computation_with_history` | `history_count == NULL` with non-NULL history | invalid dereference; process terminates by signal | [x] |
| 8 | `allocate_results` | `count == 0` | returns the host `calloc(0, sizeof(ComputationResult))` result unchanged | [x] |
| 9 | `allocate_results` | `count == -1` (negative length converts to oversized `size_t`) | returns `NULL` when `calloc` rejects the overflow | [x] |
| 10 | `select_operation`, `get_operation_priority`, `perform_computation_with_history` | enum representation outside `1..=5`, including `0`, `6`, and `UINT_MAX` | selector/perform fall back to addition; priority retains C unsigned arithmetic and conversion | [x] |

