# Error Surface

The C API does not expose an error enum. Its rejection and fallback sentinels
are `0`, `-1`, and `NULL`. Rows below come from each explicit rejection,
null check, range check, and invalid-mode fallback in `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| E01 | `divide_op` | divisor `b == 0` | `0` | [x] |
| E02 | `modulo_op` | divisor `b == 0` | `0` | [x] |
| E03 | `find_node_by_id` | no table entry has the requested ID (including an empty table) | `NULL` | [x] |
| E04 | `add_tree_node` | `node_count >= MAX_NODES` (`MAX_NODES == 50`) | `-1`; table and count unchanged | [x] |
| E05 | `add_tree_node` | `parent_id != -1` and `find_node_by_id(parent_id)` returns `NULL` (the combined C guard also checks a mismatched returned ID) | `-1`; candidate slot is written, count is unchanged | [x] |
| E06 | `calculate_tree_sum` | `find_node_by_id(node_id)` returns `NULL` (the combined C guard also checks a mismatched returned ID) | `0` | [x] |
| E07 | `parse_operation` | `op_str == NULL` | `OP_ADD` (`1`) | [x] |
| E08 | `get_operation_func` | operation integer is outside `1..=5` | pointer to `add_op` | [x] |
| E09 | `add_tree_node` | `label == NULL`; C has no guard before `strncpy` | abnormal process termination from the unchecked null pointer | [x] |
| E10 | `divide_op` | `a == INT_MIN` and `b == -1`; the C build's signed division overflows | abnormal process termination with `SIGFPE` | [x] |
| E11 | `modulo_op` | `a == INT_MIN` and `b == -1`; the C build's signed division remainder overflows | abnormal process termination with `SIGFPE` | [x] |

There are no asserts and no public length parameters. The only explicit
maximum is `MAX_NODES == 50`.
