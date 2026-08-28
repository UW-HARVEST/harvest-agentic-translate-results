# Error Surface

Rows 1-8 are the distinct C rejection, sentinel, null-check, and default-enum
branches found mechanically in `src/lib.c`. Rows 9-10 are the additional
generic FFI boundaries required by Phase C. The C API has no length argument.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|-|
| 1 | `divide_op` | `b == 0` | `0` | [x] |
| 2 | `modulo_op` | `b == 0` | `0` | [x] |
| 3 | `find_node_by_id` | no `node_table[i].id == id` for `0 <= i < node_count` (including an empty table) | `NULL` | [x] |
| 4 | `add_tree_node` | `node_count >= MAX_NODES` (`MAX_NODES == 50`) | `-1`; table and count unchanged | [x] |
| 5 | `add_tree_node` | `parent_id != -1` and `find_node_by_id(parent_id) == NULL` (the defensive `parent->id != parent_id` alternative is unreachable without concurrent mutation) | `-1`; candidate slot is written but `node_count` is unchanged | [x] |
| 6 | `calculate_tree_sum` | `find_node_by_id(node_id) == NULL` (the defensive `node->id != node_id` alternative is unreachable without concurrent mutation) | `0` | [x] |
| 7 | `parse_operation` | `op_str == NULL` | `OP_ADD` (`1`) | [x] |
| 8 | `get_operation_func` | `(int)op` is outside `1..=5`, including one-past values and arbitrary FFI integers | pointer to `add_op`; invoking it returns wrapping C `int` addition for defined inputs | [x] |
| 9 | `add_tree_node` | `label == NULL` | no C rejection; unchecked `strncpy` access terminates the child process with a memory fault | [x] |
| 10 | all integer APIs | zero, `INT_MIN`, `INT_MAX`, and one-step operation boundaries where C arithmetic remains defined | exact same integer or sentinel as C | [x] |

Signed overflow, `INT_MIN / -1`, cyclic trees, invalid global `node_count`
values, and negative `inreftree` sums invoke undefined behavior in the C source.
They have no stable C result to compare and are not valid differential rows.
