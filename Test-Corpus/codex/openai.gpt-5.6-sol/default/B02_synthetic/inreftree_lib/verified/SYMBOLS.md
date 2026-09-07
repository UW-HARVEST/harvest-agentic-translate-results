# Dynamic symbol surface

Mechanical source: `nm -D --defined-only ../c_src/build/libharvest-work-9ODZOF.so`.
The Rust comparison is `nm -D --defined-only target/release/libinreftree_lib.so`.

| # | C symbol | kind | Rust export |
|---|----------|------|-------------|
| 1 | `add_op` | function | present |
| 2 | `multiply_op` | function | present |
| 3 | `subtract_op` | function | present |
| 4 | `divide_op` | function | present |
| 5 | `modulo_op` | function | present |
| 6 | `find_node_by_id` | function | present |
| 7 | `add_tree_node` | function | present |
| 8 | `calculate_tree_sum` | function | present |
| 9 | `parse_operation` | function | present |
| 10 | `get_operation_func` | function | present |
| 11 | `inreftree` | function | present |
| 12 | `node_table` | object (`TreeNode[50]`) | present |
| 13 | `node_count` | object (`int`) | present |

Missing C symbols in Rust: **0**.

- [x] Exact C/Rust dynamic-symbol parity verified after the final release build.
