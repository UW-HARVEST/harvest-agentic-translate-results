# Dynamic symbol surface

Source command:

```text
nm -D --defined-only ../c_src/build/libharvest-work-uOvjoU.so
```

Only globally defined dynamic symbols (`T`) are listed. Toolchain-local
initialization symbols are not present in this `nm -D --defined-only` output.

| # | C symbol | Rust symbol | Status |
|---|----------|-------------|--------|
| 1 | `add_node` | `add_node` | present |
| 2 | `find_node_by_id` | `find_node_by_id` | present |
| 3 | `get_children_count` | `get_children_count` | present |
| 4 | `calculate_subtree_sum` | `calculate_subtree_sum` | present |
| 5 | `process_string` | `process_string` | present |
| 6 | `safe_double_to_int` | `safe_double_to_int` | present |
| 7 | `maxnmin` | `maxnmin` | present |

Missing C symbols in Rust: **0**.

Completion gate: [x] exact C symbol-name parity verified after release build.
