# Configuration Surface

Mechanically derived from all public dynamic entry points and each branch or
input shape distinguished by `../c_src/src/lib.c` and the libc operation it
invokes. Error branches are tracked separately in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `create_result_string` | empty operation string; formatted output fits in 64 bytes; randomized integer values | [x] |
| 2 | `create_result_string` | non-empty operation string; formatted output fits in 64 bytes; randomized string lengths and integer values | [x] |
| 3 | `create_result_string` | operation string makes formatted output reach or exceed 64 bytes, exercising `snprintf` truncation | [x] |
| 4 | `check_permissions` | `required == 0`, which always satisfies the subset comparison | [x] |
| 5 | `check_permissions` | permissions exactly equal the nonzero required mask | [x] |
| 6 | `check_permissions` | permissions contain the nonzero required mask plus extra bits | [x] |
| 7 | `check_permissions` | permissions miss at least one required bit | [x] |
| 8 | `safe_add` | both `READ_PERM` and `WRITE_PERM` set, with optional extra bits; randomized signed operands | [x] |
| 9 | `multiply_with_log` | non-null output pointer and successful log allocation; randomized signed operands | [x] |
| 10 | `copy_and_sum` | non-null source and `count == 0` (empty input) | [x] |
| 11 | `copy_and_sum` | non-null source and `count == 1` | [x] |
| 12 | `copy_and_sum` | non-null source and `count > 1`; randomized counts and elements | [x] |
| 13 | `compare_operations` | two non-null equal strings, including empty strings | [x] |
| 14 | `compare_operations` | two non-null unequal strings where `op1` sorts before `op2` | [x] |
| 15 | `compare_operations` | two non-null unequal strings where `op1` sorts after `op2` | [x] |
| 16 | `complexmode` | `mode == 1`: fixed `0644` permissions authorize addition | [x] |
| 17 | `complexmode` | `mode == 2`: multiplication with successful non-empty log creation | [x] |
| 18 | `complexmode` | `mode == 3`: fixed three-element array passed through `copy_and_sum` | [x] |
| 19 | `complexmode` | `mode == 4`: fixed `0644` permissions lack `0100`, selecting the additive branch | [x] |

`Cargo.toml` declares no features. The complete feature matrix therefore has
one member: no default features and no explicitly enabled features.
