# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/debug/libmaxnmin_lib.so
```

The C build has seven defined public dynamic symbols. The weak runtime symbols
and the undefined `strncpy@GLIBC_2.2.5` import in the unfiltered `nm -D` output
are not library exports.

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `add_node` | `T` | `add_node` | present |
| `calculate_subtree_sum` | `T` | `calculate_subtree_sum` | present |
| `find_node_by_id` | `T` | `find_node_by_id` | present |
| `get_children_count` | `T` | `get_children_count` | present |
| `maxnmin` | `T` | `maxnmin` | present |
| `process_string` | `T` | `process_string` | present |
| `safe_double_to_int` | `T` | `safe_double_to_int` | present |

Missing C exports in Rust: **0**

