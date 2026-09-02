# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. Every `return -1`, `return NULL`,
`return 0` guard, every null check, every range/limit check, and every
`default:` fallback in the source is one row. `lib.c` contains no `assert`, no
`errno` use, and no error enum other than `Operation`'s implicit fallback.

Grep basis (line numbers from `c_src/src/lib.c`):
`64`, `69` (`b == 0` guards), `79` (`return NULL`), `83-84`
(`node_count >= MAX_NODES`), `98-99` (`parent == NULL || parent->id != parent_id`),
`116-117` (`node == NULL || node->id != node_id`), `134` (`op_str == NULL`),
`149` (no-operator fallback), `159` (`default:` fallback), `180`
(`target == NULL || target->value == 0`), plus the two hardware-trapping
signed-division overflows implied by `65` / `70` and the negative-index
`.rodata` read implied by line 189 (`op_string[tree_sum % 4]`).

`MAX_NODES` is the only min/max constant in the file (`50`). The only other
sentinel constant is `-1`, used both as "no parent" / "no child" and as the
`add_tree_node` failure return.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `divide_op` | `b == 0` (line 64), any `a` | returns `0` (does **not** trap) | `err_01_divide_by_zero` | [x] |
| 2 | `modulo_op` | `b == 0` (line 69), any `a` | returns `0` (does **not** trap) | `err_02_modulo_by_zero` | [x] |
| 3 | `divide_op` | `a == INT_MIN && b == -1` — passes the `b==0` guard, then `idiv` overflows | **SIGFPE** (core dump), no value returned | `err_03_divide_intmin_neg1_faults` (subprocess) | [x] |
| 4 | `modulo_op` | `a == INT_MIN && b == -1` — same overflow on the `%` | **SIGFPE** (core dump), no value returned | `err_04_modulo_intmin_neg1_faults` (subprocess) | [x] |
| 5 | `find_node_by_id` | `id` matches no entry in `node_table[0..node_count)` (line 79) | returns `NULL` | `err_05_find_missing_id_null` | [x] |
| 6 | `find_node_by_id` | `node_count == 0` — loop body never runs, so *every* `id` misses | returns `NULL` | `err_06_find_empty_table_null` | [x] |
| 7 | `find_node_by_id` | `node_count < 0` (`node_count` is a writable public object) — `i < node_count` false immediately | returns `NULL` | `err_07_find_negative_count_null` | [x] |
| 8 | `add_tree_node` | `node_count >= MAX_NODES` i.e. `node_count >= 50` (lines 83-84) | returns `-1`, `node_count` unchanged, `node_table` untouched | `err_08_add_node_table_full` | [x] |
| 9 | `add_tree_node` | `parent_id != -1` and `find_node_by_id(parent_id) == NULL` (lines 98-99) | returns `-1`; **but** the slot at `node_table[node_count]` has *already* been fully written (id/value/parent_id/-1/-1/label) and is left behind; `node_count` not incremented | `err_09_add_missing_parent` | [x] |
| 10 | `add_tree_node` | `label == NULL` — reaches `strncpy(node->label, NULL, 31)` (line 92) | **SIGSEGV** (null deref inside `strncpy`) | `err_10_add_null_label_faults` (subprocess) | [x] |
| 11 | `calculate_tree_sum` | `node_id` matches no entry (lines 116-117), incl. `node_count == 0` | returns `0` | `err_11_sum_missing_id_zero` | [x] |
| 12 | `parse_operation` | `op_str == NULL` (line 134) — short-circuits **before** `strchr`, so it is *not* an error at all | returns `OP_ADD` (`1`) | `err_12_parse_null_is_add` | [x] |
| 13 | `parse_operation` | `op_str` contains none of `+ * - / %` (line 149), incl. the empty string `""` | returns `OP_ADD` (`1`) | `err_13_parse_no_operator_is_add` | [x] |
| 14 | `get_operation_func` | `op` is an out-of-range enum value: `0`, `6`, `-1`, `INT_MIN`, `INT_MAX`, or any `int` outside `1..=5` (line 159 `default:`) | returns `add_op` — accepted, never rejected | `err_14_get_op_func_out_of_range_enum` | [x] |
| 15 | `inreftree` | `param2 == 0`, so the selected target node (`id 2`, label `"left"`) has `value == 0` (line 180) | `target_id` is reset from `2` to `1`, changing the second operand of the final operation | `err_15_inreftree_param2_zero_retargets` | [x] |
| 16 | `inreftree` | `target == NULL` branch of line 180 — requires `target_id == -1`, i.e. no label in `node_table[0..node_count)` containing `'l'` | **unreachable**: node 2's label is always `"left"`, and `node_count >= 2` always holds because node 1 has `parent_id == -1` and node 2's parent exists | `err_16_inreftree_target_null_unreachable` (proves reachability of the sibling branch instead) | [x] |
| 17 | `inreftree` | `tree_sum % 4 < 0` (line 189) — C's `%` truncates toward zero, so `op_string[negative]` reads the `.rodata` bytes **preceding** the `"+*-%"` literal | reads `'f'` / `'t'` / `'\0'` for `-3` / `-2` / `-1`; `parse_operation` then yields `OP_ADD`, so the op is `add_op` | `err_17_inreftree_negative_modulo_oob_read` | [x] |
| 18 | `add_tree_node` | `parent->id != parent_id` sub-condition of line 98 | **unreachable dead check**: `find_node_by_id` only returns a node whose `id == parent_id` | `err_18_dead_parent_id_recheck` | [x] |
| 19 | `calculate_tree_sum` | `node->id != node_id` sub-condition of line 116 | **unreachable dead check**, same reason as row 18 | `err_19_dead_node_id_recheck` | [x] |
| 20 | generic FFI boundary | `parse_operation` with a non-NUL-terminated / unterminated buffer, and `add_tree_node` with a `label` longer than 31 bytes (silent truncation, not an error) | truncates to 31 bytes + forced `label[31] = '\0'`; no error returned | `err_20_oversized_label_truncates` | [x] |
| 21 | generic FFI boundary | `node_count == MAX_NODES` exactly (`50`) — the boundary one step past the last writable slot index `49` | `add_tree_node` returns `-1` (row 8); `add_tree_node` at `node_count == 49` still succeeds and returns `49` | `err_21_add_node_boundary_49_50` | [x] |

## Rows deliberately NOT tested by comparing return values

Rows 3, 4 and 10 are hardware faults in the C library. They are verified in a
**forked subprocess** (`fork()` + `waitpid()`) so the differential harness
survives: the test asserts that the C `.so` and the Rust `.so` terminate with
the *same signal*, rather than comparing a return value that neither produces.

Row 34 of `CONFIGS.md` (reading `node_table[50]` via `node_count > 50`) is
excluded from the differential asserts: it reads past the end of the array in
both languages, and the two libraries place `node_count` on opposite sides of
`node_table`, so the bytes read are unrelated. It is out of the defined range of
every C function, and the C code itself never produces `node_count > 50`.

`add_tree_node` is likewise never called with a **negative** `node_count`: the C
would evaluate `&node_table[node_count]`, writing *before* the array into
whatever the linker placed there — different memory in each library, so there is
nothing meaningful to compare, and the write could take out the test process.
`err_07` covers the negative-`node_count` reads (`find_node_by_id`,
`calculate_tree_sum`), which are well defined and do return `NULL` / `0`.

## Divergences found and fixed

Row 3/4 was a **real** divergence in the translation. The Rust originally used
`wrapping_div` / `wrapping_rem`, which return `INT_MIN` / `0` for
`INT_MIN / -1`, whereas the C's `idiv` faults with SIGFPE. `divide_op` and
`modulo_op` now issue the division through an `idiv` instruction so the fault is
reproduced. Removing that (mutation `divide_op wraps (loses SIGFPE)` in
`mutate.sh`) is caught by `err_03` / `err_04`.

Row 10 exposed a second, profile-dependent divergence: Rust's *debug-only* UB
checks turned the NULL-`label` dereference into a Rust panic (SIGABRT) while the
C SIGSEGV'd, so the debug `.so` behaved differently from the release one.
`[profile.dev]` now sets `debug-assertions = false` and `overflow-checks = false`
so both profiles reproduce the C behavior; the suite passes against both `.so`s.
