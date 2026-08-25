# Dynamic Symbol Surface

Generated from:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| # | C symbol | Kind | Rust export |
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
| 12 | `node_table` | object | present |
| 13 | `node_count` | object | present |

Missing from the Rust shared library: **0**

The C shared library's only undefined dynamic symbols are the libc functions
`strchr` and `strncpy` plus weak compiler/runtime hooks.
