# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. Every `return -1`, `return NULL`,
`return 0.0` early-out, saturating clamp, sentinel return, range check, and
implicit null dereference is one row. There are **no** `assert`s and no error
enums in the C source (`grep -n 'assert\|errno\|enum' c_src/src/lib.c` → none).

Constants that define the boundaries: `MAX_NODES 100`, `MAX_NAME_LEN 50`,
`INT_MAX 2147483647`, `INT_MIN -2147483648`.

| #  | function | trigger (exact invalid input / condition) | expected C result | test | ok |
|----|----------|-------------------------------------------|-------------------|------|----|
| 1  | `add_node` | `node_count >= MAX_NODES` (call #101 after 100 successful adds) | returns `-1`, storage/`node_count` unchanged | `err_01_add_node_full` | [x] |
| 2  | `add_node` | boundary: call #100 exactly (`node_count == 99`) — *not* an error | returns `99` (last success) | `err_01_add_node_full` | [x] |
| 3  | `add_node` | `name` longer than `MAX_NAME_LEN-1` = 49 bytes (no NUL in first 49) | `strncpy` truncates to 49 bytes, `name[49] = '\0'`; returns index | `err_03_add_node_name_truncation` | [x] |
| 4  | `add_node` | `name` is the empty string `""` | zero-filled name, returns index; later `process_string` on it → `0` | `err_04_add_node_empty_name` | [x] |
| 5  | `add_node` | `name == NULL` | `strncpy(dst, NULL, 49)` dereferences NULL → SIGSEGV | `err_05_add_node_null_name` (child process, signal compared) | [x] |
| 6  | `find_node_by_id` | `id` matches no stored node (incl. before any `add_node`, i.e. `node_count == 0`) | returns `NULL` | `err_06_find_node_not_found` | [x] |
| 7  | `find_node_by_id` | `id` matches a node whose `active == 0` | returns `NULL` (the `&& active` guard) — unreachable via public API since `add_node` always sets `active = 1`; verified as "no id ever yields an inactive node" | `err_07_find_node_inactive_unreachable` | [x] |
| 8  | `find_node_by_id` | extreme `id` values `INT_MIN`, `INT_MAX`, `0`, `-1` | returns `NULL` (no node uses them as `id`) | `err_06_find_node_not_found` | [x] |
| 9  | `get_children_count` | `parent_id` matches no node | returns `0` (never negative, never an error code) | `err_09_children_count_none` | [x] |
| 10 | `calculate_subtree_sum` | `node_id` not found (`find_node_by_id` → NULL) | returns `0.0` (early return, the sentinel) | `err_10_subtree_sum_missing` | [x] |
| 11 | `calculate_subtree_sum` | empty storage (`node_count == 0`), any `node_id` | returns `0.0` | `err_10_subtree_sum_missing` | [x] |
| 12 | `calculate_subtree_sum` | self-parent node (`id == parent_id`) or a parent cycle | unbounded recursion → stack exhaustion → SIGSEGV | `err_12_subtree_sum_self_cycle` (child process, signal compared) | [x] |
| 13 | `process_string` | `str == NULL` | `*str` dereferences NULL → SIGSEGV | `err_13_process_string_null` (child process, signal compared) | [x] |
| 14 | `process_string` | `str` points at `'\0'` (empty string) — the `if (*str)` guard | returns `0` | `err_14_process_string_empty` | [x] |
| 15 | `process_string` | bytes with the high bit set (`0x80`..`0xFF`) | `char` is signed on x86-64 SysV → each contributes a **negative** value | `err_15_process_string_negative_bytes` | [x] |
| 16 | `process_string` | sum overflows `int` (very long high-byte string) | signed overflow; must match the C-compiled (wrapping) result | `err_16_process_string_overflow` | [x] |
| 17 | `safe_double_to_int` | `d > (double)INT_MAX` (e.g. `2147483648.0`, `1e300`, `+INFINITY`) | clamps, returns `INT_MAX` | `err_17_sdti_over_max` | [x] |
| 18 | `safe_double_to_int` | `d == (double)INT_MAX` exactly (one step *inside* the range) | not clamped → `(int)d == INT_MAX` | `err_17_sdti_over_max` | [x] |
| 19 | `safe_double_to_int` | `d < (double)INT_MIN` (e.g. `-2147483649.0`, `-1e300`, `-INFINITY`) | clamps, returns `INT_MIN` | `err_19_sdti_under_min` | [x] |
| 20 | `safe_double_to_int` | `d == (double)INT_MIN` exactly | not clamped → `(int)d == INT_MIN` | `err_19_sdti_under_min` | [x] |
| 21 | `safe_double_to_int` | `d` is NaN (quiet, signalling, negative-sign NaN) — all three comparisons false, then `d != d` | returns `0` | `err_21_sdti_nan` | [x] |
| 22 | `safe_double_to_int` | subnormals, `-0.0`, values in `(-1, 1)` | truncation toward zero → `0` (and `-0.0` → `0`) | `err_22_sdti_tiny` | [x] |
| 23 | `maxnmin` | `param1 % 6 + 1 <= 0` (any `param1 < 0`, e.g. `-1`, `-5`, `INT_MIN`) | `find_node_by_id` → NULL, whole first block skipped | `err_23_maxnmin_negative_node_id` | [x] |
| 24 | `maxnmin` | `param2 % 6 + 1 <= 0` (any `param2 < 0`) | second node NULL, second block skipped | `err_24_maxnmin_negative_second_id` | [x] |
| 25 | `maxnmin` | `param3 == -1` → divisor `param3 + 1 == 0` | IEEE-754 division by zero → `±inf` or NaN, then clamped by `safe_double_to_int` to `INT_MAX` / `INT_MIN` / `0` | `err_25_maxnmin_div_by_zero` | [x] |
| 26 | `maxnmin` | `param3 == INT_MAX` → `param3 + 1` signed overflow | must match the C-compiled (wrapping) result `INT_MIN` | `err_26_maxnmin_param3_int_max` | [x] |
| 27 | `maxnmin` | `param1 + param2` signed overflow (`INT_MAX + INT_MAX`, `INT_MIN + INT_MIN`) | must match the C-compiled (wrapping) result | `err_27_maxnmin_sum_overflow` | [x] |
| 28 | `maxnmin` | `param4 % 3 + 1 <= 0` (any `param4 < 0`) → `get_children_count(0)` / `(-1)` / `(-2)` | `0` children for `0`/`-2`, `1` child for `-1` (root's `parent_id`) | `err_28_maxnmin_negative_parent` | [x] |
| 29 | `maxnmin` | result accumulation overflows `int` (large `param3` scaling `value * param3`) | clamped per-term by `safe_double_to_int`, then wrapping `+=`; must match C | `err_29_maxnmin_result_overflow` | [x] |
| 30 | *global* | out-of-range "enum" style ints across FFI: every function takes plain `int`, so **any** `int32` is in-range input; exhaustive boundary set `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` on every int parameter | no rejection; must produce identical values | `err_30_int_boundary_cross_product` | [x] |
