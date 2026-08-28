# CONFIGS.md — Phase B configuration-surface table

The mirror of `ERRORS.md`, for **valid** inputs. Axes derived mechanically from
the branches the C actually takes.

## Axes the C branches on

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| A. `operation_mode` | `0001`(1), `0002`(2), `0003`(3), `0004`(4), everything else (`default:`) | `switch` at lib.c:121 |
| B. library node state | (i) `node_count == 0` / `node_storage` all-zero — the **only** state reachable in the shipped `.so`, because `initialize_test_data` is `static` and never called; (ii) the 7-node tree that `initialize_test_data` builds, reachable only through the test shim | lib.c:37-38, 209-219 |
| C. `node_id` | found vs not found; and *which* node (root `id=1` with `parent_id==-1`, depth-1 `id=2,3`, depth-2 `id=4,5,6`, depth-3 `id=7`) | `find_node_by_id`, lib.c:45-53 |
| D. `depth` | case 1: `0`, `<` chain length, `>=` chain length (loop-exit reason: counter vs `parent_id == -1` vs parent lookup failure). case 2: `0`, `1..15`, `==16`, `>16` (`process_backward` start offset vs `size`), `<0` (UB). case 3: any (only its decimal width matters). case 4: any (scales by `1.0 + depth*0.1`; `0`, positive, negative, extreme) | lib.c:130, 78-84, 165, 183 |
| E. `flags` | case 2: multiplied by 16 (full `int` range, sign matters). case 3: masked `& 0177` — only low 7 bits matter, sign of the rest irrelevant. cases 1/4/default: **ignored** | lib.c:161, 169 |
| F. decimal width of `node_id` / `depth` | case `0003` only: 1..11 characters each (incl. `-` sign), driving `strlen` and hence the metric | `sprintf` + `compute_size_metric`, lib.c:165-167 |
| G. `node_count > 2` | case `0004` only: enables the backward `node_storage` scan of the last ≤3 nodes | lib.c:187-198 |
| H. `safe_double_to_int` saturation | value `> 2147483647.0`, in range, `< -2147483648.0` | lib.c:100-109 |

There are **no** compile-time `#ifdef`s and no runtime option/flag setters in the
C: the whole public API is the single function `int jumpnode(int,int,int,int)`
(`c_src/include/lib.h`). Axis B is the only piece of hidden state.

## Cross-product, pruned to combinations the C treats differently

`state=Z` means the default, zero/empty library state (the shipped `.so`
behaviour); `state=T` means the 7-node tree, exercised through the shim +
`expose_init_test_data` feature. All rows use many randomized inputs with a
fixed seed unless a specific value is named.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `jumpnode` | mode=1, state=Z, random `node_id`/`depth`/`flags` over full `int` range → node never found | `cfg_row1_mode1_empty_state` | [x] |
| 2 | `jumpnode` | mode=2, state=Z, random `node_id`/`depth`/`flags` → node never found | `cfg_row2_mode2_empty_state` | [x] |
| 3 | `jumpnode` | mode=3, state=Z, `node_id`/`depth` = small single-digit values (width 1) | `cfg_row3_mode3_width1` | [x] |
| 4 | `jumpnode` | mode=3, state=Z, `node_id`/`depth` spanning **every** decimal width 1..10 positive and 1..11 negative (incl. `INT_MIN`, `INT_MAX`, `0`, `-1`, ±9, ±10, ±99, ±100, ±10^k, ±(10^k−1)) | `cfg_row4_mode3_all_widths` | [x] |
| 5 | `jumpnode` | mode=3, state=Z, `flags` covering all 128 values of `flags & 0177` plus random full-range `flags` (high bits must be ignored, incl. negative) | `cfg_row5_mode3_flag_mask` | [x] |
| 6 | `jumpnode` | mode=3, state=Z, randomized `node_id`×`depth`×`flags` full-range property sweep | `cfg_row6_mode3_random_sweep` | [x] |
| 7 | `jumpnode` | mode=4, state=Z, random `node_id`/`depth`/`flags` → node never found | `cfg_row7_mode4_empty_state` | [x] |
| 8 | `jumpnode` | mode ∉ {1,2,3,4}, state=Z, randomized full-range `int` modes + all of −8..12 | `cfg_row8_default_branch` | [x] |
| 9 | `jumpnode` | state=Z, randomized 4-tuples over *all* four arguments simultaneously (full `int` range, mode biased to hit every branch) — the end-to-end property sweep | `cfg_row9_full_random_property` | [x] |
| 10 | `jumpnode_initialize_test_data` + `jumpnode` | mode=1, state=T, `node_id`=1 (root, `parent_id==-1`) — loop exits immediately on the `parent_id == -1` guard, all `depth` 0..8 | `cfg_row10_mode1_root` | [x] |
| 11 | `jumpnode_initialize_test_data` + `jumpnode` | mode=1, state=T, `node_id`∈{2,3} (chain length 1), `depth` 0..8 — exits by `parent_id == -1` after 1 step | `cfg_row11_mode1_depth1_nodes` | [x] |
| 12 | `jumpnode_initialize_test_data` + `jumpnode` | mode=1, state=T, `node_id`∈{4,5,6} (chain length 2) and `7` (chain length 3), `depth` 0..8 — covers "loop ends because counter ran out" (`depth` < chain) and "ends at root" | `cfg_row12_mode1_deep_nodes` | [x] |
| 13 | `jumpnode_initialize_test_data` + `jumpnode` | mode=1, state=T, every `node_id` 1..7 × randomized large `depth` (incl. `INT_MAX`) — accumulation must still terminate at the root | `cfg_row13_mode1_huge_depth` | [x] |
| 14 | `jumpnode_initialize_test_data` + `jumpnode` | mode=1, state=T, `node_id` not present (0, 8, negative, `INT_MIN`) → error 18 even with data loaded | `cfg_row14_mode1_missing_id` | [x] |
| 15 | `jumpnode_initialize_test_data` + `jumpnode` | mode=2, state=T, every `node_id` 1..7 × `depth` 0..15 (`process_backward` sums `temp_array[depth..16]`) × `flags`=0 | `cfg_row15_mode2_depth_in_range` | [x] |
| 16 | `jumpnode_initialize_test_data` + `jumpnode` | mode=2, state=T, `depth` ∈ {16, 17, 100, `INT_MAX`} — empty backward loop, result is purely `16*flags` | `cfg_row16_mode2_depth_past_end` | [x] |
| 17 | `jumpnode_initialize_test_data` + `jumpnode` | mode=2, state=T, `depth` 0..16 × randomized `flags` in the non-overflowing range (both signs) — exercises `result += 16*flags` | `cfg_row17_mode2_flags` | [x] |
| 18 | `jumpnode_initialize_test_data` + `jumpnode` | mode=3, state=T — must be identical to state=Z (case 3 never touches node state) | `cfg_row18_mode3_state_independent` | [x] |
| 19 | `jumpnode_initialize_test_data` + `jumpnode` | mode=4, state=T, every `node_id` 1..7 × `depth` 0..8 — `sqrt` accumulation over `data[]` = {0100,0200,0300,0400} = {64,128,192,256}, plus the `node_count > 2` backward scan of the last 3 nodes | `cfg_row19_mode4_tree` | [x] |
| 20 | `jumpnode_initialize_test_data` + `jumpnode` | mode=4, state=T, `depth` negative / large positive / `INT_MIN` / `INT_MAX` — `1.0 + depth*0.1` scaling drives `safe_double_to_int` into **both** saturation clamps and through 0 | `cfg_row20_mode4_scaling_saturation` | [x] |
| 21 | `jumpnode_initialize_test_data` + `jumpnode` | mode ∉ {1,2,3,4}, state=T → still 130 (state-independent) | `cfg_row21_default_with_state` | [x] |
| 22 | `jumpnode_initialize_test_data` (repeated) + `jumpnode` | idempotence / statefulness: call init 1×, 2×, 3× and re-run a fixed script of `jumpnode` calls — `node_count` must be reset to 7 each time, so results must be stable | `cfg_row22_init_idempotent` | [x] |
| 23 | `jumpnode` then `jumpnode_initialize_test_data` then `jumpnode` | ordering: the *same* call must give the error code before init and the computed value after — verifies the hidden static state transition in both libraries | `cfg_row23_state_transition` | [x] |
| 24 | `jumpnode_initialize_test_data` + `jumpnode` | state=T, randomized 4-tuple property sweep over all four arguments (mode biased across 1..4 and out-of-range; `depth` restricted to ≥0 for mode 2, see ERRORS.md row 10) | `cfg_row24_tree_random_property` | [x] |
| 25 | `jumpnode` | interleaving of all 5 modes in one process, randomized order, state=Z — verifies no cross-call state leakage (`temp_array`/`buffer` are locals) | `cfg_row25_mode_interleaving` | [x] |

## Feature combinations

| # | cargo invocation | rows covered |
|---|------------------|--------------|
| 1 | `cargo test` (default, no features) | 1–9, 25 (rows 10–24 need the export hook and are skipped) |
| 2 | `cargo test --no-default-features --features expose_init_test_data` | 1–25 |

Driven by `check_all_features.sh`.
