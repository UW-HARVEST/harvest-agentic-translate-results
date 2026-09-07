# CONFIGS.md — Phase A: configuration-surface table

## Axes the C code actually branches on

Derived from `c_src/include/lib.h` (one entry point) and every `if` / `switch` /
loop-bound in `c_src/src/lib.c`.

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `operation_mode` (arg 1) | `0001`, `0002`, `0003`, `0004`, anything else (`default:`) | `switch`, lib.c:121 |
| `node_id` (arg 2) | found in `node_storage` vs. not found; which node (leaf / mid / root / chain) | `find_node_by_id`, lib.c:45-53 |
| `depth` (arg 3) | mode 1: `<=0` (loop never runs), `1..chain-length`, `>chain-length`; mode 2: `0`, `1..15`, `==16`, `>16`, `<0` (UB); mode 3: any int, incl. `0`, negatives, `INT_MIN`/`INT_MAX` (digit count changes the string length ⇒ changes the result); mode 4: scales `1.0 + depth*0.1`, incl. values that drive the clamp | lib.c:130, 159, 165, 183 |
| `flags` (arg 4) | mode 2: multiplied by `16`; mode 3: masked `& 0177` (only low 7 bits matter, sign-independent); modes 1/4: ignored | lib.c:161, 169 |
| library state: `node_count` | `0` (the shipped `.so` — `initialize_test_data` is `static` and never called ⇒ *permanent*), `1`, `2`, `>2` (unlocks the backward-walk block in mode 4), `MAX_NODES` | lib.c:38, 47, 56, 187 |
| parent chain shape | `parent_id == -1` (root, stops the mode-1 walk), parent present, parent id dangling (`find_node_by_id` returns NULL ⇒ `break`) | lib.c:130-134 |
| `sprintf` string length | `strlen("Node_%d_Depth_%d")` varies 15..34 with the digit counts and signs of `node_id`/`depth` | lib.c:165, 89-98 |

There are no compile-time `#ifdef`s and no runtime option setters: the library
is stateless from the caller's point of view, so `state` is only varied via the
`initialize_test_data` hook (see below).

## Two test surfaces

* **S1 — shipped `.so` (default features).** `node_count == 0` forever, so modes
  1/2/4 short-circuit into their error returns and mode 3 is the only computing
  path. Tested C-`.so`-vs-Rust-`.so` through `libloading`.
* **S2 — populated state (`expose_init_test_data`).** A C harness
  (`tests/c_harness/harness.c`) `#include`s `c_src/src/lib.c` verbatim and
  exports `jumpnode_initialize_test_data`, matching the Rust feature hook. This
  is the only way to reach `find_node_by_id` hits, `add_node`,
  `process_backward`, `safe_double_to_int`'s clamps and mode 4's backward walk.
  `c_src/` itself is not modified. Test data:
  `1(root,100.5) 2(p1,50.25) 3(p1,75.75) 4(p2,25.125) 5(p2,30.875) 6(p3,40.0625) 7(p4,12.5)`,
  every node's `data[] = {0100,0200,0300,0400}`.

## Rows — one per combination the C treats differently

Every row is exercised with many randomized inputs (fixed seed `0x5EED_1234`,
xorshift64\*) over the free arguments, not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | surface | test fn | [ ] |
|---|----------------|-------------------------------------------|---------|---------|-----|
| 1 | `jumpnode` | mode 3, `node_id`/`depth` small non-negative (short strings), `flags` random | S1 | `cfg_row1_mode3_small` | [x] |
| 2 | `jumpnode` | mode 3, `node_id`/`depth` negative (minus signs lengthen the string) | S1 | `cfg_row2_mode3_negative` | [x] |
| 3 | `jumpnode` | mode 3, `node_id`/`depth` at digit-count boundaries `0, ±1, ±9, ±10, ±99, ±100, ±999999999, ±1000000000, INT_MIN, INT_MAX` (cross product) | S1 | `cfg_row3_mode3_digit_boundaries` | [x] |
| 4 | `jumpnode` | mode 3, `flags` sweeping the `& 0177` mask: all 128 residues, plus negative `flags`, `INT_MIN`, `INT_MAX` | S1 | `cfg_row4_mode3_flag_mask` | [x] |
| 5 | `jumpnode` | mode 3, fully random `int` in all four args | S1 | `cfg_row5_mode3_fuzz` | [x] |
| 6 | `jumpnode` | mode 1 on empty storage (`node_count==0`) — random `node_id`/`depth`/`flags` | S1 | `cfg_row6_mode1_empty` | [x] |
| 7 | `jumpnode` | mode 2 on empty storage — random args (incl. negative/huge `depth`, which is *safe* here because it returns before `process_backward`) | S1 | `cfg_row7_mode2_empty` | [x] |
| 8 | `jumpnode` | mode 4 on empty storage — random args | S1 | `cfg_row8_mode4_empty` | [x] |
| 9 | `jumpnode` | unknown mode (`default:`), random args | S1 | `cfg_row9_default_mode` | [x] |
| 10 | `jumpnode` | S1 whole-surface fuzz: mode drawn from `{-3..8, INT_MIN, INT_MAX, random}` × random args | S1 | `cfg_row10_s1_fuzz_all_modes` | [x] |
| 11 | `jumpnode_initialize_test_data` + `jumpnode` | init, then mode 1 from the **root** node (`id=1`, `parent_id==-1`) with `depth` in `-5..40` | S2 | `cfg_row11_mode1_root` | [x] |
| 12 | `jumpnode_initialize_test_data` + `jumpnode` | init, then mode 1 from a **1-hop** node (`id=2,3`), `depth` `-5..40` — exercises `depth` shorter than / equal to / longer than the chain | S2 | `cfg_row12_mode1_one_hop` | [x] |
| 13 | `jumpnode_initialize_test_data` + `jumpnode` | init, then mode 1 from the **deepest chain** (`id=7 → 4 → 2 → 1`), `depth` `-5..40` | S2 | `cfg_row13_mode1_deep_chain` | [x] |
| 14 | `jumpnode_initialize_test_data` + `jumpnode` | init, then mode 1 with **every** existing id × every `depth` in `-2..12` × random `flags` (flags must be ignored) | S2 | `cfg_row14_mode1_all_ids` | [x] |
| 15 | `jumpnode_initialize_test_data` + `jumpnode` | init, then mode 1 with a **non-existent** id (still the error path, but with non-empty storage) | S2 | `cfg_row15_mode1_missing_id` | [x] |
| 16 | `jumpnode_initialize_test_data` + `jumpnode` | init, then mode 2, existing id, `depth` `0..=16` (in-bounds `process_backward`: full sum, partial sums, empty sum at `depth==16`), `flags` small random | S2 | `cfg_row16_mode2_inbounds` | [x] |
| 17 | `jumpnode_initialize_test_data` + `jumpnode` | init, then mode 2, existing id, `depth > 16` (loop never runs ⇒ result is purely `16*flags`) | S2 | `cfg_row17_mode2_depth_past_end` | [x] |
| 18 | `jumpnode_initialize_test_data` + `jumpnode` | init, then mode 2, `flags` large enough that `16*flags` wraps `int` (cross-check that both sides wrap identically) | S2 | `cfg_row18_mode2_flag_wrap` | [x] |
| 19 | `jumpnode_initialize_test_data` + `jumpnode` | init, then mode 4, existing id, `depth` `-20..40` — exercises `1.0+depth*0.1` incl. the sign flip at `depth==-10` and the `node_count>2` backward walk over the last 3 nodes | S2 | `cfg_row19_mode4_depth_sweep` | [x] |
| 20 | `jumpnode_initialize_test_data` + `jumpnode` | init, then mode 4 with extreme `depth` (`±10^9`, `INT_MIN`, `INT_MAX`) driving `safe_double_to_int` into both clamps | S2 | `cfg_row20_mode4_clamps` | [x] |
| 21 | `jumpnode_initialize_test_data` + `jumpnode` | init, then mode 3 (state must make no difference) + `default:` mode, random args | S2 | `cfg_row21_mode3_and_default_with_state` | [x] |
| 22 | `jumpnode_initialize_test_data` + `jumpnode` | **repeated** init (`node_count` reset each time) interleaved with calls — confirms the reset semantics and that storage does not grow | S2 | `cfg_row22_repeated_init` | [x] |
| 23 | `jumpnode_initialize_test_data` + `jumpnode` | init, then S2 whole-surface fuzz: random mode `{-3..8}` × id `{-3..12}` × `depth` `{0..=16}` (mode-2-safe) × random `flags` | S2 | `cfg_row23_s2_fuzz` | [x] |
| 24 | `jumpnode_initialize_test_data` + `jumpnode` | init, then mode 4 with `node_count` exactly `1` and `2` (backward-walk block *disabled*) vs `>2` (enabled) — driven by a harness hook that inits then truncates | S2 | `cfg_row24_mode4_small_count` | [x] |
| 25 | `jumpnode_initialize_test_data` + `jumpnode` | `add_node` filled to `MAX_NODES` (100) and one past — capacity behaviour, then mode 4's walk over a full table | S2 | `cfg_row25_max_nodes` | [x] |

## Feature combinations

`translation/Cargo.toml` declares exactly one feature, `expose_init_test_data`,
with no default features. Full cross-product = 2 configurations:

| combo | cargo flags | rows covered |
|-------|-------------|--------------|
| (none) | `--no-default-features` | 1-10 (S1). Rows 11-25 skip: the hook symbol is absent, which the test asserts. |
| `expose_init_test_data` | `--no-default-features --features expose_init_test_data` | 1-25 (S1 rows must still pass unchanged — the feature may not alter default behaviour) |

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(... SHARED src/lib.c)`; there is
no driver executable, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. The "compare binary stdout"
checklist item is therefore **not applicable**.
