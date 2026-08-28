# Configuration Surface

The CMake build has no compile-time feature switches. `Cargo.toml` declares no
features, so the only effective configuration is the default/no-feature build.
Rows below enumerate the runtime branches and input shapes exposed by all 11
function symbols plus the two public data symbols. Arithmetic rows constrain
random generation to operations with defined C signed-`int` results.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|-|
| 1 | `add_op` | randomized `a`, `b`, and ignored arguments; defined signed addition including zero and boundaries | [x] |
| 2 | `multiply_op` | randomized `a`, `b`, and ignored arguments; defined signed multiplication including zero and boundaries | [x] |
| 3 | `subtract_op` | randomized `a`, `b`, and ignored arguments; defined signed subtraction including zero and boundaries | [x] |
| 4 | `divide_op` | randomized nonzero divisor and defined quotient; positive/negative/zero dividend shapes | [x] |
| 5 | `modulo_op` | randomized nonzero divisor and defined remainder; positive/negative/zero dividend shapes | [x] |
| 6 | `node_count`, `node_table` | direct FFI access when empty and after insertion; compare count and all `TreeNode` bytes | [x] |
| 7 | `find_node_by_id` | one-node table, matching ID | [x] |
| 8 | `find_node_by_id` | many-node table, match at first, middle, and last positions | [x] |
| 9 | `find_node_by_id` | duplicate IDs; returns the first matching slot | [x] |
| 10 | `add_tree_node` | root (`parent_id == -1`) with empty label | [x] |
| 11 | `add_tree_node` | root with label length `1..=30` | [x] |
| 12 | `add_tree_node` | root with label length exactly 31 | [x] |
| 13 | `add_tree_node` | root with label length greater than 31; stored label is truncated to 31 bytes and NUL-terminated | [x] |
| 14 | `add_tree_node` | existing parent with empty left slot; child becomes `left_child_id` | [x] |
| 15 | `add_tree_node` | existing parent with occupied left and empty right slot; child becomes `right_child_id` | [x] |
| 16 | `add_tree_node` | existing parent with both child slots occupied; node is appended but parent links are unchanged | [x] |
| 17 | `add_tree_node`, `find_node_by_id` | duplicate node ID is accepted; lookup resolves to first insertion | [x] |
| 18 | `calculate_tree_sum` | leaf node | [x] |
| 19 | `calculate_tree_sum` | node with left child only | [x] |
| 20 | `calculate_tree_sum` | node with right child only | [x] |
| 21 | `calculate_tree_sum` | node with both children | [x] |
| 22 | `calculate_tree_sum` | multi-level tree; recursively traverses left and right links | [x] |
| 23 | `parse_operation` | string contains `+` (including strings containing later operators); first-precedence result `OP_ADD` | [x] |
| 24 | `parse_operation` | no `+`, contains `*`; result `OP_MULTIPLY` | [x] |
| 25 | `parse_operation` | no `+` or `*`, contains `-`; result `OP_SUBTRACT` | [x] |
| 26 | `parse_operation` | no `+`, `*`, or `-`, contains `/`; result `OP_DIVIDE` | [x] |
| 27 | `parse_operation` | no earlier operator, contains `%`; result `OP_MODULO` | [x] |
| 28 | `parse_operation` | empty or non-operator string; fallback `OP_ADD` | [x] |
| 29 | `get_operation_func` | `OP_ADD` (`1`); invoke returned pointer over randomized inputs | [x] |
| 30 | `get_operation_func` | `OP_MULTIPLY` (`2`); invoke returned pointer over randomized inputs | [x] |
| 31 | `get_operation_func` | `OP_SUBTRACT` (`3`); invoke returned pointer over randomized inputs | [x] |
| 32 | `get_operation_func` | `OP_DIVIDE` (`4`); invoke returned pointer over randomized valid inputs | [x] |
| 33 | `get_operation_func` | `OP_MODULO` (`5`); invoke returned pointer over randomized valid inputs | [x] |
| 34 | `inreftree` | `param2 != 0`, nonnegative defined tree sum with `sum % 4 == 0`; target ID 2, add operation | [x] |
| 35 | `inreftree` | `param2 != 0`, nonnegative defined tree sum with `sum % 4 == 1`; target ID 2, multiply operation | [x] |
| 36 | `inreftree` | `param2 != 0`, nonnegative defined tree sum with `sum % 4 == 2`; target ID 2, subtract operation | [x] |
| 37 | `inreftree` | `param2 != 0`, nonnegative defined tree sum with `sum % 4 == 3`; target ID 2, modulo operation | [x] |
| 38 | `inreftree` | `param2 == 0`, nonnegative defined tree sum with `sum % 4 == 0`; target falls back to ID 1, add operation | [x] |
| 39 | `inreftree` | `param2 == 0`, nonnegative defined tree sum with `sum % 4 == 1`; target falls back to ID 1, multiply operation | [x] |
| 40 | `inreftree` | `param2 == 0`, nonnegative defined tree sum with `sum % 4 == 2`; target falls back to ID 1, subtract operation | [x] |
| 41 | `inreftree` | `param2 == 0`, nonnegative defined tree sum with `sum % 4 == 3`; target falls back to ID 1, modulo operation | [x] |
