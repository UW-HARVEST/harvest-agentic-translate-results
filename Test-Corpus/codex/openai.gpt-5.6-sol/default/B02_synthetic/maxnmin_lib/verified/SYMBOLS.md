# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-RhN4vv.so
nm -D --defined-only target/release/libmaxnmin_lib.so
```

| C symbol | C type | Rust symbol present | Source |
|----------|--------|---------------------|--------|
| `add_node` | `T` | [x] | `src/lib.c:44` |
| `calculate_subtree_sum` | `T` | [x] | `src/lib.c:82` |
| `find_node_by_id` | `T` | [x] | `src/lib.c:63` |
| `get_children_count` | `T` | [x] | `src/lib.c:72` |
| `maxnmin` | `T` | [x] | `src/lib.c:127` |
| `process_string` | `T` | [x] | `src/lib.c:99` |
| `safe_double_to_int` | `T` | [x] | `src/lib.c:112` |

Missing C-defined dynamic symbols in Rust: **0**.

The C library's only non-weak undefined dynamic symbol is the libc function
`strncpy`; it is not part of the library's public API.
