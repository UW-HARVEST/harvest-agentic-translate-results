# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. Every rejection / early-return /
sentinel / bound in the C source gets one row. There are no `assert`s and no
`RETURN_ERROR`-style macros in this TU; the rejection vocabulary is
`return -1`, `return 0`, `return NULL`, and `default:` in a `switch`.

Greps used:

```
grep -n 'return -1\|return 0;\|return NULL\|default:\|== 0\|>= MAX_NODES\|!= -1\|== -1\|== NULL' c_src/src/lib.c
```

| # | function | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|------------------------------------------|-------------------|------|---|
| 1 | `add_tree_node` | `node_count >= MAX_NODES` (50) — table full | returns `-1`; `node_table`/`node_count` untouched | `err01_add_tree_node_table_full` | [x] |
| 2 | `add_tree_node` | `parent_id != -1` and `find_node_by_id(parent_id) == NULL` (unknown parent) | returns `-1`, **but `node_table[node_count]` has already been overwritten** and `node_count` is *not* incremented | `err02_add_tree_node_unknown_parent` | [x] |
| 3 | `add_tree_node` | `parent_id != -1`, parent found, but `parent->id != parent_id` | dead branch (`find_node_by_id` guarantees equality) — must still return the same as row 2's success path, i.e. never taken | `err03_add_tree_node_parent_id_branch_dead` | [x] |
| 4 | `add_tree_node` | `parent_id == -1` (root sentinel) | no parent lookup at all; succeeds even though no node with id `-1` exists | `err04_add_tree_node_root_sentinel` | [x] |
| 5 | `add_tree_node` | parent already has both `left_child_id` and `right_child_id` set (3rd child) | node still added, `node_count` still incremented, but child is **silently unlinked** from the parent | `err05_add_tree_node_third_child_dropped` | [x] |
| 6 | `add_tree_node` | `label` longer than 31 bytes | `strncpy(...,31)` truncates, `label[31]='\0'`; **no NUL from source is copied** | `err06_add_tree_node_label_truncation` | [x] |
| 7 | `add_tree_node` | `label` exactly 31 bytes / empty string `""` | boundary of row 6: 31 bytes + forced NUL; `""` NUL-pads all 32 bytes | `err07_add_tree_node_label_boundaries` | [x] |
| 8 | `add_tree_node` | `label == NULL` | `strncpy(dst, NULL, 31)` dereferences NULL → **SIGSEGV (signal 11)** | `err08_null_label_segv_subprocess` (subprocess) | [x] |
| 9 | `find_node_by_id` | `id` not present in `node_table[0..node_count]` | returns `NULL` | `err09_find_node_missing_id` | [x] |
|10 | `find_node_by_id` | `node_count == 0` (empty table) | loop body never runs → returns `NULL` | `err10_find_node_empty_table` | [x] |
|11 | `find_node_by_id` | `node_count < 0` (corrupted counter, reachable via exported `node_count`) | `i < node_count` false immediately → returns `NULL` | `err11_find_node_negative_count` | [x] |
|12 | `find_node_by_id` | duplicate ids in table | returns the **first** match (lowest index), not the last | `err12_find_node_duplicate_ids` | [x] |
|13 | `find_node_by_id` | `id == -1` / `id == INT_MIN` / `id == INT_MAX` | no special-casing; `NULL` unless such an id was really stored | `err13_find_node_extreme_ids` | [x] |
|14 | `calculate_tree_sum` | `find_node_by_id(node_id) == NULL` (unknown id) | returns `0` (indistinguishable from a real node whose subtree sums to 0) | `err14_tree_sum_unknown_id` | [x] |
|15 | `calculate_tree_sum` | node found but `node->id != node_id` | dead branch — never taken | `err15_tree_sum_id_branch_dead` | [x] |
|16 | `calculate_tree_sum` | child id `== -1` | sentinel: that side is **not** recursed into | `err16_tree_sum_sentinel_children` | [x] |
|17 | `calculate_tree_sum` | child id `!= -1` but no such node exists (dangling child) | recursion returns `0`, no error propagated | `err17_tree_sum_dangling_child` | [x] |
|18 | `calculate_tree_sum` | cycle (`node->left_child_id` points at an ancestor/itself) | unbounded recursion → **stack overflow, SIGSEGV (signal 11)** | `err18_tree_sum_cycle_segv_subprocess` (subprocess) | [x] |
|19 | `calculate_tree_sum` | subtree values overflow `int` | signed overflow; `-O0` C wraps two's-complement | `err19_tree_sum_overflow_wraps` | [x] |
|20 | `divide_op` | `b == 0` | explicit guard → returns `0` (no trap) | `err20_divide_by_zero` | [x] |
|21 | `divide_op` | `a == INT_MIN && b == -1` | **not guarded** → `idiv` overflow → **SIGFPE (signal 8)** | `err21_intmin_div_sigfpe_subprocess` (subprocess) | [x] |
|22 | `modulo_op` | `b == 0` | explicit guard → returns `0` (no trap) | `err22_modulo_by_zero` | [x] |
|23 | `modulo_op` | `a == INT_MIN && b == -1` | **not guarded** → `idiv` overflow → **SIGFPE (signal 8)** | `err23_intmin_rem_sigfpe_subprocess` (subprocess) | [x] |
|24 | `modulo_op` | negative operands (`-7 % 2`, `7 % -2`) | C truncation-toward-zero remainder: `-1`, `1` | `err24_modulo_sign_semantics` | [x] |
|25 | `subtract_op`/`add_op`/`multiply_op` | operands overflowing `int` | signed overflow; `-O0` C wraps | `err25_arith_overflow_wraps` | [x] |
|26 | `parse_operation` | `op_str == NULL` | short-circuits **before** `strchr` → returns `OP_ADD` (1), *not* an error | `err26_parse_operation_null` | [x] |
|27 | `parse_operation` | string containing none of `+ * - / %` | falls through to `return OP_ADD` (1) | `err27_parse_operation_no_operator` | [x] |
|28 | `parse_operation` | empty string `""` | no match → `OP_ADD` (1) | `err28_parse_operation_empty` | [x] |
|29 | `parse_operation` | string containing several operators, e.g. `"%/-*+"` | priority order is fixed `+ * - / %`, **not** left-to-right | `err29_parse_operation_priority` | [x] |
|30 | `get_operation_func` | `op` outside `1..=5` — `0`, `6`, `-1`, `INT_MIN`, `INT_MAX` (C enums accept any `int` across FFI) | `default:` → returns `add_op` | `err30_get_operation_func_out_of_range_enum` | [x] |
|31 | `get_operation_func` | every valid `op` `1..=5` | returns that library's own `add/multiply/subtract/divide/modulo_op` symbol | `err31_get_operation_func_valid_enums` | [x] |
|32 | `inreftree` | `param2 == 0` → `target->value == 0` | `target_id` reset from `2` to `1` | `err32_inreftree_target_reset` | [x] |
|33 | `inreftree` | `target == NULL` | dead branch (`"left"` always matches `'l'`, so `target_id==2` always resolves) | `err32_inreftree_target_reset` | [x] |
|34 | `inreftree` | `tree_sum % 4 < 0` (i.e. `p1+p2+p3+p4 < 0` and not a multiple of 4) | **out-of-bounds read** `op_string[-1..-3]`; in the C `.so`'s `.rodata` those bytes are `'\0'`, `'t'`, `'f'` (tail of `"left-left"`), all of which `parse_operation` maps to `OP_ADD` | `err34_inreftree_negative_modulo_oob_read` | [x] |
|35 | `inreftree` | `node_count`/`node_table` pre-corrupted by the caller | `inreftree` resets `node_count = 0` first, so stale rows are ignored | `err35_inreftree_ignores_stale_state` | [x] |

## Notes on the four signal rows (8, 18, 21, 23)

Rows 8, 18, 21 and 23 abort the process, so they cannot be asserted in-process.
They are tested by `tests/harness/crash_host.rs`, which compiles a tiny **C
host** that `dlopen`s whichever `.so` it is given, runs it once per library as a
child process, and compares exit code / terminating signal / stdout. A C host is
used (rather than re-execing the Rust test binary) because the Rust runtime
installs a guard-page handler that would turn the row-18 `SIGSEGV` into an
abort — a harness artefact, not library behaviour. Measured, identical for both:

| row | case | C | Rust |
|-----|------|---|------|
| 8   | `null_label` | signal 11 (SIGSEGV) | signal 11 |
| 18  | `cycle`      | signal 11 (SIGSEGV) | signal 11 |
| 21  | `div_intmin` | signal 8 (SIGFPE)   | signal 8  |
| 23  | `rem_intmin` | signal 8 (SIGFPE)   | signal 8  |
| —   | `ok` (self-test) | exit 0, same stdout | exit 0 |

* Rows 21/23 required a **fix**: the original Rust used
  `wrapping_div`/`wrapping_rem`, which return `INT_MIN` / `0` where the C traps
  with `SIGFPE`. The Rust now performs a real `idiv` (inline asm on `x86_64`,
  which is the C `.so`'s target) so both libraries trap identically.

## Appendix — rejections that are UNREACHABLE through the public API

Two more out-of-bounds conditions exist in the C source but cannot be triggered
by any sequence of calls to the exported functions; they need a caller to write
a bogus value into the exported `node_count` object directly. They are recorded
here for completeness and deliberately NOT given differential tests, because in
both languages the result is an out-of-bounds access whose observable effect
depends on unrelated `.bss` layout:

* **`add_tree_node` with `node_count < 0`** — `&node_table[node_count]` is before
  the array, so the write lands on whatever precedes it. `node_count` is only
  ever set to `0` (`inreftree`) or incremented, and the `>= MAX_NODES` guard
  caps it at 50, so it can never legitimately go negative.
  *(Read-only use of a negative `node_count` IS tested — row 11 — because the
  loop condition `i < node_count` then rejects immediately, which is defined.)*
* **`find_node_by_id` / `calculate_tree_sum` with `node_count > 50`** — the loop
  reads `node_table[50..node_count)`. In the C `.so` `node_count` sits
  immediately *after* `node_table` (`0x4060 + 2600 == 0x4a88`), so the OOB read
  aliases the counter itself; in the Rust `.so` `node_count` sits *before*
  `node_table`. Neither layout is guaranteed by either language and the
  condition is unreachable, since `add_tree_node` refuses at 50.
  *(`node_count == 50` — the largest reachable value — reads only in-bounds
  indices 0..49 and IS tested, rows 1 and 12 of `CONFIGS.md`.)*

Everything else the C can be made to do from outside is covered by a row above.
