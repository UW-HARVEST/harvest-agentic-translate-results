# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-BiOJI9.so
nm -D --defined-only target/release/libinreftree_lib.so
```

The C library has 13 globally defined dynamic symbols. Undefined libc/toolchain
symbols are not library API and are excluded.

| # | C symbol | kind | Rust symbol | status |
|---|----------|------|-------------|--------|
| 1 | `add_op` | function | `add_op` | [x] |
| 2 | `add_tree_node` | function | `add_tree_node` | [x] |
| 3 | `calculate_tree_sum` | function | `calculate_tree_sum` | [x] |
| 4 | `divide_op` | function | `divide_op` | [x] |
| 5 | `find_node_by_id` | function | `find_node_by_id` | [x] |
| 6 | `get_operation_func` | function | `get_operation_func` | [x] |
| 7 | `inreftree` | function | `inreftree` | [x] |
| 8 | `modulo_op` | function | `modulo_op` | [x] |
| 9 | `multiply_op` | function | `multiply_op` | [x] |
| 10 | `node_count` | object | `node_count` | [x] |
| 11 | `node_table` | object | `node_table` | [x] |
| 12 | `parse_operation` | function | `parse_operation` | [x] |
| 13 | `subtract_op` | function | `subtract_op` | [x] |

Missing C symbols in Rust: **0**.
