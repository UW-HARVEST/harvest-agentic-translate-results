# Error-surface table

Mechanical source scan covered every `return -1`, `return NULL`, error/sentinel
return, explicit range check, null check, `assert`, and min/max constant in
`../c_src/src/lib.c`. The source contains no enums, length parameters, or
`assert` statements.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `add_node` | `node_count >= MAX_NODES` (`MAX_NODES == 100`) before insertion | returns `-1`; storage/count unchanged | [x] |
| 2 | `find_node_by_id` | no stored node has both `id == requested_id` and nonzero `active` | returns `NULL` | [x] |
| 3 | `calculate_subtree_sum` | `find_node_by_id(node_id) == NULL` (missing or inactive root) | returns `0.0` | [x] |
| 4 | `safe_double_to_int` | `d > (double)INT_MAX` (including positive infinity) | returns `INT_MAX` | [x] |
| 5 | `safe_double_to_int` | `d < (double)INT_MIN` (including negative infinity) | returns `INT_MIN` | [x] |
| 6 | `safe_double_to_int` | `d != d` (NaN) | returns `0` | [x] |

## Generic FFI boundary cases

The pointer-taking C APIs have no explicit null checks. These are still tested,
in isolated subprocesses because the C behavior is a fatal invalid-memory
access rather than a returned error.

| # | function | boundary input | observed C rejection to match | verified |
|---|----------|----------------|-------------------------------|----------|
| 7 | `add_node` | `name == NULL` while capacity remains | process terminates by signal while `strncpy` reads `name` | [x] |
| 8 | `process_string` | `str == NULL` | process terminates by signal while evaluating `*str` | [x] |

There are no public length or enum parameters, so zero/oversized lengths and
out-of-range enum discriminants do not apply.
