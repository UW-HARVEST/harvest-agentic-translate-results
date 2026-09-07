# CONFIGS.md — Phase B configuration-surface table

## Axes the C code actually branches on

Derived from `c_src/src/lib.c` (all `if` / `while` / `%` / comparison branches)
and `c_src/include/lib.h`.

* **Compile-time options:** none. `c_src/CMakeLists.txt` sets no `-D` flags and
  the source contains no `#ifdef`/`#if` other than header guards (`grep -c
  '#if' c_src/src/lib.c` → 0). Rust crate `Cargo.toml` declares **no
  `[features]`**, so the only feature combination is the default (empty) one.
* **Runtime options:** there are no flags/modes. The only mutable configuration
  is the **library-global state**: `node_storage[100]` + `node_count`, mutated by
  `add_node` and *reset to 0* by `maxnmin`. So "configuration" = the shape of
  the node table plus the argument shapes.

### State axes (the node table)
* `S0` empty (`node_count == 0`, pristine library)
* `S1` one node
* `Sm` many nodes (a real tree: root / children / grandchildren)
* `Sfull` exactly `MAX_NODES` = 100 nodes
* `Sdup` duplicate `id`s (first-match semantics of `find_node_by_id`)
* `Sorphan` nodes whose `parent_id` names no existing node
* `Sflat` all nodes share one `parent_id`
* `Sdeep` a deep parent chain (recursion depth in `calculate_subtree_sum`)
* `Sneg` negative / `0` / `INT_MIN` / `INT_MAX` ids and parent_ids
* `Smaxnmin` the fixed 6-node tree that `maxnmin` builds internally

### Name-shape axes (`add_node` / `process_string`)
* empty `""`, 1 byte, 48 bytes, exactly 49 bytes, 50 bytes, > 49 bytes (truncation)
* embedded NUL, high-bit bytes (`0x80`..`0xFF`, signed `char`), full byte range

### Double-shape axes (`value`, `safe_double_to_int`)
* `0.0`, `-0.0`, subnormal, small fraction, exact integer, `INT_MAX`/`INT_MIN`
  boundary ±1 ulp, huge (`1e300`), `±INFINITY`, NaN, negatives

### Int-shape axes (`maxnmin` params, ids)
* `param1/param2 % 6` residues `0..5` **and** negative residues `-5..-1`
* `param4 % 3` residues `0..2` and negative `-2..-1`
* `param3` = `0`, `-1` (zero divisor), positive, negative, `INT_MAX`, `INT_MIN`
* `INT_MIN`/`INT_MAX` in every slot

## Rows

Every row is exercised through the `.so` exports of **both** libraries with many
randomized inputs (`StdRng`-free xorshift PRNG, fixed seed `0x5EED_1234_ABCD_0001`).

| #  | entry point(s) | configuration (options set + input shape) | test | [ ] |
|----|----------------|--------------------------------------------|------|-----|
| 1  | `safe_double_to_int` | pristine lib; randomized `f64` bit patterns (whole `u64` space, incl. NaN/inf/subnormal) — 20000 samples | `cfg_01_sdti_random_bits` | [x] |
| 2  | `safe_double_to_int` | randomized doubles drawn near the `INT_MIN`/`INT_MAX` clamp edges (± few ulps and ± small deltas) | `cfg_02_sdti_random_near_edges` | [x] |
| 3  | `safe_double_to_int` | randomized in-range doubles (uniform in `[INT_MIN, INT_MAX]`), truncation-toward-zero for both signs | `cfg_03_sdti_random_in_range` | [x] |
| 4  | `process_string` | `S0`; randomized byte strings, len 0..64, bytes `0x01..0xFF` (incl. high-bit / signed-char) | `cfg_04_process_string_random` | [x] |
| 5  | `process_string` | randomized long strings (len 1..4096) of high-bit bytes → large-magnitude / overflowing sums | `cfg_05_process_string_long` | [x] |
| 6  | `process_string` | string with an embedded NUL — must stop at the NUL | `cfg_06_process_string_embedded_nul` | [x] |
| 7  | `add_node` + `find_node_by_id` | `S0` → `S1`: single randomized `(id, parent_id, name, value)`; check return index, found pointer's relative index and full struct bytes | `cfg_07_add_then_find_single` | [x] |
| 8  | `add_node` + `find_node_by_id` | `Sm`: 6-node tree, lookup of every id + absent ids; compare relative index + struct bytes | `cfg_08_add_tree_find_all` | [x] |
| 9  | `add_node` + `find_node_by_id` | `Sdup`: duplicate ids → first-match; and `Sneg` ids (`0`, `-1`, `INT_MIN`, `INT_MAX`) | `cfg_09_duplicate_and_extreme_ids` | [x] |
| 10 | `add_node` | name shapes: `""`, 1, 48, 49, 50, 200 bytes; high-bit bytes; verify the stored 50-byte `name` array byte-for-byte | `cfg_10_add_node_name_shapes` | [x] |
| 11 | `add_node` | `value` shapes: `0.0`, `-0.0`, subnormal, `1e300`, `±INFINITY`, NaN, randomized bit patterns; verify stored `value` bits | `cfg_11_add_node_value_shapes` | [x] |
| 12 | `add_node` | `Sfull`: fill to exactly 100 nodes, checking each return index `0..99` | `cfg_12_add_node_fill_to_max` | [x] |
| 13 | `get_children_count` | `S0` (empty) — every probe returns `0` | `cfg_13_children_count_empty` | [x] |
| 14 | `get_children_count` | `Sflat`: N randomized nodes all sharing one `parent_id`; probe that id and neighbours | `cfg_14_children_count_flat` | [x] |
| 15 | `get_children_count` | `Sm` random forest (randomized parent_ids from a small pool) — probe the whole pool + out-of-pool ids | `cfg_15_children_count_random_forest` | [x] |
| 16 | `get_children_count` | `Sfull` (100 nodes) with randomized parent ids incl. `INT_MIN`/`INT_MAX`/`0`/`-1` | `cfg_16_children_count_full` | [x] |
| 17 | `calculate_subtree_sum` | `S1`: leaf only → returns its own `value` (incl. NaN/inf values) | `cfg_17_subtree_sum_leaf` | [x] |
| 18 | `calculate_subtree_sum` | `Sm`: the 6-node tree, from every node id — exact `f64` bit equality (addition order matters) | `cfg_18_subtree_sum_tree` | [x] |
| 19 | `calculate_subtree_sum` | randomized valid forests (acyclic, ids `1..N`, `parent_id < id`), N up to 60, randomized values — compare raw `f64` bits from every id | `cfg_19_subtree_sum_random_forest` | [x] |
| 20 | `calculate_subtree_sum` | `Sdeep`: chain of 90 nodes (`parent_id == id-1`) — deep recursion, summation order | `cfg_20_subtree_sum_deep_chain` | [x] |
| 21 | `calculate_subtree_sum` | `Sdup`: duplicate ids so the same subtree is summed more than once (C first-match + per-row recursion) | `cfg_21_subtree_sum_duplicate_ids` | [x] |
| 22 | `calculate_subtree_sum` | `Sorphan`: nodes whose `parent_id` names nothing; sum from an orphan and from a missing id | `cfg_22_subtree_sum_orphans` | [x] |
| 23 | `calculate_subtree_sum` | value shapes producing `inf`/NaN accumulation (`1e308 + 1e308`, `inf + -inf`) — bitwise NaN comparison | `cfg_23_subtree_sum_inf_nan` | [x] |
| 24 | `maxnmin` | full cross-product of `param1 % 6 ∈ {0..5}` × `param2 % 6 ∈ {0..5}` with `param3 = 1`, `param4 = 0` — all 36 node-selection combinations | `cfg_24_maxnmin_residue_cross` | [x] |
| 25 | `maxnmin` | `param4 % 3 ∈ {0,1,2}` × negative residues, all `parent_id` selections `1,2,3` and `0,-1,-2` | `cfg_25_maxnmin_param4_residues` | [x] |
| 26 | `maxnmin` | `param3 ∈ {-2,-1,0,1,2,6,-6,1000,INT_MAX,INT_MIN}` (incl. zero divisor and overflow) × representative `param1/2/4` | `cfg_26_maxnmin_param3_shapes` | [x] |
| 27 | `maxnmin` | randomized 4-tuples over the full `i32` range — 20000 samples | `cfg_27_maxnmin_random_full_range` | [x] |
| 28 | `maxnmin` | randomized 4-tuples restricted to small magnitudes `-20..20` (dense residue coverage, both signs) — 20000 samples | `cfg_28_maxnmin_random_small` | [x] |
| 29 | `maxnmin` | boundary cross-product: every param ∈ `{INT_MIN, INT_MIN+1, -6, -1, 0, 1, 6, INT_MAX-1, INT_MAX}` (9^4 = 6561 combos) | `cfg_29_maxnmin_boundary_cross` | [x] |
| 30 | `maxnmin` after `add_node` | state interaction: pre-populate storage (1, 6, 99, 100 nodes with junk) then call `maxnmin` — it must reset `node_count` to 0 identically | `cfg_30_maxnmin_resets_state` | [x] |
| 31 | `add_node`/`find_node_by_id`/`get_children_count`/`calculate_subtree_sum` after `maxnmin` | state interaction: `maxnmin` first, then inspect the 6 nodes it left behind via the low-level API | `cfg_31_state_after_maxnmin` | [x] |
| 32 | all 7 exports, interleaved | randomized operation sequences (op = random pick of the 7 functions with randomized args), 400 sequences × 60 ops, comparing every return value and every observed struct — the composed pipeline | `cfg_32_random_op_sequences` | [x] |
