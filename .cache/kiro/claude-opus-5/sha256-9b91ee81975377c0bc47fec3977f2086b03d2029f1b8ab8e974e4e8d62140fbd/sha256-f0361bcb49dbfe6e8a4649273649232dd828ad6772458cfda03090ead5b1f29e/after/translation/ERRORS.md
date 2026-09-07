# ERRORS.md — Phase A error-surface table

Derived mechanically by grepping `c_src/src/lib.c` for every rejection path:
`return -1`, `return NULL`, `return 0` guard clauses, fallback `default:` /
trailing `return`, every explicit comparison guard, every null check, and the
one min/max constant (`MAX_NODES`). There are **no** `assert`s and no
`RETURN_ERROR`-style macros in this library — its whole error surface is guard
clauses and sentinel returns.

Constants: `MAX_NODES = 50`. Sentinels: `-1` (no parent / no child / "not
found" id in `inreftree`), `NULL`, `0`.

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `divide_op` | `b == 0` (`if (b == 0) return 0;`) | returns `0`, no trap | `err_row_01_divide_by_zero` |
| 2 | `modulo_op` | `b == 0` (`if (b == 0) return 0;`) | returns `0`, no trap | `err_row_02_modulo_by_zero` |
| 3 | `find_node_by_id` | no entry in `node_table[0..node_count]` has `id` | returns `NULL` | `err_row_03_find_node_absent` |
| 4 | `find_node_by_id` | `node_count == 0` (loop body never runs, table may hold stale matching rows) | returns `NULL` | `err_row_04_find_node_empty_table` |
| 5 | `find_node_by_id` | `node_count < 0` (`i < node_count` false immediately) | returns `NULL` | `err_row_05_find_node_negative_count` |
| 6 | `add_tree_node` | `node_count >= MAX_NODES` (i.e. `>= 50`) | returns `-1`, nothing written, `node_count` unchanged | `err_row_06_add_node_table_full` |
| 7 | `add_tree_node` | `node_count > MAX_NODES` (e.g. 60, 1000 — same `>=` guard) | returns `-1`, `node_count` unchanged | `err_row_07_add_node_count_over_max` |
| 8 | `add_tree_node` | `parent_id != -1` and `find_node_by_id(parent_id) == NULL` | returns `-1` **after** the row at `node_table[node_count]` has been fully written; `node_count` NOT incremented, so the half-linked row stays behind | `err_row_08_add_node_missing_parent` |
| 9 | `add_tree_node` | `parent_id != -1`, parent found, but parent already has BOTH `left_child_id != -1` and `right_child_id != -1` | returns `node_count-1` (SUCCESS); child is appended but never linked to the parent — silently dropped from the tree | `err_row_09_add_node_parent_full` |
| 10 | `add_tree_node` | `label` longer than 31 bytes | `strncpy(...,31)` truncates, `label[31]='\0'`; no error | `err_row_10_label_truncation` |
| 11 | `add_tree_node` | `label` exactly 31 bytes (boundary: no NUL copied by `strncpy`, then `label[31]=0`) | full 31 bytes + explicit NUL; no error | `err_row_11_label_exactly_31` |
| 12 | `add_tree_node` | `label` empty (`""`) | `strncpy` zero-pads all 31 bytes; no error | `err_row_12_label_empty` |
| 13 | `calculate_tree_sum` | `find_node_by_id(node_id)` returns `NULL` (unknown id) | returns `0` | `err_row_13_tree_sum_unknown_id` |
| 14 | `calculate_tree_sum` | `node_count == 0` | returns `0` for every id | `err_row_14_tree_sum_empty_table` |
| 15 | `calculate_tree_sum` | node's `left_child_id` / `right_child_id` names a **nonexistent** id (dangling link, `!= -1`) | that branch contributes `0`; parent value still returned | `err_row_15_tree_sum_dangling_child` |
| 16 | `parse_operation` | `op_str == NULL` — folded into the first `if`, so NULL is NOT dereferenced | returns `OP_ADD` (`1`) | `err_row_16_parse_null` |
| 17 | `parse_operation` | string containing none of `+ * - / %` (incl. `""`) | falls through to trailing `return OP_ADD` (`1`) | `err_row_17_parse_no_operator` |
| 18 | `get_operation_func` | `op` outside `1..=5` (`0`, `6`, `-1`, `INT_MIN`, `INT_MAX`, any int — C enums accept any `int` across FFI) | `default:` → `add_op`; returned pointer behaves as addition | `err_row_18_get_op_func_out_of_range` |
| 19 | `inreftree` | no label in `node_table[0..node_count]` contains `'l'` ⇒ `target_id` stays `-1` ⇒ `find_node_by_id(-1) == NULL` | `target_id` reset to `1` | (unreachable for the fixed tree; covered by `err_row_19_inreftree_target_reset` via `param2 == 0`) |
| 20 | `inreftree` | `target->value == 0` (i.e. `param2 == 0`, since the `'l'` scan always settles on node id 2 `"left"`) | `target_id` reset to `1`, changing the second operand of the final op | `err_row_19_inreftree_target_reset` |
| 21 | `inreftree` | `tree_sum % 4 < 0` (negative sum) ⇒ negative index into `"+*-%"` | reads `.rodata` bytes before the literal (`'f'`, `'t'`, `'\0'`); none is an operator, so `parse_operation` → `OP_ADD` | `err_row_21_negative_modulus_index` |

## Excluded: C-side undefined behaviour that traps or corrupts memory

These are real inputs the C accepts, but the C does not *return* — it faults or
scribbles outside its objects. There is no value to compare against, so they are
documented rather than differentially asserted. Randomized Phase B inputs avoid
them deliberately.

| # | function | trigger | C behaviour (measured) | Rust behaviour | disposition |
|---|----------|---------|------------------------|----------------|-------------|
| E1 | `divide_op` | `a == INT_MIN && b == -1` | **SIGFPE** (`idiv` `#DE`); confirmed, process dies with signal 8 / exit 136 | `wrapping_div` → `INT_MIN` | excluded — a crash is not a comparable result; `divide_op` is never reachable from `inreftree` (`op_string` is `"+*-%"`, contains no `/`) |
| E2 | `modulo_op` | `a == INT_MIN && b == -1` | **SIGFPE**; confirmed, exit 136 | `wrapping_rem` → `0` | excluded, same reasoning; `inreftree` only reaches `modulo_op` with `b ∈ {1,2}` |
| E3 | `add_tree_node` | `node_count < 0` | writes `node_table[node_count]` **before** the array — out-of-bounds store into `.bss` | same out-of-bounds store (`ptr::add` with wrapped index) | excluded — memory corruption in both; the read-only negative-count paths are covered by rows 5 and 14 |
| E4 | `calculate_tree_sum` | a cycle in `left_child_id`/`right_child_id` (e.g. node whose child id is itself) | unbounded recursion → **stack overflow / SIGSEGV** | same unbounded recursion → stack overflow | excluded — both diverge without returning |
| E5 | `add_op` / `multiply_op` / `subtract_op` | signed overflow | wraps (measured on x86-64 gcc) | `wrapping_*` — matches | **not** excluded; asserted in Phase B rows 1–3 |
