# CONFIGS.md — Phase A: CONFIGURATION-SURFACE TABLE (valid inputs)

Mirror of `ERRORS.md` for inputs the C **accepts**. Axes derived mechanically from every `if` /
`switch` / loop branch and every distinct input shape `c_src/src/lib.c` special-cases. There are no
`#ifdef`s and no runtime option struct; the "options" of this library are (a) which of the 11
exported entry points is called, (b) the *mutable global state* `node_count` / `node_table`, which
the ABI exposes and which every function reads, and (c) the shapes of the scalar/string arguments.

## Axes

| axis | values the C branches on |
|------|--------------------------|
| A. entry point | `add_op`, `multiply_op`, `subtract_op`, `divide_op`, `modulo_op`, `find_node_by_id`, `add_tree_node`, `calculate_tree_sum`, `parse_operation`, `get_operation_func`, `inreftree` (all 11 — including the low-level ones, not just the `inreftree` one-shot wrapper from `lib.h`) |
| B. global state | `node_count ∈ {0, 1, few, MAX_NODES-1, MAX_NODES}`; `node_table` empty / partially built / full; stale rows left past `node_count` |
| C. tree shape | leaf; single-left child; single-right child; both children; linear chain of depth *n*; full binary tree; duplicate ids; diamond (shared child); dangling child id; child ids in the table but *before* the parent |
| D. parent slot state | parent has 0 children (→ `left_child_id`), 1 child (→ `right_child_id`), 2 children (→ dropped); `parent_id == -1` |
| E. label shape | `""`, 1 byte, mid-length, 30, 31, 32, 33, 64 bytes; with/without `'l'`; with embedded operator chars; non-ASCII/high bytes (0x80..0xFF) |
| F. scalar values | `0`, `±1`, small, `INT_MIN`, `INT_MAX`, `INT_MIN+1`, `INT_MAX-1`, random 32-bit; values chosen to hit each `tree_sum % 4 ∈ {0,1,2,3,-1,-2,-3}`; values causing signed overflow |
| G. `parse_operation` input | each of `+ * - / %` alone; each embedded mid-string; several at once (check-order precedence); none; `NULL`; `'\0'`-only; high-byte strings |
| H. `Operation` value | `1..5` (valid variants) and out-of-range ints (covered as errors in `ERRORS.md` rows 22–23) |
| I. observable output | return value **and** the full 2600-byte `node_table` image **and** `node_count`, compared byte-for-byte after every mutation |

Every row is exercised with **many randomized inputs** (`SplitMix64`, fixed seed `0x9E3779B97F4A7C15`)
plus the hand-picked boundary values, through `dlsym` on both `.so`s. Test file:
`tests/phase_b_configs.rs`.

| #  | entry point(s) | configuration (options set + input shape) | test | ✔ |
|----|----------------|--------------------------------------------|------|---|
| 1  | `add_op` | 2000 random `(a,b)` + all boundary pairs from `{0,±1,INT_MIN,INT_MAX,INT_MIN+1,INT_MAX-1}²`, incl. overflow; `unused1/unused2` varied to prove they are ignored | `cfg01_add_op` | [x] |
| 2  | `multiply_op` | same input matrix; incl. `INT_MIN*-1`, `INT_MAX*INT_MAX` overflow | `cfg02_multiply_op` | [x] |
| 3  | `subtract_op` | same input matrix; incl. `INT_MIN-1`, `0-INT_MIN` overflow | `cfg03_subtract_op` | [x] |
| 4  | `divide_op` | same matrix minus the `INT_MIN/-1` trap (ERRORS U1); covers `b<0`, `a<0` (C truncates toward zero), `\|a\|<\|b\|`, exact division, `b==±1` | `cfg04_divide_op` | [x] |
| 5  | `modulo_op` | same matrix minus `INT_MIN%-1`; covers negative dividend/divisor sign rules (C remainder takes the dividend's sign) | `cfg05_modulo_op` | [x] |
| 6  | `find_node_by_id` | `node_count == 0` (empty) | `cfg06_find_empty` | [x] |
| 7  | `find_node_by_id` | 1 node, id matches / does not match | `cfg07_find_single` | [x] |
| 8  | `find_node_by_id` | many nodes; target at first index, a middle index, and the last index (`node_count-1`) — exercises loop entry, middle, and exit | `cfg08_find_first_mid_last` | [x] |
| 9  | `find_node_by_id` | table full (`node_count == 50`), random ids incl. negative ids and `id == -1` (the same value used as the "no child"/"no parent" sentinel) | `cfg09_find_full_table_random` | [x] |
| 10 | `find_node_by_id` | returned pointer identity: offset of the result from the exported `node_table` base must be the same index in both libraries | `cfg10_find_returns_same_index` | [x] |
| 11 | `add_tree_node` | first node, `parent_id == -1` (root); check return value, `node_count`, and full table image | `cfg11_add_root` | [x] |
| 12 | `add_tree_node` | parent with 0 children ⇒ writes `parent->left_child_id` | `cfg12_add_fills_left_slot` | [x] |
| 13 | `add_tree_node` | parent with 1 child ⇒ writes `parent->right_child_id` | `cfg13_add_fills_right_slot` | [x] |
| 14 | `add_tree_node` | parent with 2 children ⇒ link dropped, parent untouched (also ERRORS row 11) | `cfg14_add_third_child_dropped` | [x] |
| 15 | `add_tree_node` | label shapes from axis E: `""`, 1, 30, 31, 32, 33, 64 bytes, high bytes 0x80–0xFF, embedded operator chars — full 32-byte `label` image compared | `cfg15_add_label_shapes` | [x] |
| 16 | `add_tree_node` | fill the table to exactly `MAX_NODES` (50 successful inserts), comparing return value + `node_count` + full 2600-byte image after **every** insert | `cfg16_add_fill_to_capacity` | [x] |
| 17 | `add_tree_node` | randomized long sequences (200 ops/round × 30 rounds) of ids/values/parent ids drawn from a pool that mixes valid, absent, duplicate, `-1` and extreme values; both libraries driven in lockstep, full state diffed after each op | `cfg17_add_random_sequences` | [x] |
| 18 | `calculate_tree_sum` | single leaf (both child sentinels) | `cfg18_sum_leaf` | [x] |
| 19 | `calculate_tree_sum` | root + left only, root + right only, root + both | `cfg19_sum_one_and_two_children` | [x] |
| 20 | `calculate_tree_sum` | linear left chain of depth 1..49 (recursion depth stress) | `cfg20_sum_deep_left_chain` | [x] |
| 21 | `calculate_tree_sum` | linear right chain of depth 1..49 | `cfg21_sum_deep_right_chain` | [x] |
| 22 | `calculate_tree_sum` | complete binary tree filling all 50 slots; sum queried from **every** node id, not just the root | `cfg22_sum_full_tree_every_node` | [x] |
| 23 | `calculate_tree_sum` | random tree topologies with random values incl. `INT_MIN`/`INT_MAX` (accumulator overflow), summed from every id | `cfg23_sum_random_trees` | [x] |
| 24 | `parse_operation` | each single operator `"+"`, `"*"`, `"-"`, `"/"`, `"%"` | `cfg24_parse_single_operators` | [x] |
| 25 | `parse_operation` | operator embedded mid-string / at the end, e.g. `"ab*cd"`, `"xx%"` — `strchr` scans the whole string | `cfg25_parse_embedded_operator` | [x] |
| 26 | `parse_operation` | 3000 random byte strings (len 0..16, bytes 0x01..0xFF, so all 5 operators occur naturally in combination) — pins the exact `+ > * > - > / > %` check order | `cfg26_parse_random_strings` | [x] |
| 27 | `get_operation_func` | each valid variant `1..=5`: returned pointer must equal that library's own `dlsym` address for the matching `*_op`, **and** calling it through the pointer must give matching results on a random argument matrix | `cfg27_get_func_valid_variants` | [x] |
| 28 | `inreftree` | `tree_sum % 4 == 0` ⇒ `'+'` ⇒ `OP_ADD` ⇒ `add_op(tree_sum, target_id)` | `cfg28_inreftree_rem0_add` | [x] |
| 29 | `inreftree` | `tree_sum % 4 == 1` ⇒ `'*'` ⇒ `OP_MULTIPLY` | `cfg29_inreftree_rem1_multiply` | [x] |
| 30 | `inreftree` | `tree_sum % 4 == 2` ⇒ `'-'` ⇒ `OP_SUBTRACT` | `cfg30_inreftree_rem2_subtract` | [x] |
| 31 | `inreftree` | `tree_sum % 4 == 3` ⇒ `'%'` ⇒ `OP_MODULO` (note: `'/'`/`OP_DIVIDE` is **unreachable** from `inreftree` — `op_string` is `"+*-%"`) | `cfg31_inreftree_rem3_modulo` | [x] |
| 32 | `inreftree` | negative `tree_sum` with remainder `-1`, `-2`, `-3` (out-of-bounds `op_string` read, ERRORS row 26) | `cfg32_inreftree_negative_remainders` | [x] |
| 33 | `inreftree` | `param2 == 0` (target fallback `2 → 1`) crossed with each remainder class | `cfg33_inreftree_param2_zero_x_remainder` | [x] |
| 34 | `inreftree` | all-zero params; single non-zero param (4 positions); all `±1` sign combinations (2⁴) | `cfg34_inreftree_small_exhaustive` | [x] |
| 35 | `inreftree` | extreme params: full cross-product of `{0,±1,±2,INT_MIN,INT_MIN+1,INT_MAX,INT_MAX-1}⁴` (7⁴ = 2401 cases) — `tree_sum` overflow × remainder class × target fallback | `cfg35_inreftree_extremes_cross_product` | [x] |
| 36 | `inreftree` | 20000 fully random `(p1,p2,p3,p4)` | `cfg36_inreftree_random` | [x] |
| 37 | `inreftree` | called with **dirty pre-existing state** (table pre-filled to 50 by prior `add_tree_node` calls, `node_count` non-zero) — proves the `node_count = 0` reset and that stale rows past index 4 never leak in; post-call table image diffed in full | `cfg37_inreftree_after_dirty_state` | [x] |
| 38 | `inreftree` | called repeatedly (idempotence across 3 back-to-back calls with the same and with differing args) | `cfg38_inreftree_repeated_calls` | [x] |
| 39 | globals | `node_table` / `node_count` as ABI **data**: `nm -D` sizes equal (2600 / 4), initial zeroed image equal, direct writes through the `dlsym` pointer observed identically by `find_node_by_id` | `cfg39_global_data_abi` | [x] |
| 40 | composed pipeline | the exact `inreftree` sequence rebuilt by hand from the low-level entry points (`add_tree_node` ×4 → label scan → `find_node_by_id` → `calculate_tree_sum` → `parse_operation` → `get_operation_func` → call), asserting every intermediate value matches, then that the composed result equals `inreftree`'s in **both** libraries | `cfg40_manual_pipeline_matches_inreftree` | [x] |

## Appendix — suite adequacy (mutation testing)

A differential suite that passes proves nothing unless it is capable of failing.
`./mutation_test.sh` injects 34 known bugs into `src/lib.rs` one at a time, runs the whole suite
against each, and reports whether the suite caught it. Result:

```
=== mutation score: 34 caught, 0 escaped, 0 invalid/non-compiling ===
PASS: every valid mutant was caught
```

Mutants cover: arithmetic off-by-one and operand-order errors, euclidean vs truncating
division/remainder, the `b == 0` guards, `strncpy` zero-padding, `strchr` last-position
off-by-one, `find_node_by_id` first-vs-last match and loop bound, the `>= MAX_NODES` capacity
check, `MAX_NODES` itself, `node_table` length, `TreeNode` field order, the `label[31]`
terminator store, rolling back the partial write on parent failure, left-vs-right child slot
order, `node_count` increment timing and returned index, `calculate_tree_sum`'s accumulator seed
and right-subtree recursion, `parse_operation`'s check order and NULL handling, the `Operation`
constants, `get_operation_func`'s `default:` branch (including returning NULL), and every part of
`inreftree` (label scan needle, root of the sum, `value == 0` fallback, operand order,
`node_count` reset, literal contents, and all three aspects of the out-of-bounds `op_string`
read).

Two harness bugs were found and fixed this way:

1. **Stale-artifact bug (critical).** `cargo test` does **not** rebuild a `cdylib`-only lib target —
   integration tests cannot link it, so cargo sees no dependency edge. Every mutant initially
   "escaped" because the tests were loading a `.so` from an earlier build. `tests/common/mod.rs`
   now rebuilds the cdylib itself before `dlopen` and hard-fails on a `STALE ARTIFACT` if the
   `.so` is older than `src/lib.rs` or `Cargo.toml`. It also infers the profile from its own
   executable path so `cargo test` and `cargo test --release` each load the matching `.so`.
2. **Unobservable-write blind spot.** `node->label[31] = '\0'` writes a byte that
   `strncpy(dst, src, 31)` never touches, so against a zeroed table a missing store was invisible.
   `cfg15` / `err12b` now pre-poison the destination row (`0xFF`, `0x5a5a5a5a`, …) so every byte
   the C writes is observable.
