# Configuration surface

The C library has no compile-time/runtime feature flags. Its runtime axes are:
operation selector, tree occupancy/topology, label length, lookup position,
parse precedence, target fallback, and `tree_sum % 4`.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `add_op` | arbitrary signed `int` operands; unused arguments vary | [x] |
| 2 | `multiply_op` | arbitrary signed `int` operands without C signed-overflow cases; unused arguments vary | [x] |
| 3 | `subtract_op` | arbitrary signed `int` operands without C signed-overflow cases; unused arguments vary | [x] |
| 4 | `divide_op` | nonzero positive/negative divisor, including magnitude-one and truncation cases | [x] |
| 5 | `modulo_op` | nonzero positive/negative divisor, including magnitude-one and signed-remainder cases | [x] |
| 6 | `find_node_by_id` | one node; match at first position | [x] |
| 7 | `find_node_by_id` | many nodes; match in a middle position | [x] |
| 8 | `find_node_by_id` | many nodes; match at last position | [x] |
| 9 | `add_tree_node` | root (`parent_id == -1`) + empty label | [x] |
| 10 | `add_tree_node` | root + label length `1..=30` | [x] |
| 11 | `add_tree_node` | root + label length exactly 31 | [x] |
| 12 | `add_tree_node` | root + label length greater than 31 (truncated to 31) | [x] |
| 13 | `add_tree_node` | existing parent with empty left slot + empty label | [x] |
| 14 | `add_tree_node` | existing parent with empty left slot + label length `1..=30` | [x] |
| 15 | `add_tree_node` | existing parent with empty left slot + label length exactly 31 | [x] |
| 16 | `add_tree_node` | existing parent with empty left slot + label length greater than 31 | [x] |
| 17 | `add_tree_node` | existing parent with left occupied/right empty + empty label | [x] |
| 18 | `add_tree_node` | existing parent with left occupied/right empty + label length `1..=30` | [x] |
| 19 | `add_tree_node` | existing parent with left occupied/right empty + label length exactly 31 | [x] |
| 20 | `add_tree_node` | existing parent with left occupied/right empty + label length greater than 31 | [x] |
| 21 | `add_tree_node` | existing parent with both child slots occupied + empty label; new node remains unattached | [x] |
| 22 | `add_tree_node` | existing parent with both child slots occupied + label length `1..=30`; new node remains unattached | [x] |
| 23 | `add_tree_node` | existing parent with both child slots occupied + label length exactly 31; new node remains unattached | [x] |
| 24 | `add_tree_node` | existing parent with both child slots occupied + label length greater than 31; new node remains unattached | [x] |
| 25 | `calculate_tree_sum` | leaf node (no children) | [x] |
| 26 | `calculate_tree_sum` | node with left child only | [x] |
| 27 | `calculate_tree_sum` | node with right child only | [x] |
| 28 | `calculate_tree_sum` | node with both children | [x] |
| 29 | `calculate_tree_sum` | nested descendants (recursive depth greater than one), mixed signed values without overflow | [x] |
| 30 | `parse_operation` | non-null string containing `+` (including strings also containing lower-precedence operators) | [x] |
| 31 | `parse_operation` | no `+`, contains `*` (possibly also `-`, `/`, `%`) | [x] |
| 32 | `parse_operation` | no `+`/`*`, contains `-` (possibly also `/`, `%`) | [x] |
| 33 | `parse_operation` | no `+`/`*`/`-`, contains `/` (possibly also `%`) | [x] |
| 34 | `parse_operation` | no `+`/`*`/`-`/`/`, contains `%` | [x] |
| 35 | `parse_operation` | empty or nonempty string containing no recognized operation character | [x] |
| 36 | `get_operation_func` + returned function | `OP_ADD` (`1`) | [x] |
| 37 | `get_operation_func` + returned function | `OP_MULTIPLY` (`2`) | [x] |
| 38 | `get_operation_func` + returned function | `OP_SUBTRACT` (`3`) | [x] |
| 39 | `get_operation_func` + returned function | `OP_DIVIDE` (`4`) | [x] |
| 40 | `get_operation_func` + returned function | `OP_MODULO` (`5`) | [x] |
| 41 | `inreftree` | `param2 != 0` selects node 2; nonnegative tree sum has remainder 0 (`+`) | [x] |
| 42 | `inreftree` | `param2 != 0` selects node 2; nonnegative tree sum has remainder 1 (`*`) | [x] |
| 43 | `inreftree` | `param2 != 0` selects node 2; nonnegative tree sum has remainder 2 (`-`) | [x] |
| 44 | `inreftree` | `param2 != 0` selects node 2; nonnegative tree sum has remainder 3 (`%`) | [x] |
| 45 | `inreftree` | `param2 == 0` triggers target fallback to node 1; nonnegative tree sum has remainder 0 (`+`) | [x] |
| 46 | `inreftree` | `param2 == 0` triggers target fallback to node 1; nonnegative tree sum has remainder 1 (`*`) | [x] |
| 47 | `inreftree` | `param2 == 0` triggers target fallback to node 1; nonnegative tree sum has remainder 2 (`-`) | [x] |
| 48 | `inreftree` | `param2 == 0` triggers target fallback to node 1; nonnegative tree sum has remainder 3 (`%`) | [x] |
| 49 | `inreftree` | `param2 != 0` selects node 2; negative tree sum has C remainder `-1` and falls through to addition | [x] |
| 50 | `inreftree` | `param2 != 0` selects node 2; negative tree sum has C remainder `-2` and falls through to addition | [x] |
| 51 | `inreftree` | `param2 != 0` selects node 2; negative tree sum has C remainder `-3` and falls through to addition | [x] |
| 52 | `inreftree` | `param2 == 0` triggers target fallback to node 1; negative tree sum has C remainder `-1` and falls through to addition | [x] |
| 53 | `inreftree` | `param2 == 0` triggers target fallback to node 1; negative tree sum has C remainder `-2` and falls through to addition | [x] |
| 54 | `inreftree` | `param2 == 0` triggers target fallback to node 1; negative tree sum has C remainder `-3` and falls through to addition | [x] |
| 55 | `node_count`, `node_table` | exported globals observed after zero-node/reset state | [x] |
| 56 | `node_count`, `node_table` | exported globals observed after one and many successful insertions | [x] |

No C or Rust binary executable is built; only shared libraries are produced.
Cargo declares no features, so the sole feature combination is the default
empty feature set (equivalent to `--no-default-features`).
