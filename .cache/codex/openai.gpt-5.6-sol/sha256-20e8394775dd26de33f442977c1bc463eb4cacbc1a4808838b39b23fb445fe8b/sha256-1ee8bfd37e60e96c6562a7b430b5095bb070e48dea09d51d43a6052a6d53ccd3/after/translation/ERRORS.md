# Error Surface

Mechanically derived from every `if`/`switch` rejection, null check, allocation
check, and error return in `../c_src/src/lib.c`. The source has no assertions,
error enums, or explicit numeric min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `create_result_string` | `malloc(64)` returns `NULL` | returns `NULL` | [x] |
| 2 | `safe_add` | `(perms & (0400 \| 0200)) != (0400 \| 0200)` | prints `Insufficient permissions for addition\n`; returns `0` | [x] |
| 3 | `multiply_with_log` | `create_result_string("multiply", a * b)` returns `NULL` | stores `NULL` in `*log_msg`; returns `0` | [x] |
| 4 | `copy_and_sum` | `src == NULL` | prints `Source pointer is NULL\n`; returns `-1` | [x] |
| 5 | `copy_and_sum` | `malloc(count * sizeof(int))` returns `NULL` | prints `Memory allocation failed\n`; returns `-1` | [x] |
| 6 | `compare_operations` | `op1 == NULL \|\| op2 == NULL` | prints `One or both operation strings are NULL\n`; returns `-1` | [x] |
| 7 | `complexmode` | result-tracker `malloc(sizeof(Result))` returns `NULL` | prints `Failed to allocate result tracker\n`; returns `-1` | [x] |
| 8 | `complexmode` | `mode` is not `1`, `2`, `3`, or `4` | prints `Invalid mode\n`; returns `-1` | [x] |
| 9 | `complexmode` | `mode == 2` and log allocation fails, making `log_message == NULL` | prints `Log message creation failed\n` and the operation line; returns `0` | [x] |

Generic FFI boundaries additionally exercised by Phase C:

- null pointer arguments, including the unchecked `multiply_with_log` output
  pointer;
- zero, negative-as-oversized, and ordinary positive `copy_and_sum` lengths;
- permission masks one bit short of the required mask;
- out-of-range mode values immediately below and above the valid range, plus
  full-width integer values.
