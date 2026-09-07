# Error surface

Rows 1–8 are mechanically derived from explicit checks/default rejection
behavior in `../c_src/src/lib.c`. Row 9 records the required generic null
pointer boundary for the only unchecked pointer parameter.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `divide_op` | `b == 0` | returns `0` | [x] |
| 2 | `modulo_op` | `b == 0` | returns `0` | [x] |
| 3 | `find_node_by_id` | no index `i` in `[0, node_count)` has `node_table[i].id == id` (including `node_count == 0`) | returns `NULL` | [x] |
| 4 | `add_tree_node` | `node_count >= MAX_NODES` (`MAX_NODES == 50`) | returns `-1`; table/count unchanged | [x] |
| 5 | `add_tree_node` | `parent_id != -1` and `find_node_by_id(parent_id) == NULL` or the returned node's `id != parent_id` | returns `-1`; staged slot is written but `node_count` is not incremented | [x] |
| 6 | `calculate_tree_sum` | `find_node_by_id(node_id) == NULL` or the returned node's `id != node_id` | returns `0` | [x] |
| 7 | `parse_operation` | `op_str == NULL` | returns `OP_ADD` (`1`) | [x] |
| 8 | `get_operation_func` | integer `op` is outside `1..=5` (an out-of-range C enum value) | returns the `add_op` function pointer | [x] |
| 9 | `add_tree_node` | `label == NULL` while `node_count < 50` | process receives the same fatal memory-access rejection when `strncpy` reads the null pointer | [x] |

There are no length parameters, assertions, error enums, or additional
explicit min/max range checks in the C source.
