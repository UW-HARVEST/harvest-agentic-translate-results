# Error Surface

The public header exposes only `jumpnode(int, int, int, int)`. Static storage
starts empty, and no public function calls `initialize_test_data` or `add_node`,
so the three node-dependent modes always reject through their missing-node
branches.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `jumpnode` | `operation_mode == 0001` and `find_node_by_id(node_id) == NULL` (always true through the public API because `node_count == 0`) | `STATUS_ERROR \| 0020` = 18 | [x] |
| 2 | `jumpnode` | `operation_mode == 0002` and `find_node_by_id(node_id) == NULL` (always true through the public API because `node_count == 0`) | `STATUS_ERROR \| 0040` = 34 | [x] |
| 3 | `jumpnode` | `operation_mode == 0004` and `find_node_by_id(node_id) == NULL` (always true through the public API because `node_count == 0`) | `STATUS_ERROR \| 0100` = 66 | [x] |
| 4 | `jumpnode` | `operation_mode` is any integer other than `0001`, `0002`, `0003`, or `0004` | `STATUS_ERROR \| 0200` = 130 | [x] |

## Internal Check Audit

These source checks are not additional public-input rejection paths:

| function | source condition | behavior | public reachability |
|----------|------------------|----------|---------------------|
| `find_node_by_id` | scan reaches `node_count` without an ID match | returns `NULL`; represented by rows 1-3 where public code rejects it | reachable |
| `add_node` | `node_count >= MAX_NODES` (`MAX_NODES == 100`) | returns `STATUS_ERROR` = 2 | unreachable: function is static and has no public caller |
| `safe_double_to_int` | `value > 2147483647.0` | clamps to 2147483647 before conversion; not an error | unreachable: node-dependent callers reject first |
| `safe_double_to_int` | `value < -2147483648.0` | clamps to -2147483648 before conversion; not an error | unreachable: node-dependent callers reject first |
| mode `0001` parent lookup | `find_node_by_id(current_node->parent_id) == NULL` | stops traversal and returns the accumulated value; not an error | unreachable: initial node lookup rejects first |

There are no pointer, length, or enum parameters in the public ABI. Generic
FFI boundaries therefore reduce to all four `int` arguments: zero, signed
extrema, and operation selectors immediately outside the valid range.
