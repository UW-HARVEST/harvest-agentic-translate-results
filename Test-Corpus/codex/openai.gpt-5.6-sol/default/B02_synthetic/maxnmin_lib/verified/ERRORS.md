# Error Surface

Derived from every `return`, null check, and explicit min/max check in
`../c_src/src/lib.c`. Conditions that are valid empty-input branches are listed
in `CONFIGS.md` instead.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| E01 | `add_node` | `node_count >= MAX_NODES` (`MAX_NODES` is 100) | `-1` | [x] |
| E02 | `find_node_by_id` | no active stored node has the requested `id` | `NULL` | [x] |
| E03 | `calculate_subtree_sum` | `find_node_by_id(node_id) == NULL` | `0.0` | [x] |
| E04 | `safe_double_to_int` | `d > (double)INT_MAX` (including positive infinity) | `INT_MAX` | [x] |
| E05 | `safe_double_to_int` | `d < (double)INT_MIN` (including negative infinity) | `INT_MIN` | [x] |
| E06 | `safe_double_to_int` | `d != d` (NaN) | `0` | [x] |
| E07 | `maxnmin` | `(param1 % 6) + 1` selects no node (negative non-multiple-of-6 `param1`) | skip name and subtree contributions; return the remaining computed result | [x] |
| E08 | `maxnmin` | `(param2 % 6) + 1` selects no node (negative non-multiple-of-6 `param2`) | skip multiplied-value contribution; return the remaining computed result | [x] |

## Pointer Boundaries

`add_node` and `process_string` dereference their pointer arguments without a
null check. A null pointer therefore has undefined behavior in C, not a defined
error code or sentinel. Differential tests exercise these calls in subprocesses
so a fault cannot terminate the test runner. There are no public length
arguments or enum arguments in this library.

The subprocess tests confirmed matching fault signals for both pointer-taking
entry points under the default and no-default-features configurations.
