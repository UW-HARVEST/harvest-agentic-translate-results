# CONFIGS.md — Phase A configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the axes
`c_src/src/lib.c` actually branches on.

## Axes the C distinguishes

**A1 — runtime options/modes.** The library has no flags, no `#ifdef`s, and no
setters. Its only mutable configuration is the two exported data objects:
`node_count` (int) and `node_table` (`TreeNode[50]`). These *are* the option
surface: every lower-level entry point reads them, so tests set them directly
through the `.so`'s data symbols in both libraries.

**A2 — the operation mode.** `Operation` ∈ {`OP_ADD`=1, `OP_MULTIPLY`=2,
`OP_SUBTRACT`=3, `OP_DIVIDE`=4, `OP_MODULO`=5}, selected two ways:
`parse_operation` (first matching char, checked in the order `+ * - / %`) and
`get_operation_func` (`switch` on `(int)op` with a `default`). Note `inreftree`'s
`op_string` is `"+*-%"` — it can never select `OP_DIVIDE`.

**A3 — input shapes.** Operand values (zero / positive / negative / `INT_MIN` /
`INT_MAX` / overflow-producing pairs); table population (empty / one / many / full
= 50); node position within the scan (first / middle / last / absent); duplicate
ids; parent link state (no children / left only / both); child links (leaf / left
only / right only / both / chain depth 3 / dangling); label length (0 / short /
30 / 31 / >31) and label content (contains `'l'` or not); string content for
`parse_operation` (each operator, operator not first, several operators,
non-operator, empty); `node_count` relative to `MAX_NODES` (0, 1, 49, 50).

**A4 — entry points.** All 11 exported functions, low-level first. `inreftree` is
the only convenience/one-shot wrapper; the other 10 are driven directly.

**Feature combinations.** `translation/Cargo.toml` declares no `[features]`
table, so the only build configuration is the default. `enumerate_features.sh`
re-derives this mechanically. There is no `[[bin]]` target and no binary in
`CMakeLists.txt` (`add_library(... SHARED ...)` only), so no stdout comparison
applies.

## Configuration rows

Every row is exercised with many randomized inputs (fixed seed, xorshift PRNG in
`tests/common/mod.rs`), calling both `.so`s through `libloading` and comparing
byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `add_op` | randomized `(a,b)` over full `i32` range incl. overflow pairs, `INT_MIN`/`INT_MAX`, zeros; `unused1/2` randomized to prove they are ignored | [x] |
| 2 | `multiply_op` | same shape; includes products that overflow `i32` | [x] |
| 3 | `subtract_op` | same shape; includes `INT_MIN - positive` underflow | [x] |
| 4 | `divide_op` | `b != 0`, randomized signs (4 sign quadrants) → C truncation toward zero; excludes `INT_MIN/-1` (ERRORS E1) | [x] |
| 5 | `modulo_op` | `b != 0`, randomized signs → C remainder keeps dividend's sign; excludes `INT_MIN%-1` (ERRORS E2) | [x] |
| 6 | `get_operation_func` + returned pointer | `op` = each of 1..5, then the returned `OperationFunc` invoked on randomized operands (`b != 0`) — identifies the function by behaviour, since raw pointers differ per `.so` | [x] |
| 7 | `parse_operation` | strings containing exactly one operator, for each of `+ * - / %`, at randomized positions with randomized filler | [x] |
| 8 | `parse_operation` | strings containing SEVERAL operators in randomized order — exercises the fixed check precedence (`+` wins over `*` wins over `-` …) regardless of textual position | [x] |
| 9 | `parse_operation` | 1-byte strings for all 256 byte values, plus randomized non-operator strings and `""` | [x] |
| 10 | `find_node_by_id` | `node_count` = 1 / 2 / 25 / 50 with the target at the first, a middle, and the last slot; ids randomized (incl. negative and `INT_MIN`/`INT_MAX`) | [x] |
| 11 | `find_node_by_id` | duplicate ids present → first match wins; plus rows beyond `node_count` that would match (must be ignored) | [x] |
| 12 | `add_tree_node` | `parent_id == -1` (root) into an empty table; label shapes 0/short/30/31/>31 bytes; verifies return value, all 5 int fields, all 32 label bytes, and `node_count` | [x] |
| 13 | `add_tree_node` | `parent_id` names an existing node with **no** children → links `left_child_id`; then a second child → links `right_child_id`; then a third → parent full, appended unlinked | [x] |
| 14 | `add_tree_node` | fill the table to exactly `MAX_NODES` (50) one call at a time, randomized ids/values/labels, chained parents — boundary 49 → 50 | [x] |
| 15 | `calculate_tree_sum` | hand-built tables covering leaf / left-only / right-only / both-children / depth-3 chain / wide 50-node tree, randomized values incl. overflow-producing sums | [x] |
| 16 | `inreftree` | randomized `(p1,p2,p3,p4)` covering all four `tree_sum % 4` non-negative residues **and** all three negative residues (the out-of-bounds `op_string` index), plus `INT_MIN`/`INT_MAX` and overflowing sums | [x] |
| 17 | `inreftree` | `param2 == 0` → target reset path (`target_id` 2 → 1), crossed with each `tree_sum % 4` residue | [x] |
| 18 | `inreftree` | repeated/interleaved calls — proves the reset of `node_count` to 0 and the stale `node_table` bytes left by the previous call produce identical results in both libraries | [x] |
| 19 | `inreftree` then low-level calls | after `inreftree` returns, read `node_count`/`node_table` and call `find_node_by_id` / `calculate_tree_sum` / `add_tree_node` on the leftover state — the composed pipeline, invisible to per-function tests | [x] |
| 20 | `node_table` / `node_count` data symbols | full 2600-byte `node_table` image plus `node_count` compared byte-for-byte between the two `.so`s after every mutating sequence above | [x] |
| 21 | all 11 functions | long randomized **mixed command sequence** (add/find/sum/parse/dispatch/inreftree/table-poke) applied in lockstep to both libraries, comparing every return value and the whole table image after each step | [x] |
| 22 | `add_tree_node` (+ `node_table` poke) | append over a **dirty** slot whose `label[31]` is already non-zero — the only configuration in which the explicit `node->label[31] = '\0'` store is observable, since `strncpy(...,31)` never writes that byte. Refines row 12; added after mutation testing showed row 12 alone could not see it. | [x] |

## Row → test mapping

| rows | test |
|------|------|
| 1–11, 12, 13–21 | `tests/phase_b_valid.rs::configs_row_NN_*` (one `#[test]` per row) |
| 22 | `tests/phase_b_valid.rs::configs_row_12b_label31_forced_nul_over_dirty_slot` |

All 22 tests pass under both build configurations enumerated by
`enumerate_features.sh` (`DEFAULT` and `--no-default-features`).

## Harness validation (why these rows are trusted)

A differential suite that never fails proves nothing, so the harness was
mutation-tested: six deliberate bugs were injected into `src/lib.rs` one at a
time and the suite re-run.

| mutant | outcome |
|--------|---------|
| `OP_STRING_OFFSET` 26 → 27 | **killed** (5 tests failed) |
| `parse_operation` check order `+` ↔ `*` | **killed** (1 test) |
| `add_tree_node` early-returns before writing the row (removes the C quirk) | **killed** (2 tests) |
| `node_count >= MAX_NODES` → `>` | **killed** (3 tests) |
| drop `label[31] = 0` | **killed** by row 22 (survived before row 22 was added) |
| `strncpy(...,31)` → `strncpy(...,32)` | survived — provably **equivalent**: with `n=32`, `strncpy` writes indices 0..31 and the following `label[31]=0` overwrites index 31, so bytes 0..30 and byte 31 are identical either way |
