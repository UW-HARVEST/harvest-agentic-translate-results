# Error Surface

Rows 1-9 correspond to distinct explicit rejection/error branches in
`c_src/src/lib.c`. Rows 10-12 are the additional generic FFI boundaries
required by Phase C. Allocation-failure rows are exercised with a one-shot
`malloc` interposer so the exact branch is deterministic.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `create_result_string` | `malloc(64)` returns `NULL` | returns `NULL` | [x] |
| 2 | `safe_add` | `(perms & (READ_PERM \| WRITE_PERM)) != (READ_PERM \| WRITE_PERM)` | prints `Insufficient permissions for addition\n`; returns `0` | [x] |
| 3 | `multiply_with_log` | `create_result_string` returns `NULL` | stores `NULL` in `*log_msg`; returns `0` | [x] |
| 4 | `copy_and_sum` | `src == NULL` | prints `Source pointer is NULL\n`; returns `-1` | [x] |
| 5 | `copy_and_sum` | `malloc(count * sizeof(int))` returns `NULL` | prints `Memory allocation failed\n`; returns `-1` | [x] |
| 6 | `compare_operations` | `op1 == NULL || op2 == NULL` | prints `One or both operation strings are NULL\n`; returns `-1` | [x] |
| 7 | `complexmode` | allocation of `Result` tracker returns `NULL` | prints `Failed to allocate result tracker\n`; returns `-1` | [x] |
| 8 | `complexmode` mode 2 | `log_message == NULL || strcmp(log_message, "") == 0` after `multiply_with_log` | prints `Log message creation failed\n` and `Operation performed: multiplication\n`; returns `0` for the reachable allocation-failure case | [x] |
| 9 | `complexmode` | `mode` is not `1`, `2`, `3`, or `4` | prints `Invalid mode\n`; returns `-1` | [x] |
| 10 | `multiply_with_log` | generic boundary: `log_msg == NULL` (the C code performs an unchecked dereference) | process terminates with `SIGSEGV` | [x] |
| 11 | `copy_and_sum` | generic boundary: `count == 0` with `src == NULL` | the null check wins; prints `Source pointer is NULL\n`; returns `-1` | [x] |
| 12 | `copy_and_sum` | generic boundary: negative/oversized `count` (for example `-1`) with non-null `src` | requested allocation is effectively oversized, allocation fails, prints `Memory allocation failed\n`; returns `-1` | [x] |

Source scan found no assertions, error enums, explicit numeric min/max input
checks, or public enum parameters.
