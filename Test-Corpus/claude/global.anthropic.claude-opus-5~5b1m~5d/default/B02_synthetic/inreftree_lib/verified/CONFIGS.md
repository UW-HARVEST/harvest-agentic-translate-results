# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C actually branches on

There is no init/option struct in this library; the "configuration" is the
**mutable global state** (`node_table`, `node_count` — both exported) plus the
**shape of the tree** built through `add_tree_node`, plus the **operation
selector**. Enumerated from the source:

* **A. Entry point.** The header only declares `inreftree`, but the `.so`
  exports 11 functions and 2 objects (see `SYMBOLS.md`). The low-level entry
  points (`add_tree_node`, `find_node_by_id`, `calculate_tree_sum`,
  `parse_operation`, `get_operation_func`, the five `*_op`) must be driven
  directly, not only through the `inreftree` one-shot wrapper.
* **B. Global-state configuration** (`node_count` ∈ {0, 1, mid, 49, 50, >50, <0},
  `node_table` pristine vs. pre-seeded vs. stale-past-`node_count`).
* **C. Tree shape**: empty / single root / root+1 child / root+2 children /
  3-deep chain / node with 3+ children (3rd silently dropped) / duplicate ids /
  dangling child ids / `-1` child sentinels / full 50-node table.
* **D. `parent_id`**: `-1` (root sentinel) vs. existing id vs. missing id.
* **E. Parent child-slot state**: `left_child_id == -1` (fill left) vs. left
  taken & `right_child_id == -1` (fill right) vs. both taken (drop).
* **F. `label` shape**: short / exactly 31 bytes / 32+ bytes (truncated) /
  empty / embedded non-ASCII / high-bit bytes.
* **G. `Operation` selector** passed to `get_operation_func`: each of `1..=5`
  and out-of-range (`default:`).
* **H. `parse_operation` input shape**: each of `+ * - / %` alone, several
  together (fixed priority `+ * - / %`), none, empty, NULL.
* **I. Operand value shape** for the five `*_op`: zero / positive / negative /
  mixed signs / `INT_MIN` / `INT_MAX` / divisor `0` / divisor `-1`.
* **J. `inreftree` param shape**, which selects the branches downstream:
  `param2 == 0` (target reset 2→1) vs. `!= 0`; and
  `tree_sum % 4` ∈ {0,1,2,3} (→ `+ * - %`) vs. {-1,-2,-3} (→ OOB `.rodata`
  read). Note `'/'` is absent from `"+*-%"`, so `OP_DIVIDE` is unreachable
  *via `inreftree`* and only reachable through the low-level entry points.

`Cargo.toml` has no `[features]` table → exactly one feature configuration.

## Rows (each = one meaningful combination; all randomized, seed-fixed)

| # | entry point(s) | configuration (options set + input shape) | test (`tests/phase_b_configs.rs`) | ✔ |
|---|----------------|-------------------------------------------|------|---|
| 1 | `add_op` | I: 0/±/mixed/`INT_MIN`/`INT_MAX` + 20k random `(a,b)`; overflow wraps | `cfg01_add_op` | [x] |
| 2 | `multiply_op` | I: same, incl. overflowing products | `cfg02_multiply_op` | [x] |
| 3 | `subtract_op` | I: same, incl. `INT_MIN - 1` underflow | `cfg03_subtract_op` | [x] |
| 4 | `divide_op` | I: `b != 0`, both signs, `a=INT_MIN,b∈{1,2,-2}`, truncation toward zero | `cfg04_divide_op` | [x] |
| 5 | `modulo_op` | I: `b != 0`, both signs, C truncating-remainder sign rule | `cfg05_modulo_op` | [x] |
| 6 | `find_node_by_id` + `node_table`/`node_count` writes | B/C: table seeded directly through the exported `node_table` symbol, `node_count` ∈ {0,1,7,49,50}; probe every id incl. absent ones. Compares the returned pointer **as an index into each library's own `node_table`** | `cfg06_find_node_by_id_seeded_state` | [x] |
| 7 | `add_tree_node` | D=`-1`, F=short label, from empty table → creates root, returns index 0 | `cfg07_add_root_sentinel_parent` | [x] |
| 8 | `add_tree_node` | D=existing parent, E=left free → links `left_child_id` | `cfg08_add_links_left_child` | [x] |
| 9 | `add_tree_node` | D=existing parent, E=left taken/right free → links `right_child_id` | `cfg09_add_links_right_child` | [x] |
|10 | `add_tree_node` | D=existing parent, E=both taken → node added, link dropped | `cfg10_add_third_child_link_dropped` | [x] |
|11 | `add_tree_node` | F: label 0/1/30/31/32/64 bytes + random high-bit bytes → full 32-byte `label` compared | `cfg11_add_label_shapes` | [x] |
|12 | `add_tree_node` | B: fill to `node_count == 49`, then 50th succeeds, 51st rejected | `cfg12_add_fills_table_to_capacity` | [x] |
|13 | `add_tree_node` | random sequence of 200 `(id, value, parent_id, label)` ops incl. duplicate & missing parents; after every call compare full 2600-byte `node_table` + `node_count` + return value | `cfg13_add_tree_node_random_op_sequences` | [x] |
|14 | `calculate_tree_sum` | C=single root, no children (`-1` sentinels) | `cfg14_sum_single_root` | [x] |
|15 | `calculate_tree_sum` | C=root + left only / right only / both | `cfg15_sum_root_plus_children` | [x] |
|16 | `calculate_tree_sum` | C=3-deep left chain, and unbalanced deep chain (49 nodes) | `cfg16_sum_deep_chains` | [x] |
|17 | `calculate_tree_sum` | C=dangling child ids + duplicate ids + values chosen to overflow | `cfg17_sum_dangling_dup_overflow` | [x] |
|18 | `calculate_tree_sum` | called on a **non-root** id (subtree sum), and on every id in a random tree | `cfg18_sum_every_id_of_random_tree` | [x] |
|19 | `parse_operation` | H: each single operator `+ * - / %` → 1,2,3,4,5 | `cfg19_parse_single_operators` | [x] |
|20 | `parse_operation` | H: multi-operator strings (all 120 permutations of `"+*-/%"`) → fixed priority | `cfg20_parse_all_permutations_of_operators` | [x] |
|21 | `parse_operation` | H: 5k random strings over a mixed alphabet (operators + letters + digits + high-bit bytes), plus embedded-NUL cases | `cfg21_parse_random_strings` | [x] |
|22 | `get_operation_func` | G: `op` = 1..5 → returned pointer must equal that library's own `*_op` symbol | `cfg22_get_operation_func_returns_matching_symbol` | [x] |
|23 | `get_operation_func` → returned fn | G×I: call the returned pointer through FFI with random operands for every `op` in 1..5 | `cfg23_call_through_returned_function_pointer` | [x] |
|24 | `inreftree` | J: `param2 != 0`, `tree_sum % 4 == 0` → `'+'` → `OP_ADD` | `cfg24_inreftree_residue_0_add` | [x] |
|25 | `inreftree` | J: `tree_sum % 4 == 1` → `'*'` → `OP_MULTIPLY` | `cfg25_inreftree_residue_1_multiply` | [x] |
|26 | `inreftree` | J: `tree_sum % 4 == 2` → `'-'` → `OP_SUBTRACT` | `cfg26_inreftree_residue_2_subtract` | [x] |
|27 | `inreftree` | J: `tree_sum % 4 == 3` → `'%'` → `OP_MODULO` | `cfg27_inreftree_residue_3_modulo` | [x] |
|28 | `inreftree` | J: `tree_sum % 4 ∈ {-1,-2,-3}` → OOB `op_string[-1..-3]` read | `cfg28_inreftree_negative_residues_oob_rodata_read` | [x] |
|29 | `inreftree` | J: `param2 == 0` (target reset) crossed with each `tree_sum % 4` residue | `cfg29_inreftree_target_reset_crossed_with_residues` | [x] |
|30 | `inreftree` | J: `tree_sum == 0`, and `tree_sum` overflowing (`INT_MAX`-ish params) | `cfg30_inreftree_zero_and_overflowing_sums` | [x] |
|31 | `inreftree` | 100k random `(p1,p2,p3,p4)` incl. `INT_MIN`/`INT_MAX` corners; also asserts the resulting `node_table`/`node_count` global state matches | `cfg31_inreftree_randomized_bulk` | [x] |
|32 | `inreftree` | B: called with pre-corrupted `node_count`/`node_table`, then twice in a row (idempotence of the `node_count = 0` reset) | `cfg32_inreftree_ignores_stale_state_and_is_repeatable` | [x] |
|33 | `node_table` / `node_count` (data symbols) | ABI: `sizeof(TreeNode) == 52`, array 2600 bytes, field offsets probed by writing through one library and reading field-by-field | `cfg33_node_table_abi_layout_matches` | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — there
is **no** `add_executable`, and the Rust `Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. So there is no driver binary and
the "compare stdout byte-for-byte" gate is **not applicable**; the differential
harness compares return values and the full global-state bytes instead.
