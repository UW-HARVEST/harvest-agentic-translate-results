# Error Surface

The only public ABI is `jumpnode(int, int, int, int)`. Static storage has
zero-initialized `node_count`, and no exported function calls `add_node` or
`initialize_test_data`, so no external caller can create a node.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| E1 | `jumpnode` | `operation_mode == 0001` and no stored node has `id == node_id` (the externally reachable state for every `node_id`) | `STATUS_ERROR \| 0020` = 18 | [x] |
| E2 | `jumpnode` | `operation_mode == 0002` and no stored node has `id == node_id` (the externally reachable state for every `node_id`) | `STATUS_ERROR \| 0040` = 34 | [x] |
| E3 | `jumpnode` | `operation_mode == 0004` and no stored node has `id == node_id` (the externally reachable state for every `node_id`) | `STATUS_ERROR \| 0100` = 66 | [x] |
| E4 | `jumpnode` | `operation_mode` is not one of `0001`, `0002`, `0003`, or `0004`; includes values one below/above the range and arbitrary out-of-range enum-like integers | `STATUS_ERROR \| 0200` = 130 | [x] |

## Static source checks

The mechanical scan also found these checks in static helpers. They are not
separate public error-surface rows because neither the helpers nor any state
mutation API are exported; therefore no call through either shared library can
construct their preconditions.

| function | exact condition | C behavior | public reachability |
|----------|-----------------|------------|---------------------|
| `find_node_by_id` | no element in `node_storage[0..node_count]` has the requested ID | returns `NULL` | exercised by E1-E3 |
| `add_node` | `node_count >= MAX_NODES` where `MAX_NODES == 100` | returns `STATUS_ERROR` = 2 | unreachable; function is static and never called by public code |
| `safe_double_to_int` | `value > 2147483647.0` | clamps to `2147483647.0` before conversion | unreachable; only mode 1/4 call sites require a node |
| `safe_double_to_int` | `value < -2147483648.0` | clamps to `-2147483648.0` before conversion | unreachable; only mode 1/4 call sites require a node |

There are no public pointer or length parameters, assertions, error enums,
`return -1` statements, or error macros. Generic null-pointer and zero/oversized
length cases do not exist in this ABI.
