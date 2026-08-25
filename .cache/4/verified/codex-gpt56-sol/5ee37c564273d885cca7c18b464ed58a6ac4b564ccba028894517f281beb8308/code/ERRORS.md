# Error Surface

Mechanically derived from every `if`/`switch` error condition and error return
in `c_src/src/lib.c`. Allocator and formatter failures are retained even when
ordinary inputs cannot reliably trigger them.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `create_result_string` | `malloc(64) == NULL` | returns `NULL` | [x] |
| 2 | `safe_add` | `(perms & (0400 \| 0200)) != (0400 \| 0200)` | prints `Insufficient permissions for addition`; returns `0` | [x] |
| 3 | `multiply_with_log` | `create_result_string("multiply", a * b) == NULL` | stores `NULL` through `log_msg`; returns `0` | [x] |
| 4 | `copy_and_sum` | `src == NULL` (including `count == 0`) | prints `Source pointer is NULL`; returns `-1` | [x] |
| 5 | `copy_and_sum` | `malloc(count * sizeof(int)) == NULL` | prints `Memory allocation failed`; returns `-1` | [x] |
| 6 | `compare_operations` | `op1 == NULL \|\| op2 == NULL` (left, right, and both null) | prints `One or both operation strings are NULL`; returns `-1` | [x] |
| 7 | `complexmode` | initial `malloc(sizeof(Result)) == NULL` | prints `Failed to allocate result tracker`; returns `-1` | [x] |
| 8 | `complexmode` mode 2 | nested `create_result_string` allocation fails, making `log_message == NULL` | prints `Log message creation failed` and `Operation performed: multiplication`; returns `0` | [x] |
| 9 | `complexmode` mode 2 | `log_message != NULL` and `strcmp(log_message, "") == 0` | prints `Log message creation failed` and `Operation performed: multiplication`; returns multiplication result | [x] |
| 10 | `complexmode` | `mode` is not `1`, `2`, `3`, or `4` | prints `Invalid mode`; returns `-1` | [x] |

## Generic FFI Boundaries

These are required boundary probes even though the C implementation does not
explicitly reject all of them:

| # | function | boundary | expected C behavior | [ ] |
|---|----------|----------|---------------------|-----|
| G1 | `create_result_string` | `op == NULL` | behavior observed through the C shared object and matched exactly | [x] |
| G2 | `multiply_with_log` | `log_msg == NULL` | process termination behavior matched exactly | [x] |
| G3 | `copy_and_sum` | zero, negative, and oversized `count` | exact C return/termination behavior matched | [x] |
| G4 | `compare_operations` | either and both pointers null | returns `-1` for every null placement | [x] |
| G5 | all integer APIs | `INT_MIN`, `INT_MAX`, and one-step branch boundaries | exact C results matched | [x] |
| G6 | `complexmode` | out-of-range mode values including `0`, `5`, `INT_MIN`, and `INT_MAX` | returns `-1` | [x] |

There are no C enum parameters or documented numeric min/max constants beyond
the fixed permission masks (`0400`, `0200`, and `0100`).
